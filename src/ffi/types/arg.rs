use std::ffi::CStr;
use std::ffi::CString;
use crate::ffi::types::Value;
use crate::ffi::types::primitive::Pointer;
// =================================================================================================

mod private
{
  use crate::ffi::types::Value;

pub trait Sealed
  {
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
      fn intoFfiValue(self) -> Value { Value::from(self) }
    }
  };
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