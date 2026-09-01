use crate::ffi::types::primitive::DynamicList;
use crate::ffi::errors::FFIError;
use crate::ffi::types::primitive::Primitive;
use crate::ffi::types::Value;
// =================================================================================================

impl DynamicList 
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

impl From<Vec<Value>> for DynamicList
{
  /// todo desc
  fn from(values: Vec<Value>) -> Self
  {
    Self { values }
  }
}

pub trait IntoValue
{
  /// todo desd
  fn intoValue(self) -> Value;
}

impl<T: Primitive> IntoValue for T
{
  /// todo desc
  fn intoValue(self) -> Value
  {
    T::toValue(self)
  }
}

impl IntoValue for Value
{
  /// Позволяет возвращать Value напрямую, если требуется полная динамика.
  fn intoValue(self) -> Value
  {
    self
  }
}

// =================================================================================================