use chillffi::ffi::value::{Type, Value};
use chillffi::ffi::errors::FFIError;
use chillffi::ffi;
// =================================================================================================

fn main() -> ()
{
  // clock_gettime(CLOCK_REALTIME, &timespec) — struct out-param via Alloc/ReadMemory,
  // the case a plain Value::Pointer can't cover on its own.
  let (secs, nanos): (i64, i64) = ffi!{
    let libc: Library = Library::load("libc.so.6")?;

    // struct timespec { time_t tv_sec; long tv_nsec; } — 16 bytes on x86_64 Linux
    let Value::Pointer(addr) = Library::alloc(16)? else { 
      return Err(FFIError::Other("expected pointer".into())) 
    };

    libc.call("clock_gettime", vec![Value::I32(0 /* CLOCK_REALTIME */), Value::Pointer(addr)], Type::I32)?;

    let Value::RawString(bytes) = Library::readMemory(addr, 16)? else { 
      return Err(FFIError::Other("expected bytes".into())) 
    };
    Library::free(addr)?;

    let secs: i64 = i64::from_ne_bytes(bytes[0..8].try_into().unwrap());
    let nanos: i64 = i64::from_ne_bytes(bytes[8..16].try_into().unwrap());
    Ok((secs, nanos))
  }.expect("clock_gettime failed");

  println!("clock_gettime(CLOCK_REALTIME) = {}.{:09}", secs, nanos);
}

// =================================================================================================