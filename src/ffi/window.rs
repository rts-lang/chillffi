use std::sync::{Mutex, MutexGuard};
use serde::{Deserialize, Serialize};
use crate::ffi::value::{Type, Value};
use crate::zygote::{FFIRequest, FFIResponse, ZygoteHandle, ZygoteState, sendAndReceive, encode, decode, spawnZygote};
// =================================================================================================

/// Результат выполнения кода в изолированном воркере.
#[derive(Serialize, Deserialize)]
pub enum WindowResult
{
  Ok(Value),
  Err(String),
}

// =================================================================================================

/// Хранилище для функции, которую нужно выполнить в воркере.
static WindowFunc: Mutex<Option<Box<dyn FnOnce() -> Result<Value, String> + Send>>> = Mutex::new(None);

// =================================================================================================

/// Отправляет запрос зиготе на выполнение кода в отдельном воркере.
pub fn callWindow<F>(func: F) -> Result<Value, String>
where
  F: FnOnce() -> Result<Value, String> + Send + 'static,
{
  // Блокируем мьютекс и сохраняем замыкание в глобальном хранилище
  let mut guard = WindowFunc.lock().unwrap();
  *guard = Some(Box::new(func));
  drop(guard);

  // Формируем специальный запрос для зиготы с маркером "__window__"
  let request: FFIRequest = FFIRequest {
    libraryPath: String::new(),
    functionName: "__window__".to_string(),
    args: Vec::new(),
    resultType: Type::None,
  };
  let bytes: Vec<u8> = encode(&request);

  // Получаем мьютекс зиготы и отправляем запрос
  let mutex: &Mutex<ZygoteHandle> = ZygoteState.get().expect("Zygote not initialized");
  let mut zygoteGuard: MutexGuard<ZygoteHandle> = mutex.lock().unwrap();

  // Пытаемся отправить запрос и получить ответ
  if let Ok(responseBytes) = sendAndReceive(&mut zygoteGuard.socket, &bytes)
  {
    let resp: FFIResponse = decode(&responseBytes).map_err(|e| e.to_string())?;
    return match resp {
      FFIResponse::Ok(val) => Ok(val),
      FFIResponse::Err(e) => Err(e),
    };
  }

  // Если связь с зиготой потеряна — пересоздаем её и повторяем запрос
  *zygoteGuard = spawnZygote().map_err(|e| format!("Zygote respawn failed: {}", e))?;
  let responseBytes: Vec<u8> = sendAndReceive(&mut zygoteGuard.socket, &bytes).map_err(|e| e.to_string())?;
  drop(zygoteGuard);

  // Декодируем и возвращаем результат
  let resp: FFIResponse = decode(&responseBytes).map_err(|e| e.to_string())?;
  match resp {
    FFIResponse::Ok(val) => Ok(val),
    FFIResponse::Err(e) => Err(e),
  }
}

// =================================================================================================

/// Выполняется **ВНУТРИ** форкнутого воркера.
pub fn executeInWorker() -> WindowResult
{
  // Блокируем мьютекс и забираем замыкание из хранилища
  let mut guard = WindowFunc.lock().unwrap();
  let func: Box<dyn FnOnce() -> Result<Value, String> + Send> = guard.take().expect("Window function not set");
  drop(guard);

  // Выполняем замыкание и возвращаем результат
  func().map(WindowResult::Ok).unwrap_or_else(|e| WindowResult::Err(e))
}

// =================================================================================================

/// Входная точка для макроса `ffi!{}`.
pub fn ffiBlock<F>(func: F) -> Result<Value, String>
where
  F: FnOnce() -> Result<Value, String> + Send + 'static,
{
  callWindow(func)
}

// =================================================================================================