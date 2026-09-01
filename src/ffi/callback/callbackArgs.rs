use crate::ffi::types::primitive::Primitive;
use crate::ffi::errors::FFIError;
use crate::ffi::types::Value;
// =================================================================================================

/// Динамические аргументы, переданные в callback.
/// Позволяет извлекать аргументы по индексу с автоматическим приведением к Rust-типу.
pub struct CallbackArgs 
{
  /// todo desc
  pub(crate) values: Vec<Value>,
}

impl CallbackArgs 
{
  /// todo desc
  pub fn get<T: Primitive>(&self, index: usize) -> Result<T, FFIError> 
  {
    self.values
      .get(index)
      .ok_or_else(|| FFIError::Other(format!("arg index {} out of bounds", index)))
      .and_then(|v| T::fromValue(v.clone()))
  }

  /// todo desc
  pub fn len(&self) -> usize 
  {
    self.values.len()
  }
}

// =================================================================================================

impl From<Vec<Value>> for CallbackArgs 
{
  fn from(values: Vec<Value>) -> Self 
  {
    Self { values }
  }
}

/// Трейт для прозрачного преобразования результата closure в Value.
pub trait IntoValue 
{
  fn intoValue(self) -> Value;
}

impl<T: Primitive> IntoValue for T 
{
  fn intoValue(self) -> Value 
  {
    T::toValue(self)
  }
}

impl IntoValue for Value 
{
  /// Позволяет возвращать Value напрямую, если требуется полная динамика
  fn intoValue(self) -> Value 
  {
    self
  }
}

// =================================================================================================