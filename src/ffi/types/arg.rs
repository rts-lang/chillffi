use crate::ffi::types::primitive::Arg;
use std::ffi::CStr;
use std::ffi::CString;
use crate::ffi::types::Value;
use crate::ffi::types::primitive::{Callback, Pointer};
// =================================================================================================

mod private
{
  use crate::ffi::types::Value;

  pub trait Sealed
  {
    // todo desc
    fn intoFfiValue(self) -> Value;
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
      // todo desc
      fn intoFfiValue(self) -> Value { Value::from(self) }
    }
    
    impl From<$type> for Arg
    {
      /// todo desc (переписать легче)
      /// Public handle so macro-generated code in foreign crates can build
      /// argument lists without naming `Value`.
      fn from(v: $type) -> Self { Self(Value::from(v)) }
    }
  };
}

impl From<Callback> for Arg
{
  /// todo desc
  /// Callback — ручной Sealed, поэтому и From вручную
  fn from(c: Callback) -> Self { Self(Value::Function(c.0)) }
}

// =================================================================================================

// Callback

impl private::Sealed for Callback
{
  /// todo desc
  fn intoFfiValue(self) -> Value { Value::Function(self.0) }
}

// =================================================================================================

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