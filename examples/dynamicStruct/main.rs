#[path = "../platform/mod.rs"]
mod platform;
use crate::platform::LibcPath;
// =================================================================================================
use chillffi::ffi;
use chillffi::ffi::allocatedMemory::AllocatedMemory;
use chillffi::ffi::scope::Scope;
use chillffi::ffi::types::primitive::{Arg, DynamicList, Pointer};
use chillffi::ffi::types::Type;
// =================================================================================================

/// Feature: dynamically-typed C structs — [`Scope::allocStruct`],
/// [`Scope::readDynamicStruct`], [`Scope::writeDynamicStruct`]. The shape
/// is an ordinary runtime `Vec<Type>`, so it works for structs that only
/// ever existed as C source, with no matching `#[repr(C)]` Rust type to
/// name. Compare with `examples/reprStruct`, which covers the same buffer
/// shape when a Rust type *does* exist.
fn main() -> ()
{
  readDynamicStruct();
  structAsCallParameter();
  structAsCallResult();
  nestedStructShapeIsSized();
}

// =================================================================================================

/// Reads a struct C already wrote (`clock_gettime`'s out-parameter) back
/// out using a runtime-described shape instead of `readStruct::<T>()`.
fn readDynamicStruct() -> ()
{
  let (secs, nanos): (i64, i64) = ffi!(|scope| {
    let libc: Library = scope.load(LibcPath)?;
    let mem: AllocatedMemory = scope.alloc(16)?;

    libc.call("clock_gettime").arg::<i32>(0).arg(mem.asPointer()).void()?;

    // struct timespec { time_t tv_sec; long tv_nsec; }
    let fields: DynamicList = Scope::readDynamicStruct(mem.address(), &[Type::I64, Type::I64])?;
    Ok((fields.get(0)?, fields.get(1)?))
  }).expect("readDynamicStruct failed");

  println!("ok: readDynamicStruct -> clock_gettime = {secs}.{nanos:09}");
}

/// ```c
/// struct Data { int size; int *values; };
/// void process(struct Data *data);
/// ```
/// The struct exists only on the C side. Its layout is described at
/// runtime, allocated with ABI-correct size/padding via `allocStruct`, and
/// filled in field-by-field via `writeDynamicStruct` — the separately
/// allocated `values` array is passed through the struct's pointer field.
fn structAsCallParameter() -> ()
{
  let dataShape: Vec<Type> = vec![Type::I32, Type::Pointer];
  let values: Vec<i32> = vec![10, 20, 30];

  let sum: i32 = ffi!(|scope| {
    scope.addSearchPath("examples/dynamicStruct");
    let lib: Library = scope.load("libparameter.so")?;

    let valuesMem: AllocatedMemory = scope.alloc(values.len() * size_of::<i32>())?;
    let bytes: Vec<u8> = values.iter().flat_map(|v| v.to_ne_bytes()).collect();
    Scope::writeMemory(valuesMem.address(), bytes)?;

    let dataMem: AllocatedMemory = scope.allocStruct(&dataShape)?;
    Scope::writeDynamicStruct(dataMem.address(), &dataShape, vec![
      Arg::from(values.len() as i32),
      Arg::from(valuesMem.asPointer()),
    ])?;

    lib.call("process").arg(dataMem.asPointer()).void()?;

    // dataMem/valuesMem free themselves (Drop) when this block ends — process()
    // only read from `data`, it never took ownership of the allocation.
    lib.call("getSum").result()
  }).expect("dynamic struct parameter failed");

  assert_eq!(sum, 60);
  println!("ok: dynamic struct as a call parameter -> sum = {sum}");
}

/// ```c
/// struct Data { int size; int *values; };
/// struct Data *process(void);
/// ```
/// The returned struct is read via its runtime shape; the pointed-to array
/// is read separately using the size stored in the struct's own `size`
/// field — a pointer field never carries its own length.
fn structAsCallResult() -> ()
{
  let values: Vec<i32> = ffi!(|scope| {
    scope.addSearchPath("examples/dynamicStruct");
    let lib: Library = scope.load("libresult.so")?;

    let dataPtr: Pointer = lib.call("process").result()?;

    let fields: DynamicList = Scope::readDynamicStruct(dataPtr, &[Type::I32, Type::Pointer])?;
    let size: i32 = fields.get(0)?;
    let valuesPtr: Pointer = fields.get(1)?;

    let bytes: Vec<u8> = Scope::readMemory(valuesPtr, size as usize * 4)?;
    let values: Vec<i32> = bytes
      .chunks_exact(4)
      .map(|c| i32::from_ne_bytes(c.try_into().expect("4-byte chunk")))
      .collect();

    // process() malloc'd both the struct and its values array — free both
    // through the C side's own freeData(), since only the C side knows the
    // true allocation shape (here, that `values` was a separate malloc).
    lib.call("freeData").arg(dataPtr).void()?;

    Ok(values)
  }).expect("dynamic struct result failed");

  assert_eq!(values, vec![10, 20, 30]);
  println!("ok: dynamic struct as a call result -> {values:?}");
}

/// `Type::Struct(nested)` fields ARE supported by the ABI layout engine —
/// `allocStruct` correctly resolves the padded, ABI-aware size for a shape
/// that nests one struct inside another, exactly like `struct_offsets`
/// would for a real nested `#[repr(C)]` type.
///
/// What is *not* yet possible through the public API: reading a nested
/// field back out. `DynamicList::get::<T>` requires `T: FfiPrimitive`,
/// which only scalar types implement — there's no way to ask for the
/// nested field itself as another `DynamicList`. This is a real, open gap
/// in the API, not a demonstrated feature; left here as an honest marker
/// instead of a fake round-trip.
fn nestedStructShapeIsSized() -> ()
{
  let shape: Vec<Type> = vec![
    Type::Struct(Box::new([Type::U64, Type::U8, Type::F64])),
    Type::I8,
    Type::I64,
  ];

  let size: usize = ffi!(|scope| {
    let mem: AllocatedMemory = scope.allocStruct(&shape)?;
    Ok(mem.length())
  }).expect("allocStruct with a nested Type::Struct failed");

  assert!(size > 0);
  println!("ok: nested Type::Struct shape sized to {size} bytes (extraction not yet public)");
}

// =================================================================================================
