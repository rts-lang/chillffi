#[path = "../platform/mod.rs"]
mod platform;
use crate::platform::LibcPath;
// =================================================================================================
use chillffi::callback;
use chillffi::ffi;
use chillffi::ffi::allocatedMemory::AllocatedMemory;
use chillffi::ffi::types::primitive::Callback;
use chillffi::ffi::types::primitive::Pointer;
use std::cmp::Ordering;
// =================================================================================================

/// Feature: [`callback!`] — registering a Rust closure so C can call it as
/// a function pointer (e.g. a `qsort` comparator). Capture is automatic,
/// exactly like an ordinary closure — no explicit `[]` list, no
/// `serde_closure`. The catch: captured variables must be `Copy + Send`,
/// since the closure's state crosses the fork as a raw bit-copy.
fn main() -> ()
{
  noCaptures();
  multipleCapturedVariables();
}

// =================================================================================================

/// Baseline: a comparator with nothing captured from the environment at all.
fn noCaptures() -> ()
{
  let sorted: Vec<i32> = ffi!(|scope| {
    let libc: Library = scope.load(LibcPath)?;
    let mem: AllocatedMemory = scope.alloc(5 * 4)?;

    let data: [i32; 5] = [3, 1, 4, 1, 5];
    let raw: &[u8] = unsafe { std::slice::from_raw_parts(data.as_ptr() as *const u8, 20) };
    mem.write(raw)?;

    // Direct dereferencing is correct: the closure runs inside the clone
    // (where the data resides), not in the parent process.
    let compar: Callback = callback!(scope, |a: Pointer, b: Pointer| -> i32 {
      let av: i32 = unsafe { *(a.0 as *const i32) };
      let bv: i32 = unsafe { *(b.0 as *const i32) };
      av.cmp(&bv) as i32
    });

    libc.call("qsort")
      .arg(mem.asPointer())
      .arg::<usize>(5)
      .arg::<usize>(4)
      .arg(compar)
      .void()?;

    let bytes: Vec<u8> = mem.read()?;
    Ok(bytes.chunks_exact(4).map(|b| i32::from_ne_bytes(b.try_into().unwrap())).collect())
  }).expect("qsort failed");

  assert_eq!(sorted, vec![1, 1, 3, 4, 5]);
  println!("ok: no-capture comparator -> {sorted:?}");
}

/// Two unrelated captured variables of *different* types (`i32` and
/// `bool`), no explicit capture list for either — both actually travel
/// into the clone that runs the comparator, not just "visible by accident".
fn multipleCapturedVariables() -> ()
{
  let bias: i32 = 3; // Shifts every value before comparing
  let descending: bool = true; // Flips the resulting order

  let sorted: Vec<i32> = ffi!(|scope| {
    let libc: Library = scope.load(LibcPath)?;
    let mem: AllocatedMemory = scope.alloc(5 * 4)?;

    let data: [i32; 5] = [3, 1, 4, 1, 5];
    let raw: &[u8] = unsafe { std::slice::from_raw_parts(data.as_ptr() as *const u8, 20) };
    mem.write(raw)?;

    let compar: Callback = callback!(scope, |a: Pointer, b: Pointer| -> i32 {
      let av: i32 = unsafe { *(a.0 as *const i32) };
      let bv: i32 = unsafe { *(b.0 as *const i32) };

      let cmp: Ordering = if descending {
        (bv + bias).cmp(&(av + bias))
      } else {
        (av + bias).cmp(&(bv + bias))
      };
      cmp as i32
    });

    libc.call("qsort")
      .arg(mem.asPointer())
      .arg::<usize>(5)
      .arg::<usize>(4)
      .arg(compar)
      .void()?;

    let bytes: Vec<u8> = mem.read()?;
    Ok(bytes.chunks_exact(4).map(|b| i32::from_ne_bytes(b.try_into().unwrap())).collect())
  }).expect("qsort failed");

  assert_eq!(sorted, vec![5, 4, 3, 1, 1]);
  println!("ok: multi-capture comparator (bias={bias}, descending={descending}) -> {sorted:?}");
}

// =================================================================================================
