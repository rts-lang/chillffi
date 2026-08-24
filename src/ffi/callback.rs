use std::sync::{Mutex, MutexGuard, OnceLock};
use fxhash::FxHashMap;
use crate::ffi::value::Value;
// =================================================================================================

/// todo desc
type CallbackFn = Box<dyn Fn(Vec<Value>) -> Value + Send + 'static>;

/// todo desc
static Registry: OnceLock<Mutex<FxHashMap<u64, CallbackFn>>> = OnceLock::new();

/// todo desc
fn registry() -> &'static Mutex<FxHashMap<u64, CallbackFn>> 
{
  Registry.get_or_init(|| Mutex::new(FxHashMap::default()))
}

/// Регистрирует Rust-клоужер под заданным id.
pub fn register(id: u64, f: CallbackFn) 
{
  registry().lock().unwrap().insert(id, f);
}

/// Вызывает ранее зарегистрированный клоужер.
pub fn invoke(id: u64, args: Vec<Value>) -> Value 
{
  let map: MutexGuard<FxHashMap<u64, CallbackFn>> = registry().lock().unwrap();
  let f: &CallbackFn = map.get(&id).expect("Callback not found in registry");
  f(args)
}

// =================================================================================================