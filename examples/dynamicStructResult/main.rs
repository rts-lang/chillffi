use chillffi::ffi::types::Type;
use chillffi::ffi::types::primitive::{DynamicList, Pointer};
use chillffi::ffi::scope::Scope;
use chillffi::ffi;
// =================================================================================================

/// todo desc (переписать проще)
/// 
/// Mirrors chillffi issue #18:
///
/// ```c
/// struct Data { int size; int *values; };
/// struct Data *process();
/// ```
///
/// `process()` returns a *pointer* — at the C ABI level that's just a
/// register-sized integer, so it's already callable with the ordinary
/// `Pointer` result type; no special "struct return" support is needed for
/// this shape. (Returning a struct *by value* is a separate, harder
/// feature — see the `Type::Struct` todo in `worker.rs::invokeFFI`.)
///
/// What issue #18 actually needs is this recipe: declare the struct's
/// layout as `Vec<Type>`, then dereference the returned pointer with
/// [`Scope::readDynamicStruct`]. A field that is itself a pointer (`int
/// *values`) is read as `Type::Pointer` and chased separately with
/// [`Scope::readMemory`] — same as in C itself, there's no length
/// information at the type level for a raw pointer, only in `size`, a
/// sibling field.
fn main() -> ()
{
  // struct Data { int size; int *values; }
  let dataShape: Vec<Type> = vec![Type::I32, Type::Pointer];

  let values: Vec<i32> = ffi!(|scope| {
    scope.addSearchPath("examples/dynamicStructResult");
    let lib: Library = scope.load("libdata.so")?;

    // struct Data *process();
    let dataPtr: Pointer = lib.call("process").result()?;

    // Dereference the returned pointer using the known struct layout.
    let fields: DynamicList = Scope::readDynamicStruct(dataPtr, &dataShape)?;
    let size: i32 = fields.get(0)?;
    let valuesPtr: Pointer = fields.get(1)?;

    // `values` is itself a pointer — `size` (its sibling field) is the only
    // place its length lives, so it's a manual readMemory + reinterpret,
    // same as it would be in any other C FFI binding.
    let bytes: Vec<u8> = Scope::readMemory(valuesPtr, size as usize * 4)?;
    let values: Vec<i32> = bytes
      .chunks_exact(4)
      .map(|c| i32::from_ne_bytes(c.try_into().expect("4-byte chunk")))
      .collect();

    // process() malloc'd both the struct and its values array — free both
    // through the C side's own freeData(), not field-by-field via
    // Scope::free(), since only the C side actually knows the true
    // allocation shape (and here, that values itself was a separate malloc).
    lib.call("freeData").arg(dataPtr).void()?;

    Ok(values)
  }).expect("dynamic struct result failed");

  //
  println!("process() -> Data {{ size: {}, values: {:?} }}", values.len(), values);
  assert_eq!(values, vec![10, 20, 30]);
  println!("ok: dynamic struct as a call result");
}

// =================================================================================================