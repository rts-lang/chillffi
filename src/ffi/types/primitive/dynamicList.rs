use crate::ffi::errors::FFIError;
use crate::ffi::types::primitive::{Arg, FfiPrimitive};
use crate::ffi::types::Value;
// =================================================================================================

/// Dynamic list of FFI values.
pub struct DynamicList
{
  /// FFI Values.
  values: Box<[Value]>
}

impl DynamicList
{
  /// Creates a wrapper from a vector of values.
  ///
  /// (due to [`Value`] being used only within the crate)
  pub(crate) const fn fromValues(values: Box<[Value]>) -> Self
  {
    Self { values }
  }

  /// Returns the number of fields in the structure.
  pub const fn len(&self) -> usize
  {
    self.values.len()
  }

  /// Checks whether the structure is empty.
  pub const fn isEmpty(&self) -> bool
  {
    self.values.is_empty()
  }

  /// Extracts a field by index and converts it into the required type `T`.
  pub fn get<T: FfiPrimitive>(&self, index: usize) -> Result<T, FFIError>
  {
    self.values
      .get(index)
      .ok_or_else(|| FFIError::Other(format!("field index {} out of bounds", index)))
      .and_then(|v| T::fromFfiValue(Arg(v.clone())))
  }

  /// todo desc
  pub fn getStruct(&self, index: usize) -> Result<StructValue, FFIError>
  {
    match self.values.get(index) {
      Some(Value::Struct(values)) => Ok(StructValue::fromValues(values.clone())),
      Some(other) => Err(FFIError::Other(format!(
        "field {index}: expected Struct, got {other:?}"
      ))),
      None => Err(FFIError::Other(format!("field index {index} out of bounds"))),
    }
  }
}

// =================================================================================================

impl From<Box<[Value]>> for DynamicList
{
  /// Converts a vector of `Value`s into a [`DynamicList`].
  fn from(values: Box<[Value]>) -> Self
  {
    Self { values }
  }
}

// =================================================================================================

/// todo desc
#[derive(Debug, Clone)]
pub struct StructValue
{
  /// todo desc
  pub(crate) values: Box<[Value]>
}

impl StructValue
{
  pub fn new(fields: impl IntoIterator<Item = Arg>) -> Self
  {
    Self {
      values: fields.into_iter().map(|a| a.0).collect::<Vec<_>>().into_boxed_slice()
    }
  }

  /// todo desc
  pub(crate) const fn fromValues(values: Box<[Value]>) -> Self
  {
    Self { values }
  }

  /// todo desc
  pub const fn len(&self) -> usize
  {
    self.values.len()
  }
  
  /// todo desc
  pub const fn isEmpty(&self) -> bool
  {
    self.values.is_empty()
  }

  /// todo desc
  pub fn get<T: FfiPrimitive>(&self, index: usize) -> Result<T, FFIError>
  {
    self.values
      .get(index)
      .ok_or_else(|| FFIError::Other(format!("field index {} out of bounds", index)))
      .and_then(|v| T::fromFfiValue(Arg(v.clone())))
  }

  /// todo desc
  pub fn getStruct(&self, index: usize) -> Result<Self, FFIError>
  {
    match self.values.get(index) 
    {
      Some(Value::Struct(values)) => Ok(Self::fromValues(values.clone())),
      Some(other) => Err(FFIError::Other(format!(
        "field {index}: expected Struct, got {other:?}"
      ))),
      None => Err(FFIError::Other(format!("field index {index} out of bounds"))),
    }
  }

  /// todo desc
  pub fn intoList(self) -> DynamicList
  {
    DynamicList::fromValues(self.values)
  }
}

impl From<StructValue> for DynamicList
{
  /// todo desc
  fn from(s: StructValue) -> Self
  {
    Self::fromValues(s.values)
  }
}

impl From<DynamicList> for StructValue
{
  /// todo desc
  fn from(list: DynamicList) -> Self
  {
    Self { values: list.values }
  }
}

// =================================================================================================