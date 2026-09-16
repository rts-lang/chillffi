#[path = "../platform/mod.rs"]
mod platform;
use crate::platform::LibmPath;
// =================================================================================================
use chillffi::ffi;
use chillffi::ffi::errors::FFIError;
// =================================================================================================

/// Feature: [`Library`] itself — loading, calling, reading back its
/// resolved path, unloading (explicit and automatic), and the two ways
/// loading/calling can fail.
fn main() -> ()
{
  loadCallAndPath();
  explicitUnload();
  automaticDropOnScopeExit();
  libraryLoadFailed();
  symbolNotFound();
}

// =================================================================================================

/// `scope.load` doesn't touch the filesystem yet — it just resolves and
/// remembers a path string. `Library::path()` reports that resolved string,
/// so this also doubles as a sanity check that a bare name (no search paths
/// registered) resolves to itself.
fn loadCallAndPath() -> ()
{
  let (path, result): (String, f64) = ffi!(|scope| {
    let libm: Library = scope.load(LibmPath)?;
    let path: String = libm.path().to_string();
    let result: f64 = libm.call("sqrt").arg::<f64>(16.0).result()?;
    Ok((path, result))
  }).expect("load/call/path failed");

  assert_eq!(path, LibmPath);
  assert!((result - 4.0).abs() < f64::EPSILON);
  println!("ok: loaded '{path}', sqrt(16.0) = {result}");
}

/// `Library::unload` takes `self` by value — the compiler, not a runtime
/// check, is what prevents using the handle afterward.
fn explicitUnload() -> ()
{
  ffi!(|scope| {
    let libm: Library = scope.load(LibmPath)?;
    let result: f64 = libm.call("sqrt").arg::<f64>(9.0).result()?;
    assert!((result - 3.0).abs() < f64::EPSILON);
    libm.unload()?;
    // `libm` is gone here — any further use would be a compile error, not a panic.
    Ok(())
  }).expect("explicit unload failed");

  println!("ok: explicit unload()");
}

/// Same guarantee without calling `unload()` — plain scope-exit `Drop`
/// unregisters the library exactly the same way.
fn automaticDropOnScopeExit() -> ()
{
  ffi!(|scope| {
    let libm: Library = scope.load(LibmPath)?;
    let result: f64 = libm.call("sqrt").arg::<f64>(25.0).result()?;
    assert!((result - 5.0).abs() < f64::EPSILON);
    Ok(())
    // `libm` drops here.
  }).expect("automatic drop failed");

  println!("ok: automatic drop on scope exit");
}

/// `scope.load` succeeds for *any* string — resolution is lazy. The actual
/// `dlopen` only happens inside the clone on the first real call, which is
/// where a bad path is finally reported.
fn libraryLoadFailed() -> ()
{
  let err: FFIError = ffi!(|scope| {
    let bogus: Library = scope.load("libTotallyDoesNotExist9000.so")?;
    bogus.call("whatever").void()
  }).expect_err("loading a nonexistent library should fail");

  assert!(matches!(err, FFIError::LibraryLoadFailed{ .. }), "unexpected error: {err:?}");
  println!("ok: LibraryLoadFailed — {err}");
}

/// The library itself loads fine — it's the symbol lookup inside it that fails.
fn symbolNotFound() -> ()
{
  let err: FFIError = ffi!(|scope| {
    let libm: Library = scope.load(LibmPath)?;
    libm.call("thisSymbolDoesNotExistAnywhere").void()
  }).expect_err("calling a missing symbol should fail");

  assert!(matches!(err, FFIError::SymbolNotFound{ .. }), "unexpected error: {err:?}");
  println!("ok: SymbolNotFound — {err}");
}

// =================================================================================================
