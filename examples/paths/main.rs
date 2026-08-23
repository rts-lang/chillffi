use chillffi::pathResolver::addGlobalSearchPath;
use chillffi::ffi::value::{Type, Value};
use chillffi::ffi;
// =================================================================================================

fn main() -> ()
{
  testRawPath();
  testScopePath();
  testGlobalPath();
}

/// Путь с '/' — PathResolver не участвует, идёт напрямую в dlopen.
fn testRawPath() -> ()
{
  let result: Value = ffi!{
    let lib: Library = Library::load("./examples/paths/libprint.so")?;
    Ok(lib.call("print", vec![Value::String(b"raw path\n".to_vec())], Type::Pointer)?)
  }.expect("raw path failed");

  assert!(matches!(result, Value::Pointer(0)));
  println!("ok: raw path");
}

/// Временный путь через scope — резолвится только внутри этого блока.
fn testScopePath() -> ()
{
  let result: Value = ffi!(|scope| {
    scope.addSearchPath("examples/paths");
    let lib: Library = Library::load("libprint.so")?;
    Ok(lib.call("print", vec![Value::String(b"scope path\n".to_vec())], Type::Pointer)?)
  }).expect("scope path failed");

  assert!(matches!(result, Value::Pointer(0)));
  println!("ok: scope path");
}

/// Глобальный путь — задан один раз, виден во всех последующих блоках.
fn testGlobalPath() -> ()
{
  addGlobalSearchPath("examples/paths");

  let result: Value = ffi!{
    let lib: Library = Library::load("libprint.so")?;
    Ok(lib.call("print", vec![Value::String(b"global path\n".to_vec())], Type::Pointer)?)
  }.expect("global path failed");

  assert!(matches!(result, Value::Pointer(0)));
  println!("ok: global path");
}

// =================================================================================================