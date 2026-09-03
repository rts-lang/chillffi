use chillffi::ffi::types::Type;
use chillffi::ffi::types::primitive::Arg;
use chillffi::ffi::allocatedMemory::AllocatedMemory;
use chillffi::ffi::scope::Scope;
use chillffi::ffi;
// =================================================================================================

/// todo desc (переписать проще)
///
/// Mirrors chillffi issue #17:
///
/// ```c
/// struct Data { int size; int *values; };
/// void process(struct Data *data);
/// ```
///
/// `data` here is a pointer built entirely on the Rust side — the struct
/// only exists as C source, so there's no Rust type to run `size_of` on for
/// the allocation. [`Scope::allocStruct`] resolves the correct (ABI-aware,
/// padding included) byte size from the field shape instead of it being
/// guessed by hand, and [`Scope::writeDynamicStruct`] fills it in using
/// that same layout math.
fn main() -> ()
{
  // struct Data { int size; int *values; }
  let dataShape: Vec<Type> = vec![Type::I32, Type::Pointer];
  let values: Vec<i32> = vec![10, 20, 30];

  let sum: i32 = ffi!(|scope| {
    scope.addSearchPath("examples/dynamicStructParameter");
    let lib: Library = scope.load("libdata.so")?;

    // int values[3] = { 10, 20, 30 }; — a plain byte buffer, not a struct,
    // so an ordinary byte-length alloc is exactly right here.
    let valuesMem: AllocatedMemory = scope.alloc(values.len() * size_of::<i32>())?;
    let bytes: Vec<u8> = values.iter().flat_map(|v| v.to_ne_bytes()).collect();
    Scope::writeMemory(valuesMem.address(), bytes)?;

    // struct Data *data = malloc(sizeof(struct Data));
    let dataMem: AllocatedMemory = scope.allocStruct(&dataShape)?;
    Scope::writeDynamicStruct(dataMem.address(), &dataShape, vec![
      Arg::from(values.len() as i32),
      Arg::from(valuesMem.asPointer()),
    ])?;

    // process(data);
    lib.call("process").arg(dataMem.asPointer()).void()?;

    // dataMem/valuesMem free themselves (Drop) when this block ends — process()
    // only read from `data`, it never took ownership of the allocation.
    lib.call("getSum").result::<i32>()
  }).expect("dynamic struct parameter failed");

  println!("process(Data {{ size: {}, values: {:?} }}) -> sum = {sum}", values.len(), values);
  assert_eq!(sum, 60);
  println!("ok: dynamic struct as a call parameter");
}

// =================================================================================================