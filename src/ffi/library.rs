use crate::ffi::value::{Type, Value};
use serde::{Serialize, Deserialize};
use std::collections::HashMap;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Mutex, MutexGuard, OnceLock};
use crate::zygote::{FFIRequest, FFIResponse, ZygoteStack};
// =================================================================================================

/// Ошибки, возникающие при работе с загрузкой библиотек и выполнением вызовов
#[derive(Debug)]
pub enum FFIError
{
  ZygoteNotInitialized,
  ZygoteCommunicationFailed(String),
  LibraryLoadFailed(String),
  LibraryNotFound,
  SymbolNotFound,
  BadArgument,
  BadResultType,
  CallFailed(String),
  UnsupportedPointerReturn,
  EncodeFailed,
  DecodeFailed,
  Other(String)
}

impl std::fmt::Display for FFIError
{
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result
  {
    write!(f, "{:?}", self)
  }
}

impl<E: std::error::Error + 'static> From<E> for FFIError
{
  fn from(err: E) -> Self {
    Self::Other(err.to_string())
  }
}

// =================================================================================================

/// Счётчик для выдачи уникальных идентификаторов библиотекам
static NextLibraryID: AtomicUsize = AtomicUsize::new(1);
/// Глобальный реестр загруженных библиотек по их идентификаторам
static RegisteredLibraries: OnceLock<Mutex<HashMap<usize, String>>> = OnceLock::new();

/// Возвращает следующий уникальный идентификатор библиотеки
fn nextLibraryId() -> usize
{
  NextLibraryID.fetch_add(1, Ordering::SeqCst)
}

/// Возвращает общий реестр зарегистрированных библиотек
fn getRegistry() -> &'static Mutex<HashMap<usize, String>>
{
  RegisteredLibraries.get_or_init(|| Mutex::new(HashMap::new()))
}

/// Добавляет библиотеку в реестр по её идентификатору
fn registerLibrary(id: usize, path: &str) -> ()
{
  let mut registry = getRegistry().lock().unwrap();
  registry.insert(id, path.to_string());
}

/// Удаляет библиотеку из реестра по её идентификатору
fn unregisterLibrary(id: usize) -> ()
{
  let mut registry = getRegistry().lock().unwrap();
  registry.remove(&id);
}

/// Запрашивает загрузку библиотеки в текущую зиготу
fn sendLoadLibrary(_id: usize, _path: &str) -> Result<(), FFIError>
{
  // todo: Отправить ZygoteCommand::LoadLibrary в зиготу для кеширования
  //   Пока что библиотека будет загружаться при каждом вызове через callById
  //   - только резервирует место для будущего кеширования библиотек.
  Ok(())
}

/// Запрашивает выгрузку библиотеки из текущей зиготы
fn sendUnloadLibrary(_id: usize) -> Result<(), FFIError>
{
  // todo: Отправить ZygoteCommand::UnloadLibrary в зиготу
  //   Пока операция является заглушкой до реализации кеширования.
  Ok(())
}

/// Выполняет вызов FFI функции по идентификатору зарегистрированной библиотеки
fn callById(
  libraryId: usize,
  functionName: &str,
  args: Vec<Value>,
  resultType: Type,
) -> Result<Value, FFIError> 
{
  let registry: MutexGuard<HashMap<usize, String>> = getRegistry().lock().unwrap();
  let libraryPath: String = registry.get(&libraryId)
    .ok_or(FFIError::LibraryNotFound)?
    .clone();
  drop(registry);

  let request: FFIRequest = FFIRequest {
    libraryPath,
    functionName: functionName.to_string(),
    args,
    resultType,
  };

  // Выбираем клон Зиготы, созданный в текущем блоке ffi!{}
  ZygoteStack.with(|stack| {
    let mut mut_stack = stack.borrow_mut();
    let zygote = mut_stack.last_mut().ok_or(FFIError::ZygoteNotInitialized)?;

    match zygote.call(request) {
      Ok(FFIResponse::Ok(value)) => Ok(value),
      Ok(FFIResponse::Err(e)) => Err(FFIError::CallFailed(e)),
      Err(e) => Err(FFIError::ZygoteCommunicationFailed(e)),
    }
  })
}

// =================================================================================================

/// Дескриптор загруженной библиотеки с ограничением доступных методов
#[doc(hidden)]
pub struct __Library<const Allowed: bool = false>
{
  // Идентификатор библиотеки внутри менеджера
  libraryId: usize,
  // Путь к загруженной библиотеке
  libraryPath: String
}

/// Методы, доступные всегда (id, и т.д.)
impl<const Allowed: bool> __Library<Allowed>
{
  /// Возвращает идентификатор библиотеки
  pub fn id(&self) -> usize
  {
    self.libraryId
  }
}

// Методы, доступные только внутри ffi!{}
impl __Library<true>
{
  /// Выполняет вызов функции из загруженной библиотеки
  pub fn call(
    &self,
    functionName: &str,
    args: Vec<Value>,
    resultType: Type,
  ) -> Result<Value, FFIError>
  {
    callById(self.libraryId, functionName, args, resultType)
  }

  /// Выгружает библиотеку и удаляет её из реестра
  pub fn unload(self) -> Result<(), FFIError>
  {
    sendUnloadLibrary(self.libraryId)?;
    unregisterLibrary(self.libraryId);
    Ok(())
  }

  /// Загружает библиотеку и регистрирует её для дальнейших вызовов
  pub fn load(libraryPath: &str) -> Result<Self, FFIError>
  {
    let libraryId: usize = nextLibraryId();
    let ownedPath: String = String::from(libraryPath);
    registerLibrary(libraryId, &ownedPath);
    match sendLoadLibrary(libraryId, &ownedPath)
    {
      Ok(()) => Ok(Self{ libraryId, libraryPath: ownedPath }),
      Err(error) => { unregisterLibrary(libraryId); Err(error) }
    }
  }
}

/// Публичный тип снаружи — без load/call
pub type Library = __Library<false>;

/// Скрытый тип для ffi! — с load/call
#[doc(hidden)]
pub type __FFILibrary = __Library<true>;

// =================================================================================================

/// Способ указания библиотеки для выполнения вызова
#[derive(Serialize, Deserialize)]
pub enum CallTarget
{
  Path(String),
  LibraryId(u64)
}

/// Описание запроса на выполнение FFI функции
#[derive(Serialize, Deserialize)]
pub struct CallRequest
{
  pub target: CallTarget,
  pub functionName: String,
  pub args: Vec<Value>,
  pub resultType: Type
}

/// Команды управления загрузкой библиотек и выполнения вызовов
#[derive(Serialize, Deserialize)]
pub enum ZygoteCommand
{
  LoadLibrary(LoadLibraryRequest),
  UnloadLibrary(UnloadLibraryRequest),
  Call(CallRequest)
}

/// Запрос на загрузку библиотеки и сохранение её идентификатора
#[derive(Serialize, Deserialize)]
pub struct LoadLibraryRequest
{
  pub libraryId: u64,
  pub libraryPath: String
}

/// Запрос на выгрузку ранее загруженной библиотеки
#[derive(Serialize, Deserialize)]
pub struct UnloadLibraryRequest
{
  pub libraryId: u64
}

// =================================================================================================