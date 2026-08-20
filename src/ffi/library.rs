use std::sync::MutexGuard;
use crate::zygote::ClonedZygote;
use crate::zygote::ZygoteState;
use crate::ffi::value::{Type, Value};
use std::collections::HashMap;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Mutex, OnceLock};
use crate::zygote::{FFIRequest, FFIResponse, ZygoteStack};
// =================================================================================================

/// Errors occurring during library loading and call execution
/// 
/// todo Тут нет описания каждой ошибки, что может быть полезно.
#[derive(Debug)]
pub enum FFIError
{
  ZygoteNotInitialized,
  NoActiveZygoteScope,
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

/// Counter for assigning unique identifiers to libraries
static NextLibraryID: AtomicUsize = AtomicUsize::new(1);
/// Global registry of loaded libraries by their identifiers
static RegisteredLibraries: OnceLock<Mutex<HashMap<usize, String>>> = OnceLock::new();

/// Returns the next unique library identifier
fn nextLibraryId() -> usize
{
  NextLibraryID.fetch_add(1, Ordering::SeqCst)
}

/// Returns the global registry of registered libraries
fn getRegistry() -> &'static Mutex<HashMap<usize, String>>
{
  RegisteredLibraries.get_or_init(|| Mutex::new(HashMap::new()))
}

/// Adds a library to the registry by its identifier
fn registerLibrary(id: usize, path: &str) -> ()
{
  let mut registry = getRegistry().lock().unwrap();
  registry.insert(id, path.to_string());
}

/// Removes a library from the registry by its identifier
fn unregisterLibrary(id: usize) -> ()
{
  let mut registry = getRegistry().lock().unwrap();
  registry.remove(&id);
}

/// Requests loading a library into the current zygote
fn sendLoadLibrary(_id: usize, _path: &str) -> Result<(), FFIError>
{
  // todo: Отправить ZygoteCommand::LoadLibrary в зиготу для кеширования
  //   Пока что библиотека будет загружаться при каждом вызове через callById
  //   - только резервирует место для будущего кеширования библиотек.
  Ok(())
}

/// Requests unloading a library from the current zygote
fn sendUnloadLibrary(_id: usize) -> Result<(), FFIError>
{
  // todo: Отправить ZygoteCommand::UnloadLibrary в зиготу
  //   Пока операция является заглушкой до реализации кеширования.
  Ok(())
}

/// Performs an FFI function call 
/// by the identifier of the registered library
fn callById(
  libraryId: usize,
  functionName: &str,
  args: Vec<Value>,
  resultType: Type,
) -> Result<Value, FFIError> 
{
  // Check whether the global zygote in ZygoteState has been initialized
  if ZygoteState.get().is_none() {
    return Err(FFIError::ZygoteNotInitialized);
  }

  // Retrieve the path to the `.so` from the registry and construct an FFIRequest
  let registry: MutexGuard<HashMap<usize, String>> = getRegistry().lock().unwrap();
  let libraryPath: String = registry
    .get(&libraryId)
    .ok_or(FFIError::LibraryNotFound)?
    .clone();
  drop(registry);

  let request: FFIRequest = FFIRequest {
    libraryPath,
    functionName: functionName.to_string(),
    args,
    resultType,
  };

  // Search for the active clone in the local stack of the current thread
  ZygoteStack.with(|stack| {
    let mut mutStack = stack.borrow_mut();

    // If the stack is empty — it means the call is being made outside the context of ffi!{}
    let zygote: &mut ClonedZygote = mutStack
      .last_mut()
      .ok_or(FFIError::NoActiveZygoteScope)?;

    // Execute the FFI request through the current zygote
    match zygote.call(request) {
      Ok(FFIResponse::Ok(val)) => Ok(val),
      Ok(FFIResponse::Err(err)) => Err(FFIError::CallFailed(err)),
      Err(err) => Err(FFIError::ZygoteCommunicationFailed(err)),
    }
  })
}

// =================================================================================================

/// Handle of the loaded library with a restriction on available methods
#[doc(hidden)]
pub struct __Library<const Allowed: bool = false>
{
  /// Library identifier
  libraryId: usize,
  /// Путь к загруженной библиотеке
  libraryPath: String
}

// Methods that are always available
impl<const Allowed: bool> __Library<Allowed>
{
  /// Returns the library identifier
  pub fn id(&self) -> usize
  {
    self.libraryId
  }
}

// Methods available only inside ffi!{}
impl __Library<true>
{
  /// Executes a function call from the loaded library
  pub fn call(
    &self,
    functionName: &str,
    args: Vec<Value>,
    resultType: Type,
  ) -> Result<Value, FFIError>
  {
    callById(self.libraryId, functionName, args, resultType)
  }

  /// Unloads the library and removes it from the registry
  pub fn unload(self) -> Result<(), FFIError>
  {
    sendUnloadLibrary(self.libraryId)?;
    unregisterLibrary(self.libraryId);
    Ok(())
  }

  /// Loads the library and registers it for further calls
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

/// Public type from the outside
pub type Library = __Library<false>;

/// Hidden type for ffi!
#[doc(hidden)]
pub type __FFILibrary = __Library<true>;

// =================================================================================================