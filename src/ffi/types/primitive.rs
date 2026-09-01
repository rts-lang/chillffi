use crate::ffi::types::Value;
use crate::ffi::errors::FFIError;
use crate::ffi::types::Type;
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

// Pointer

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

impl std::fmt::UpperHex for Pointer
{
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result
  {
    std::fmt::UpperHex::fmt(&self.0, f)
  }
}

// =================================================================================================

// None

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

// =================================================================================================

// DynamicStruct

/// Динамическая структура, прочитанная из памяти.
/// Поля доступны по индексу с автоматическим приведением к нужному Rust-типу.
pub struct DynamicStruct 
{
  /// todo desc
  values: Vec<Value>
}

impl DynamicStruct 
{
  /// Создаёт обёртку из вектора значений (используется внутри крейта).
  pub(crate) fn fromValues(values: Vec<Value>) -> Self 
  {
    Self { values }
  }

  /// Возвращает количество полей в структуре.
  pub fn len(&self) -> usize 
  {
    self.values.len()
  }

  /// Проверяет, пуста ли структура.
  pub fn is_empty(&self) -> bool 
  {
    self.values.is_empty()
  }

  /// Извлекает поле по индексу и преобразует его в требуемый тип `T`.
  pub fn get<T: Primitive>(&self, index: usize) -> Result<T, FFIError> 
  {
    self.values
      .get(index)
      .ok_or_else(|| FFIError::Other(format!("field index {} out of bounds", index)))
      .and_then(|v| T::fromValue(v.clone()))
  }
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