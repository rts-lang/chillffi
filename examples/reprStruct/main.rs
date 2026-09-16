#[path = "../platform/mod.rs"]
mod platform;
use crate::platform::LibcPath;
// =================================================================================================
use bytemuck::{Pod, Zeroable};
use chillffi::ffi;
use chillffi::ffi::allocatedMemory::AllocatedMemory;
// =================================================================================================

/// struct timespec { time_t tv_sec; long tv_nsec; } — 16 bytes on x86_64 Linux.
/// `#[repr(C)]` is mandatory — otherwise Rust is free to reorder the fields.
/// `derive(Pod)` checks at compile time that there is no padding: if there
/// were, for example, i32+i64 without an explicit `_pad`, bytemuck would
/// refuse to compile rather than silently letting you read garbage from the gap.
#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable, Debug, PartialEq)]
struct Timespec { secs: i64, nanos: i64 }

/// Feature: [`AllocatedMemory::readStruct`]/[`writeStruct`] — a
/// compile-time-typed, `#[repr(C)]` view over a raw buffer. Compare with
/// `examples/dynamicStruct`, which describes the exact same kind of shape
/// at runtime instead, for when no Rust type exists to name.
fn main() -> ()
{
  manualByteParsing();
  readStructRemovesManualParsing();
  writeThenReadStructRoundtrip();
}

// =================================================================================================

/// BEFORE: what every example before this one did — slice indices and
/// `from_ne_bytes` by hand. Kept here only as the baseline the next test
/// improves on.
fn manualByteParsing() -> ()
{
  let (secs, nanos): (i64, i64) = ffi!(|scope| {
    let libc: Library = scope.load(LibcPath)?;
    let mem: AllocatedMemory = scope.alloc(16)?;

    libc.call("clock_gettime").arg::<i32>(0 /* CLOCK_REALTIME */).arg(mem.asPointer()).void()?;

    let bytes: Vec<u8> = mem.read()?;
    let secs: i64 = i64::from_ne_bytes(bytes[0..8].try_into().unwrap());
    let nanos: i64 = i64::from_ne_bytes(bytes[8..16].try_into().unwrap());
    Ok((secs, nanos))
  }).expect("manual parsing failed");

  println!("manual:  clock_gettime = {secs}.{nanos:09}");
}

/// AFTER: `readStruct` removes all manual parsing — size and field layout
/// are checked by the type instead of by hand-calculated slices.
fn readStructRemovesManualParsing() -> ()
{
  let ts: Timespec = ffi!(|scope| {
    let libc: Library = scope.load(LibcPath)?;
    let mem: AllocatedMemory = scope.alloc(size_of::<Timespec>())?;

    libc.call("clock_gettime").arg::<i32>(0 /* CLOCK_REALTIME */).arg(mem.asPointer()).void()?;

    mem.readStruct::<Timespec>()
  }).expect("readStruct failed");

  println!("typed:   clock_gettime = {}.{:09}", ts.secs, ts.nanos);
}

/// `writeStruct` is the write-side mirror — fills the buffer directly from
/// a Rust value, entirely on our side, no C call needed to prove the
/// roundtrip is byte-exact.
fn writeThenReadStructRoundtrip() -> ()
{
  let (original, readBack): (Timespec, Timespec) = ffi!(|scope| {
    let mem: AllocatedMemory = scope.alloc(size_of::<Timespec>())?;

    let original: Timespec = Timespec { secs: 1_700_000_000, nanos: 123_456_789 };
    mem.writeStruct(&original)?;

    let readBack: Timespec = mem.readStruct::<Timespec>()?;
    Ok((original, readBack))
  }).expect("writeStruct/readStruct roundtrip failed");

  assert_eq!(original, readBack, "readStruct should return exactly what writeStruct wrote");
  println!("ok: writeStruct/readStruct roundtrip -> {readBack:?}");
}

// =================================================================================================
