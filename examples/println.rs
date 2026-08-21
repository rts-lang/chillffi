use chillffi::ffi::value::{Type, Value};
use chillffi::ffi::errors::FFIError;
use chillffi::ffi;
// =================================================================================================

fn println(text: &str) -> Result<(), FFIError>
{
  ffi!{
    let libc: Library = Library::load("libc.so.6")?;

    // C-string null termination
    let mut bytes: Vec<u8> = text.as_bytes().to_vec();
    bytes.push(0);

    let Value::Pointer(addr) = Library::alloc(bytes.len())? else {
      return Err(FFIError::Other("expected pointer".into()))
    };

    Library::writeMemory(addr, Value::RawString(bytes))?;

    // puts(const char *s) automatically appends a newline
    libc.call("puts", vec![Value::Pointer(addr)], Type::I32)?;

    Library::free(addr)?;
    Ok(())
  }
}

fn main() -> ()
{
  // Test: Print custom message via libc's puts
  println("Hello from libc via chillffi!").expect("Failed to print via libc");
}

// =================================================================================================