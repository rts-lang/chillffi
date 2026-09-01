use std::ffi::CStr;
use std::ffi::CString;
use crate::ffi::errors::FFIError;
use serde::{Deserialize, Serialize};
// =================================================================================================

/// A value that can be passed between processes and used when calling FFI.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub(crate) enum Value
{
  /// Just an empty value.
  None,

  //
  U8(u8),
  U16(u16),
  U32(u32),
  U64(u64),
  Usize(usize),

  //
  I8(i8),
  I16(i16),
  I32(i32),
  I64(i64),
  Isize(isize),

  //
  F32(f32),
  F64(f64),

  //
  Bool(bool),

  /// Store the address as a regular number;
  ///
  /// The address is stored as usize rather than as a raw pointer — for two reasons:
  /// serialization through bincode/serde (raw pointers cannot do this)
  /// and the fact that the owner of the memory is the C code on the zygote clone side,
  /// not Rust.
  ///
  /// # Lifetime
  /// Valid strictly within the same ffi!{} block — that is, inside the same
  /// zygote clone. The clone dies when leaving the block
  /// (ZygoteGuard::drop → SIGKILL), and along with it dies the address
  /// space to which this address belonged.
  ///
  /// Using it outside the block is undefined behavior, not an Err:
  /// a new clone is forked from the same parent zygote and often has
  /// the same mapped address space, therefore at that address
  /// there may be unrelated memory instead of the expected crash.
  ///
  /// Ownership and deallocation (free/strdup and similar) are exclusively on
  /// the side of the calling C code.
  Pointer(usize),

  /// In C code, one would expect `uint8_t *data`;
  /// But without `len` these bytes are useless and `size_t len` is necessary.
  RawString(Vec<u8>),

  /// In C code, one would expect `const char *str`; `\0` terminated.
  CString(Vec<u8>),

  /// In C code, one would expect `const char *str, size_t len`.
  String(Vec<u8>),

  /// Represents a Rust closure passed to C as a function pointer.
  ///
  /// The `u64` holds the unique ID used to locate the JIT-compiled trampoline 
  /// inside the clone's callback registry.
  Function(u64),

  /// An ordered list of fields (not named).
  Struct(Vec<Value>)
}

// =================================================================================================

/// Description of the value type 
/// for defining FFI arguments and result.
///
/// todo: add unit tests for Type variants verification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Type
{
  /// Just an empty value.
  None,

  //
  U8,
  U16,
  U32,
  U64,
  Usize,

  //
  I8,
  I16,
  I32,
  I64,
  Isize,

  //
  F32,
  F64,

  //
  Bool,

  /// Raw pointer.
  Pointer,

  /// An ordered list of fields (not named).
  Struct(Vec<Type>)
}

// =================================================================================================

/// Wrapper for a raw memory address.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Pointer(pub usize);

/// Bridges a concrete Rust primitive to its [`Value`]/[`Type`] tag.
pub trait Primitive: Sized
{
  const TypeTag: Type;

  /// Converts a dynamic [`Value`] into a concrete primitive type.
  fn fromValue(value: Value) -> Result<Self, FFIError>;
  /// Converts this primitive into a dynamic [`Value`].
  fn toValue(self) -> Value;
}

/// Declares a binding between a primitive and a [`Value`] type.
macro_rules! implFFIPrimitive
{
  ($rustType:ty, $variant:ident) =>
  {
    impl Primitive for $rustType
    {
      const TypeTag: Type = Type::$variant;

      /// Parses the specific [`Value`] variant into this primitive type.
      fn fromValue(value: Value) -> Result<Self, FFIError>
      {
        match value {
          Value::$variant(v) => Ok(v),
          _ => Err(FFIError::Other(format!("expected {}, got {:?}", stringify!($variant), value))),
        }
      }

      /// Wraps this primitive value into its corresponding [`Value`] enum variant.
      fn toValue(self) -> Value { Value::$variant(self) }
    }
    
    impl From<$rustType> for Value
    {
      /// Converts the raw primitive into a dynamic [`Value`].
      fn from(v: $rustType) -> Self { Value::$variant(v) }
    }
  };
}

// =================================================================================================

// Declaration of all primitive types
implFFIPrimitive!(u8, U8);
implFFIPrimitive!(u16, U16);
implFFIPrimitive!(u32, U32);
implFFIPrimitive!(u64, U64);
implFFIPrimitive!(usize, Usize);
implFFIPrimitive!(i8, I8);
implFFIPrimitive!(i16, I16);
implFFIPrimitive!(i32, I32);
implFFIPrimitive!(i64, I64);
implFFIPrimitive!(isize, Isize);
implFFIPrimitive!(f32, F32);
implFFIPrimitive!(f64, F64);
implFFIPrimitive!(bool, Bool);

// =================================================================================================

mod private
{
  pub trait Sealed
  {
    fn intoFfiValue(self) -> crate::ffi::value::Value;
  }
}

/// Sealed marker trait for types that can be passed into FFI calls.
/// Prevents external users from constructing or using `Value` directly.
pub trait FfiArg: private::Sealed {}

impl<T: private::Sealed> FfiArg for T {}

macro_rules! implSealedArg
{
  ($type:ty) =>
  {
    impl private::Sealed for $type
    {
      fn intoFfiValue(self) -> Value { Value::from(self) }
    }
  };
}

// Implementing conversions for all supported FFI argument types
implSealedArg!(u8);
implSealedArg!(u16);
implSealedArg!(u32);
implSealedArg!(u64);
implSealedArg!(usize);
implSealedArg!(i8);
implSealedArg!(i16);
implSealedArg!(i32);
implSealedArg!(i64);
implSealedArg!(isize);
implSealedArg!(f32);
implSealedArg!(f64);
implSealedArg!(bool);
implSealedArg!(Pointer);
implSealedArg!(String);
implSealedArg!(&str);
implSealedArg!(CString);
implSealedArg!(&CStr);
implSealedArg!(Vec<u8>);
implSealedArg!(&[u8]);

// =================================================================================================

impl Primitive for Pointer
{
  const TypeTag: Type = Type::Pointer;

  /// Extracts the address from a [`Value::Pointer`].
  fn fromValue(value: Value) -> Result<Self, FFIError>
  {
    match value
    {
      Value::Pointer(addr) => Ok(Self(addr)),
      _ => Err(FFIError::Other(format!("expected Pointer, got {:?}", value))),
    }
  }

  /// Converts this [`Pointer`] wrapper into a [`Value::Pointer`].
  fn toValue(self) -> Value { Value::Pointer(self.0) }
}

impl Primitive for ()
{
  const TypeTag: Type = Type::None;

  /// Validates and converts a [`Value::None`] into a Rust unit type `()`.
  fn fromValue(value: Value) -> Result<Self, FFIError>
  {
    match value {
      Value::None => Ok(()),
      _ => Err(FFIError::Other(format!("expected None, got {:?}", value))),
    }
  }

  /// Converts a unit type `()` into a [`Value::None`].
  fn toValue(self) -> Value { Value::None }
}

impl From<Pointer> for usize
{
  /// Extracts the underlying `usize` memory address from a [`Pointer`].
  fn from(p: Pointer) -> Self { p.0 }
}

impl From<Pointer> for Value
{
  /// Converts a [`Pointer`] directly into a [`Value::Pointer`] variant.
  fn from(p: Pointer) -> Self { Self::Pointer(p.0) }
}

// =================================================================================================

impl From<String> for Value
{
  /// Converts a Rust [`String`] into [`Value::String`] (ptr + len).
  fn from(s: String) -> Self { Value::String(s.into_bytes()) }
}

impl From<&str> for Value
{
  /// Converts a string slice `&str` into [`Value::String`] (ptr + len).
  fn from(s: &str) -> Self { Value::String(s.as_bytes().to_vec()) }
}

impl From<CString> for Value
{
  /// Converts a [`CString`] into [`Value::CString`].
  fn from(c: CString) -> Self { Value::CString(c.into_bytes()) }
}

impl From<&CStr> for Value
{
  /// Converts a C-string literal (`c"hello"`) into [`Value::CString`].
  fn from(c: &CStr) -> Self { Value::CString(c.to_bytes().to_vec()) }
}

impl From<Vec<u8>> for Value
{
  /// Converts raw bytes `Vec<u8>` into [`Value::RawString`].
  fn from(v: Vec<u8>) -> Self { Value::RawString(v) }
}

impl From<&[u8]> for Value
{
  /// Converts a byte slice `&[u8]` into [`Value::RawString`].
  fn from(v: &[u8]) -> Self { Value::RawString(v.to_vec()) }
}

impl TryFrom<Value> for String
{
  type Error = FFIError;

  /// Attempts to convert a string [`Value`] into a Rust [`String`].
  fn try_from(value: Value) -> Result<Self, Self::Error>
  {
    let bytes: Vec<u8> = extractStringBytes(value)?;
    String::from_utf8(bytes)
      .map_err(|e| FFIError::Other(format!("not valid UTF-8: {e}")))
  }
}

impl TryFrom<Value> for CString
{
  type Error = FFIError;

  /// Attempts to convert a string [`Value`] into a [`CString`].
  fn try_from(value: Value) -> Result<Self, Self::Error>
  {
    let bytes: Vec<u8> = extractStringBytes(value)?;
    CString::new(bytes)
      .map_err(|e| FFIError::Other(format!("interior NUL byte: {e}")))
  }
}

impl TryFrom<Value> for Vec<u8>
{
  type Error = FFIError;

  /// Extracts raw bytes from a string [`Value`].
  fn try_from(value: Value) -> Result<Self, Self::Error>
  {
    extractStringBytes(value)
  }
}

/// Helper function to extract raw byte vector from any string [`Value`] variant.
fn extractStringBytes(value: Value) -> Result<Vec<u8>, FFIError>
{
  match value
  {
    Value::String(b) | Value::CString(b) | Value::RawString(b) => Ok(b),
    _ => Err(FFIError::Other(format!("expected a string Value, got {:?}", value))),
  }
}

// =================================================================================================

#[cfg(test)]
mod tests
{
  use crate::ffi;
  use std::ffi::CString;
  use crate::ffi::value::{Pointer, Value};
  // ===============================================================================================

  /// Checks all signed integer types 
  ///
  /// [`Value::I8`], [`Value::I16`], [`Value::I32`], [`Value::I64`], [`Value::Isize`]
  #[test]
  fn signedIntegers() -> ()
  {
    ffi!(|scope| {
      let libc: Library = scope.load("libc.so.6")?;
      
      let resI8: i8 = libc.call("abs").arg::<i8>(-5).result()?;
      assert!(matches!(resI8, 5));
      
      let resI16: i16 = libc.call("abs").arg::<i16>(-15).result()?;
      assert!(matches!(resI16, 15));
      
      let resI32: i32 = libc.call("abs").arg::<i32>(-42).result()?;
      assert!(matches!(resI32, 42));
      
      let resI64: i64 = libc.call("labs").arg::<i64>(-100000).result()?;
      assert!(matches!(resI64, 100000));
      
      let resIsize: isize = libc.call("labs").arg::<isize>(-500).result()?;
      assert!(matches!(resIsize, 500));

      Ok(())
    }).expect("Signed integers test failed");
  }

  /// Checks all unsigned integer types
  ///
  /// [`Value::U8`], [`Value::U16`], [`Value::U32`], [`Value::U64`], [`Value::Usize`].
  #[test]
  fn unsignedIntegers() -> ()
  {
    ffi!(|scope| {
      let libc: Library = scope.load("libc.so.6")?;
      
      let resU8: u8 = 
        libc.call("strnlen")
          .arg(c"a")
          .arg::<u8>(10)
          .result()?;
      assert!(matches!(resU8, 1));
      
      let resU16: u16 = 
        libc.call("strnlen")
        .arg(c"ab")
        .arg::<u16>(10)
        .result()?;
      assert!(matches!(resU16, 2));
      
      let resU32: u32 = 
        libc.call("strnlen")
        .arg(c"abc")
        .arg::<u32>(10)
        .result()?;
      assert!(matches!(resU32, 3));
      
      let resU64: u64 = 
        libc.call("strnlen")
        .arg(c"abcd")
        .arg::<u64>(10)
        .result()?;
      assert!(matches!(resU64, 4));
      
      let resUsize: usize = 
        libc.call("strnlen")
        .arg(c"abcde")
        .arg::<usize>(10)
        .result()?;
      assert!(matches!(resUsize, 5));

      Ok(())
    }).expect("Unsigned integers test failed");
  }

  /// Checks passing floating point numbers
  ///
  /// [`Value::F32`], [`Value::F64`].
  #[test]
  fn float() -> ()
  {
    let resultF32: f32 = ffi!(|scope| {
      let libm: Library = scope.load("libm.so.6")?;
      Ok( libm.call("sqrtf").arg::<f32>(16.0).result()? )
    }).expect("FFI F32 call failed");

    assert!((resultF32 - 4.0).abs() < f32::EPSILON);

    let resultF64: f64 = ffi!(|scope| {
      let libm: Library = scope.load("libm.so.6")?;
      Ok(
        libm.call("pow")
          .arg::<f64>(2.0)
          .arg::<f64>(3.0)
          .result()?
      )
    }).expect("FFI F64 call failed");

    assert!((resultF64 - 8.0).abs() < f64::EPSILON);
  }

  // ===============================================================================================

  /// Checks passing [`Value::Bool`].
  #[test]
  fn bool() -> ()
  {
    let result: bool = ffi!(|scope| {
      let libc: Library = scope.load("libc.so.6")?;
      Ok( libc.call("isalpha").arg(true).result()? )
    }).expect("FFI Bool call failed");

    assert!(!result);
  }

  // ===============================================================================================

  /// Checks pointer type handling: passing a valid pointer 
  /// and receiving NULL for a missing variable.
  #[test]
  fn pointer() -> ()
  {
    let result: Pointer = ffi!(|scope| {
      let libc: Library = scope.load("libc.so.6")?;
      Ok(
        libc.call("getenv")
          .arg(c"noSuchVar")
          .result()?
      )
    }).expect("FFI pointer call failed");

    assert_eq!(result, Pointer(0));
  }

  /// Checks that a pointer returned inside one ffi!{} block stays valid for reuse as an argument
  /// within the same block (same clone process, same address space).
  ///
  /// Env vars set in the host are invisible to the clone — its environ was captured at zygote
  /// startup (before main()). strdup() sidesteps this: it allocates directly in the clone's own
  /// heap, so the round-trip is verified without relying on inherited process state.
  #[test]
  fn pointerRoundtrip() -> ()
  {
    let len: usize = ffi!(|scope| {
      let libc: Library = scope.load("libc.so.6")?;
      let ptr: Pointer = 
        libc.call("strdup")
          .arg(c"hello")
          .result()?;
      assert_ne!(ptr, Pointer(0));
  
      let result: usize = libc.call("strlen").arg(ptr).result()?;
      libc.call("free").arg(ptr).void()?;
      Ok(result)
    }).expect("pointer roundtrip failed");

    assert!(matches!(len, 5));
  }

  // ===============================================================================================

  /// Checks string conversion bridges and `TryFrom` validation.
  #[test]
  fn stringBridges() -> ()
  {
    let valString: Value = String::from("hello").into();
    assert!(matches!(valString, Value::String(_)));

    let valStr: Value = "hello".into();
    assert_eq!(valStr, Value::String(b"hello".to_vec()));

    let valCString: Value = CString::new("hello").unwrap().into();
    assert_eq!(valCString, Value::CString(b"hello".to_vec()));

    let valRawBytes: Value = vec![0u8, 1, 2].into();
    assert_eq!(valRawBytes, Value::RawString(vec![0, 1, 2]));

    // Reverse conversions
    let resString: String = Value::RawString(b"hello".to_vec()).try_into().unwrap();
    assert_eq!(resString, "hello");

    assert!(CString::try_from(Value::RawString(b"with\0nul".to_vec())).is_err());
  }

  /// Checks passing CString to a C function expecting a \0-terminated string (strlen).
  #[test]
  fn cString() -> ()
  {
    let result: usize = ffi!(|scope| {
      let libc: Library = scope.load("libc.so.6")?;
      Ok(
        libc.call("strlen")
          .arg(c"hello")
          .result()?
      )
    }).expect("FFI CString call failed");

    assert_eq!(result, 5);
  }

  /// Checks passing String which automatically expands to two C-ABI arguments (ptr + len)
  /// for functions accepting buffer pointer and max length (strnlen).
  #[test]
  fn string() -> ()
  {
    let result: usize = ffi!(|scope| {
      let libc: Library = scope.load("libc.so.6")?;
      Ok(
        libc.call("strnlen")
          .arg("hello world")
          .result()?
      )
    }).expect("FFI String call failed");

    assert_eq!(result, 11);
  }

  /// Checks passing RawString as a single raw byte pointer (atoi).
  #[test]
  fn rawString() -> ()
  {
    let result: i32 = ffi!(|scope| {
      let libc: Library = scope.load("libc.so.6")?;
      Ok(
        libc.call("atoi")
          .arg(b"12345\0".to_vec())
          .result()?
      )
    }).expect("FFI RawString call failed");

    assert_eq!(result, 12345);
  }

  // ===============================================================================================
}

// =================================================================================================