use std::sync::{Mutex, OnceLock};
use fxhash::FxHashMap;
use crate::ffi::value::Value;

type CallbackFn = Box<dyn Fn(Vec<Value>) -> Value + Send + 'static>;

static REGISTRY: OnceLock<Mutex<FxHashMap<u64, CallbackFn>>> = OnceLock::new();

fn registry() -> &'static Mutex<FxHashMap<u64, CallbackFn>> {
  REGISTRY.get_or_init(|| Mutex::new(FxHashMap::default()))
}

/// Регистрирует Rust-клоужер под заданным id.
pub fn register(id: u64, f: CallbackFn) {
  registry().lock().unwrap().insert(id, f);
}

/// Вызывает ранее зарегистрированный клоужер.
pub fn invoke(id: u64, args: Vec<Value>) -> Value {
  let map = registry().lock().unwrap();
  let f = map.get(&id).expect("Callback not found in registry");
  f(args)
}