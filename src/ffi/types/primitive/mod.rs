mod dynamicList;
mod pointer;
mod none;
mod strings;
// =================================================================================================
use crate::ffi::errors::FFIError;
use crate::ffi::types::{Value, Type};
// =================================================================================================

/// Wrapper for a raw memory address.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Pointer(pub usize);

// =================================================================================================

/// Динамическая список FFI значений.
pub struct DynamicList
{
  /// todo desc
  values: Vec<Value>
}

// =================================================================================================

/// todo desc
pub struct Callback(pub(crate) u64);

// =================================================================================================

/// Публичный заменитель [`Value`] для сигнатур публичных методов.
///
/// `Value` — `pub(crate)`, поэтому не может появляться в публичном API
/// (`Scope::callPointer` и т.п.): вызов из внешнего крейта падает с
/// E0603 «type is private». `Arg` оборачивает `Value`, строится из любого
/// `FfiArg`-типа через `From` (включая `Callback`), разворачивается только
/// внутри крейта.
#[derive(Debug, Clone)]
pub struct Arg(pub(crate) Value);

// =================================================================================================

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