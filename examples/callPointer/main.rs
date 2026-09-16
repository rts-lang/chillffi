#[path = "../platform/mod.rs"]
mod platform;
use crate::platform::LibcPath;
// =================================================================================================
use chillffi::callback;
use chillffi::callvPointer;
use chillffi::ffi;
use chillffi::ffi::types::primitive::{Callback, Pointer};
// =================================================================================================

/// Feature: [`Scope::callPointer`]/[`callvPointer!`] — calling a raw
/// function pointer directly, with no `dlopen`/`dlsym` involved, because
/// the address is already known. Verified here by round-tripping through
/// `signal()`, which both takes and returns a function pointer.
fn main() -> ()
{
  callvPointerOnSignalReturnedHandler();
}

// =================================================================================================

/// `signal()` returns the *previous* handler as a raw address. Installing
/// our own Rust callback first, then asking `signal()` to swap it back to
/// `SIG_DFL`, gets that exact address back — a real, known-good pointer to
/// call through `callvPointer!`, bypassing `signal()`/`dlsym` entirely.
fn callvPointerOnSignalReturnedHandler() -> ()
{
  ffi!(|scope| {
    let libc: Library = scope.load(LibcPath)?;

    let handler: Callback = callback!(scope, |signum: i32| -> () {
      println!("[handler] called directly via callvPointer!, signum = {signum}");
    });

    // Install it. The signal is never raised — signal() only stores and
    // returns pointers, delivery is irrelevant here.
    libc.call("signal").arg::<i32>(10 /* SIGUSR1 */).arg(handler).void()?;

    // Restore SIG_DFL and capture what signal() reports as "previous" —
    // has to be the exact address just installed above.
    let old: Pointer = libc.call("signal").arg::<i32>(10).arg(Pointer(0)).result()?;

    // Call that address directly, bypassing signal() entirely.
    callvPointer!(scope, old, 10_i32)?;

    Ok(())
  }).expect("signal roundtrip failed");

  println!("ok: the pointer signal() returned was a real, callable callback");
}

// =================================================================================================
