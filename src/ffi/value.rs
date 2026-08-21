use serde::{Deserialize, Serialize};
// =================================================================================================

/// A value that can be passed between processes
/// and used when calling FFI.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Value
{
  /// Just an empty value
  None,
  //
  U8(u8),
  U16(u16),
  U32(u32),
  U64(u64),
  Usize(usize), // todo Должно быть тут?
  //
  I8(i8),
  I16(i16),
  I32(i32),
  I64(i64),
  Isize(isize), // todo Должно быть тут?
  //
  F32(f32),
  F64(f64),
  //
  Bool(bool),
  /// В C коде ожидали бы `uint8_t *data`; 
  /// Но без len эти байты бесполезны и `size_t len` необходим.
  RawString(Vec<u8>),
  /// В C коде ожидали бы `const char *str`; `\0` terminated.
  CString(Vec<u8>),
  /// В C коде ожидали бы `const char *str, size_t len`
  String(Vec<u8>)
}

// =================================================================================================

/// Description of the value type 
/// for defining FFI arguments and result
#[derive(Serialize, Deserialize)]
pub enum Type
{
  /// Just an empty value
  None,
  //
  U8,
  U16,
  U32,
  U64,
  Usize, // todo Должно быть тут?
  //
  I8,
  I16,
  I32,
  I64,
  Isize, // todo Должно быть тут?
  //
  F32,
  F64,
  //
  Bool,
  /// Raw pointer
  Pointer
}

// =================================================================================================

#[cfg(test)]
mod tests
{
  use crate::tests::setup;
  use crate::ffi;
  use crate::ffi::value::{Type, Value};
  // ===============================================================================================

  /// Checks passing CString to a C function expecting a \0-terminated string (strlen).
  #[test]
  fn cString() -> ()
  {
    setup();

    let result: Value = ffi!{
      let libc: Library = Library::load("libc.so.6")?;
      let args: Vec<Value> = vec![Value::CString(b"hello".to_vec())];
      Ok(libc.call("strlen", args, Type::Usize)?)
    }.expect("FFI CString call failed");

    if let Value::Usize(len) = result {
      assert_eq!(len, 5);
    } else {
      panic!("Expected Value::Usize");
    }
  }

  /// Checks passing String which automatically expands to two C-ABI arguments (ptr + len)
  /// for functions accepting buffer pointer and max length (strnlen).
  #[test]
  fn string() -> ()
  {
    setup();

    let result: Value = ffi!{
      let libc: Library = Library::load("libc.so.6")?;
      let args: Vec<Value> = vec![Value::String(b"hello world".to_vec())];
      Ok(libc.call("strnlen", args, Type::Usize)?)
    }.expect("FFI String call failed");

    if let Value::Usize(len) = result {
      assert_eq!(len, 11);
    } else {
      panic!("Expected Value::Usize");
    }
  }

  /// Checks passing RawString as a single raw byte pointer (atoi).
  #[test]
  fn rawString() -> ()
  {
    setup();

    let result: Value = ffi!{
      let libc: Library = Library::load("libc.so.6")?;
      let args: Vec<Value> = vec![Value::RawString(b"12345\0".to_vec())];
      Ok(libc.call("atoi", args, Type::I32)?)
    }.expect("FFI RawString call failed");

    if let Value::I32(val) = result {
      assert_eq!(val, 12345);
    } else {
      panic!("Expected Value::I32");
    }
  }

  // ===============================================================================================
}

// =================================================================================================