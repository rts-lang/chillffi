#[path = "../platform/mod.rs"]
mod platform;
use crate::platform::LibcPath;
// =================================================================================================
use chillffi::ffi;
// =================================================================================================

/// Feature: string arguments. `.arg(...)` accepts three different Rust
/// string-ish types, and each one crosses the C ABI boundary differently —
/// this is about *that* difference, not about strings in general.
fn main() -> ()
{
  string();
  cString();
  rawString();
}

// =================================================================================================

/// `String`/`&str` → [`Value::String`] → *two* C arguments automatically:
/// pointer, then length. `strnlen(const char *s, size_t maxlen)` is exactly
/// that shape, so a single `.arg("...")` fills both parameters by itself.
fn string() -> ()
{
  let result: usize = ffi!(|scope| {
    let libc: Library = scope.load(LibcPath)?;
    libc.call("strnlen").arg("hello world").result()
  }).expect("String call failed");

  assert_eq!(result, 11);
  println!("ok: strnlen(\"hello world\") = {result}  (auto-split into 2 C args: pointer + len)");
}

/// `CString`/`&CStr` (e.g. a `c"..."` literal) → [`Value::CString`] → a
/// single C argument: a `\0`-terminated pointer. This is what a plain
/// `const char *` parameter expects.
fn cString() -> ()
{
  let result: usize = ffi!(|scope| {
    let libc: Library = scope.load(LibcPath)?;
    libc.call("strlen").arg(c"chillffi").result()
  }).expect("CString call failed");

  assert_eq!(result, 8);
  println!("ok: strlen(c\"chillffi\") = {result}  (1 C arg: pointer)");
}

/// `Vec<u8>`/`&[u8]` → [`Value::RawString`] → a single C argument: a raw
/// pointer, with *no* `\0` guarantee and no length pairing. Safe here only
/// because the byte vector supplies its own terminator by hand.
fn rawString() -> ()
{
  let result: i32 = ffi!(|scope| {
    let libc: Library = scope.load(LibcPath)?;
    libc.call("atoi").arg(b"12345\0".to_vec()).result()
  }).expect("RawString call failed");

  assert_eq!(result, 12345);
  println!("ok: atoi(b\"12345\\0\") = {result}  (1 C arg: pointer, no auto length/NUL)");
}

// =================================================================================================
