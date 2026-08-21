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
///
/// todo: add unit tests for Type variants verification
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

  /// Checks all signed integer types (I8, I16, I32, I64, Isize).
  #[test]
  fn signedIntegers() -> ()
  {
    setup();

    ffi!{
      let libc: Library = Library::load("libc.so.6")?;

      // I8 & I16
      let resI8 = libc.call("abs", vec![Value::I8(-5)], Type::I8)?;
      let resI16 = libc.call("abs", vec![Value::I16(-15)], Type::I16)?;
      assert!(matches!(resI8, Value::I8(5)));
      assert!(matches!(resI16, Value::I16(15)));

      // I32, I64 & Isize
      let resI32 = libc.call("abs", vec![Value::I32(-42)], Type::I32)?;
      let resI64 = libc.call("labs", vec![Value::I64(-100000)], Type::I64)?;
      let resIsize = libc.call("labs", vec![Value::Isize(-500)], Type::Isize)?;
      assert!(matches!(resI32, Value::I32(42)));
      assert!(matches!(resI64, Value::I64(100000)));
      assert!(matches!(resIsize, Value::Isize(500)));

      Ok(())
    }.expect("Signed integers test failed");
  }

  /// Checks all unsigned integer types (U8, U16, U32, U64, Usize).
  #[test]
  fn unsignedIntegers() -> ()
  {
    setup();

    ffi!{
      let libc: Library = Library::load("libc.so.6")?;

      // U8, U16, U32
      let resU8 = libc.call("strnlen", vec![Value::CString(b"a".to_vec()), Value::U8(10)], Type::U8)?;
      let resU16 = libc.call("strnlen", vec![Value::CString(b"ab".to_vec()), Value::U16(10)], Type::U16)?;
      let resU32 = libc.call("strnlen", vec![Value::CString(b"abc".to_vec()), Value::U32(10)], Type::U32)?;
      assert!(matches!(resU8, Value::U8(1)));
      assert!(matches!(resU16, Value::U16(2)));
      assert!(matches!(resU32, Value::U32(3)));

      // U64 & Usize
      let resU64 = libc.call("strnlen", vec![Value::CString(b"abcd".to_vec()), Value::U64(10)], Type::U64)?;
      let resUsize = libc.call("strnlen", vec![Value::CString(b"abcde".to_vec()), Value::Usize(10)], Type::Usize)?;
      assert!(matches!(resU64, Value::U64(4)));
      assert!(matches!(resUsize, Value::Usize(5)));

      Ok(())
    }.expect("Unsigned integers test failed");
  }

  /// Checks passing floating point numbers (F32, F64).
  #[test]
  fn floatTypes() -> ()
  {
    setup();

    let resultF32: Value = ffi!{
      let libm: Library = Library::load("libm.so.6")?;
      let args: Vec<Value> = vec![Value::F32(16.0)];
      Ok(libm.call("sqrtf", args, Type::F32)?)
    }.expect("FFI F32 call failed");

    if let Value::F32(val) = resultF32 {
      assert!((val - 4.0).abs() < f32::EPSILON);
    } else {
      panic!("Expected Value::F32");
    }

    let resultF64: Value = ffi!{
      let libm: Library = Library::load("libm.so.6")?;
      let args: Vec<Value> = vec![Value::F64(2.0), Value::F64(3.0)];
      Ok(libm.call("pow", args, Type::F64)?)
    }.expect("FFI F64 call failed");

    if let Value::F64(val) = resultF64 {
      assert!((val - 8.0).abs() < f64::EPSILON);
    } else {
      panic!("Expected Value::F64");
    }
  }

  /// Checks passing Bool and Usize types.
  #[test]
  fn boolAndUnsignedTypes() -> ()
  {
    setup();

    let resultBool: Value = ffi!{
      let libc: Library = Library::load("libc.so.6")?;
      let args: Vec<Value> = vec![Value::Bool(true)];
      Ok(libc.call("isalpha", args, Type::Bool)?)
    }.expect("FFI Bool call failed");

    if let Value::Bool(val) = resultBool {
      assert!(!val);
    } else {
      panic!("Expected Value::Bool");
    }

    let resultUsize: Value = ffi!{
      let libc: Library = Library::load("libc.so.6")?;
      let args: Vec<Value> = vec![Value::CString(b"test".to_vec()), Value::Usize(4)];
      Ok(libc.call("strnlen", args, Type::Usize)?)
    }.expect("FFI Usize call failed");

    if let Value::Usize(val) = resultUsize {
      assert_eq!(val, 4);
    } else {
      panic!("Expected Value::Usize");
    }
  }

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