use std::cmp::Ordering;
use chillffi::ffi::value::{Value, Type};
use chillffi::callv;
use chillffi::ffi;
use chillffi::ffi::allocatedMemory::AllocatedMemory;
use serde_closure::Fn;
// =================================================================================================

/// todo desc
fn main()
{
  println!("=== Starting qsort via chillffi ===\n");

  let sorted: Vec<i32> = ffi!(|scope| {
    let libc: Library = Library::load("libc.so.6")?;
    println!("[ffi!] Loaded libc.so.6");

    let mem: AllocatedMemory = scope.alloc(5 * 4)?;
    let ptrAddr: usize = match mem.asPointer() {
      Value::Pointer(addr) => addr,
      other => panic!("expected Pointer, got {:?}", other),
    };
    println!("[ffi!] Allocated 20 bytes at address: 0x{:X}", ptrAddr);

    let data: [i32; 5] = [3, 1, 4, 1, 5];
    println!("[ffi!] Original data: {:?}", data);

    let raw: &[u8] = unsafe { std::slice::from_raw_parts(data.as_ptr() as *const u8, 20) };
    mem.write(Value::RawString(raw.to_vec()))?;
    println!("[ffi!] Written raw bytes to clone memory\n");

    let compar: Value = scope.callback(
      vec![Type::Pointer, Type::Pointer],
      Type::I32,
      Fn!(|args: Vec<Value>| -> Value {
        let a: usize = match args[0] {
            Value::Pointer(a) => a,
            _ => panic!("expected Pointer for arg 0"),
        };
        let b: usize = match args[1] {
            Value::Pointer(b) => b,
            _ => panic!("expected Pointer for arg 1"),
        };

        // Прямое разыменование — мы внутри клона, та же память
        let av: i32 = unsafe { *(a as *const i32) };
        let bv: i32 = unsafe { *(b as *const i32) };

        let cmp: Ordering = av.cmp(&bv);
        let result: i32 = cmp as i32;

        println!(
          "  [callback] comparing *0x{:X} = {}  vs  *0x{:X} = {}  =>  {}",
          a, av, b, bv,
          match cmp {
            std::cmp::Ordering::Less    => "LESS (return -1)",
            std::cmp::Ordering::Equal   => "EQUAL (return 0)",
            std::cmp::Ordering::Greater => "GREATER (return 1)",
          }
        );

        Value::I32(result)
      }),
    );
    println!("[ffi!] Registered comparator callback\n");

    println!("[ffi!] Calling qsort(mem, 5, 4, compar)...");
    callv!(libc, "qsort",
      mem.asPointer(),
      5 as usize,
      4 as usize,
      compar
    )?;
    println!("[ffi!] qsort returned\n");

    let Value::RawString(bytes) = mem.read()? else { panic!() };
    println!("[ffi!] Read back raw bytes: {:?}\n", bytes);

    let vec: Vec<i32> = bytes.chunks_exact(4)
      .map(|b| i32::from_ne_bytes(b.try_into().unwrap()))
      .collect();

    println!("[ffi!] Decoded to i32 vector: {:?}", vec);

    Ok(vec)
  }).expect("qsort failed");

  println!("\n=== Result outside ffi! block ===");
  println!("sorted = {:?}", sorted);
  assert_eq!(sorted, vec![1, 1, 3, 4, 5]);
  println!("Assertion passed: [1, 1, 3, 4, 5] ✓");
}

// =================================================================================================