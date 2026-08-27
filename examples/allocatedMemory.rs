use chillffi::ffi::value::Value;
use chillffi::ffi::errors::FFIError;
use chillffi::ffi::allocatedMemory::AllocatedMemory;
use chillffi::call;
use chillffi::ffi;
// =================================================================================================

/// stat(): C-string + struct out-param via alloc/read.
///
/// todo Нужно описание между строк, что тут происходит
fn main() -> ()
{
  let size: i64 = ffi!(|scope| {
    let libc: Library = Library::load("libc.so.6")?;

    // struct stat — 144 bytes on x86_64 Linux (glibc)
    let statMem: AllocatedMemory = scope.alloc(144)?;

    let result: i32 = call!(
      libc, "stat",
      Value::CString(b"/etc/hostname".to_vec()),
      statMem.asPointer()
    )?;
    if result != 0 {
      return Err(FFIError::Other("stat() returned non-zero".into()));
    }

    let Value::RawString(bytes) = statMem.read()? else {
      return Err(FFIError::Other("expected bytes".into()))
    };

    // st_size — offset 48
    Ok(i64::from_ne_bytes(bytes[48..56].try_into().unwrap()))
  }).expect("stat() failed");

  //
  println!("file size = {} bytes", size);

  let expected: u64 = std::fs::metadata("/etc/hostname").expect("metadata").len();
  assert_eq!(size as u64, expected);
  println!("ok: stat via libc");
}

// =================================================================================================