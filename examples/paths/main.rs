#[path = "../platform/mod.rs"]
mod platform;
// =================================================================================================
use chillffi::ffi;
use chillffi::ffi::types::primitive::Pointer;
use chillffi::pathResolver::addGlobalSearchPath;
// =================================================================================================

/// Test library resolution using direct paths, 
/// scope search paths, and global search paths
fn main() -> ()
{
  rawPath();
  scopePath();
  globalPath();
}

// =================================================================================================

/// A path with '/' — PathResolver is not involved; it goes directly to dlopen.
fn rawPath() -> ()
{
  let result: Pointer = ffi!(|scope| {
    let libprint: Library = scope.load(platformExt!("./examples/paths/libprint"))?;
    libprint.call("print")
      .arg("raw path\n")
      .result()
  }).expect("raw path failed");

  assert!(matches!(result, Pointer(0)));
  println!("ok: raw path");
}

/// Temporary path through scope — resolves only inside this block.
fn scopePath() -> ()
{
  let result: Pointer = ffi!(|scope| {
    scope.addSearchPath("examples/paths");
    let libprint: Library = scope.load(platformExt!("libprint"))?;
    libprint.call("print")
      .arg("scope path\n")
      .result()
  }).expect("scope path failed");

  assert!(matches!(result, Pointer(0)));
  println!("ok: scope path");
}

/// The global path — set once, visible in all subsequent blocks.
fn globalPath() -> ()
{
  addGlobalSearchPath("examples/paths");

  let result: Pointer = ffi!(|scope| {
    let libprint: Library = scope.load(platformExt!("libprint"))?;
    libprint.call("print")
      .arg("global path\n")
      .result()
  }).expect("global path failed");

  assert!(matches!(result, Pointer(0)));
  println!("ok: global path");
}

// =================================================================================================