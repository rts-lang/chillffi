use crate::ffi::errors::FFIError;
use crate::ffi::library::sendRawRequest;
use crate::ffi::value::Value;
use crate::zygote::FFIRequest;
// =================================================================================================

/// AllocatedMemory сама по себе нужна при выделении памяти со стороны Rust;
/// Это RAII-обёртка над памятью, выделенной в куче клона зиготы через `Library::alloc`;
/// Автоматически отправляет запрос `Free` при выходе из области видимости (`Drop`).
/// 
/// Важно: В `Library` есть свои методы для работы с памятью - 
/// они тоже нужны, но уже когда мы, не являемся создателями участка памяти.
/// 
/// Для работы с сырыми адресами, выделенными C-стороной (например, `strdup`),
/// используйте напрямую методы `Library` напрямую.
pub struct AllocatedMemory 
{
  /// todo desc
  address: usize,
  /// todo desc
  length: usize,
}

impl AllocatedMemory 
{
  /// todo desc
  pub(super) fn new(address: usize, length: usize) -> Self {
    Self { address, length }
  }

  /// todo desc
  pub fn address(&self) -> usize { self.address }
  /// todo desc
  pub fn length(&self) -> usize { self.length }

  /// todo desc
  pub fn asPointer(&self) -> Value {
    Value::Pointer(self.address)
  }

  /// todo desc
  pub fn read(&self) -> Result<Value, FFIError> {
    sendRawRequest(FFIRequest::ReadMemory { pointer: self.address, length: self.length })
  }

  /// todo desc
  pub fn write(&self, value: Value) -> Result<(), FFIError> {
    sendRawRequest(FFIRequest::WriteMemory { pointer: self.address, value })?;
    Ok(())
  }
}

impl Drop for AllocatedMemory 
{
  /// todo desc
  fn drop(&mut self) 
  {
    if self.address != 0 {
      let _ = sendRawRequest(FFIRequest::Free { pointer: self.address });
    }
  }
}

// =================================================================================================