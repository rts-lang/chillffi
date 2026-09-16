#[path = "../platform/mod.rs"]
mod platform;
use crate::platform::LibmPath;
// =================================================================================================
use chillffi::ffi;
use chillffi::ffi::errors::FFIError;
// =================================================================================================

/// Feature: crash isolation — the headline guarantee of this crate. A
/// crash inside an isolated clone must surface as `Err`, never unwind or
/// take down the process that ran this example, and must never affect any
/// `ffi!` block that comes after it.
fn main() -> ()
{
  segfaultIsContained();
  abortIsContained();
  runtimeSurvivesAfterCrash();
}

// =================================================================================================

/// A real SIGSEGV inside the clone. If isolation didn't work, this
/// process — the one printing these lines — would die with it.
fn segfaultIsContained() -> ()
{
  let result: Result<(), FFIError> = ffi!(|scope| {
    scope.addSearchPath("examples/isolation");
    let lib: Library = scope.load(platformExt!("libcrash"))?;
    lib.call("triggerSegfault").void()
  });

  let err: FFIError = result.expect_err("a segfaulting clone must not report success");
  assert!(matches!(err, FFIError::ZygoteCommunicationFailed(_)), "unexpected error: {err:?}");
  println!("ok: segfault contained — {err}");
}

/// Same guarantee for `abort()` (SIGABRT) — a different signal, same boundary.
fn abortIsContained() -> ()
{
  let result: Result<(), FFIError> = ffi!(|scope| {
    scope.addSearchPath("examples/isolation");
    let lib: Library = scope.load(platformExt!("libcrash"))?;
    lib.call("triggerAbort").void()
  });

  let err: FFIError = result.expect_err("an aborting clone must not report success");
  assert!(matches!(err, FFIError::ZygoteCommunicationFailed(_)), "unexpected error: {err:?}");
  println!("ok: abort() contained — {err}");
}

/// The whole point: after two crashed clones, a completely unrelated
/// `ffi!` block still works. Each `ffi!` forks its own fresh clone from
/// the always-alive main zygote — a crashed *clone* never touches the main
/// zygote itself, so no supervisor restart or delay is even needed here.
fn runtimeSurvivesAfterCrash() -> ()
{
  let result: f64 = ffi!(|scope| {
    let libm: Library = scope.load(LibmPath)?;
    libm.call("sqrt").arg::<f64>(16.0).result()
  }).expect("runtime should still work after two crashed clones");

  assert!((result - 4.0).abs() < f64::EPSILON);
  println!("ok: runtime alive after crashes, sqrt(16) = {result}");
}

// =================================================================================================
