use chillffi::ffi::value::{Value, Type};
use chillffi::ffi::errors::FFIError;
use chillffi::callv;
use chillffi::ffi;
// =================================================================================================

/// todo desc
fn main() -> ()
{
  let sorted_bytes: Vec<u8> = ffi!(|scope| {
    let libc = Library::load("libc.so.6")?;

    // 1. Выделяем память в клоне, кладём туда [3,1,4,1,5]
    let mem = scope.alloc(5 * 4)?; // 5 i32
    let data: [i32; 5] = [3, 1, 4, 1, 5];
    let raw = unsafe {
      std::slice::from_raw_parts(data.as_ptr() as *const u8, 20)
    };
    mem.write(Value::RawString(raw.to_vec()))?;

    // 2. Создаём колбек compar(const void*, const void*) -> int
    let compar = scope.callback(
      vec![Type::Pointer, Type::Pointer],
      Type::I32,
      |args| {
        let Value::Pointer(a) = args[0] else { panic!("a") };
        let Value::Pointer(b) = args[1] else { panic!("b") };

        // Важно: a и b — адреса внутри клона (внутри mem).
        // Колбек выполняется в клоне, поэтому разыменование безопасно.
        let av = unsafe { *(a as *const i32) };
        let bv = unsafe { *(b as *const i32) };

        Value::I32(av.cmp(&bv) as i32)
      },
    );

    // 3. Вызываем qsort(base, nmemb, size, compar)
    callv!(libc, "qsort",
      mem.asPointer(),          // void *base
      5 as usize,               // size_t nmemb
      4 as usize,               // size_t size
      compar                    // int (*compar)(const void*, const void*)
    )?;

    // 4. Читаем отсортированный результат обратно в родителя
    let Value::RawString(bytes) = mem.read()? else {
      return Err(FFIError::Other("expected bytes".into()));
    };
    Ok(bytes)
  }).expect("qsort failed");

  // Проверяем
  let sorted: Vec<i32> = sorted_bytes.chunks_exact(4)
    .map(|b| i32::from_ne_bytes(b.try_into().unwrap()))
    .collect();

  assert_eq!(sorted, vec![1, 1, 3, 4, 5]);
  println!("ok: qsort callback roundtrip");
}