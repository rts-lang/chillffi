use std::cell::UnsafeCell;
use crate::ffi::allocatedMemory::AllocatedMemory;
use crate::ffi::errors::FFIError;
use crate::ffi::library::sendRawRequest;
use crate::ffi::value::Value;
use crate::zygote::FFIRequest;
// =================================================================================================

/// todo desc
struct HeavyStack
{
  // Выделенный стек или арена
}

/// Владелец HeavyStack. Заводится макросом ffi!{} один раз на блок (только если
/// пользователь запросил Scope), живёт и умирает строго с этим блоком.
/// Не публикуется напрямую — доступ только через Scope<'g>.
#[doc(hidden)]
pub struct ScopeGuard
{
  inner: UnsafeCell<Option<HeavyStack>>,
}

impl ScopeGuard
{
  #[doc(hidden)]
  pub fn new() -> Self
  {
    Self { inner: UnsafeCell::new(None) }
  }
}

/// Ручка на ScopeGuard текущего ffi!{}-блока — заимствует его на 'g.
/// Именно поэтому AllocatedMemory<'g> не может покинуть блок: ScopeGuard,
/// которого она заимствует, дропается на границе блока, и это проверяет компилятор.
pub struct Scope<'g>
{
  guard: &'g ScopeGuard,
}

impl<'g> Scope<'g>
{
  #[doc(hidden)]
  pub fn new(guard: &'g ScopeGuard) -> Self
  {
    Self { guard }
  }

  /// Allocates `length` bytes in the clone's heap.
  pub fn alloc(&self, length: usize) -> Result<AllocatedMemory<'g>, FFIError>
  {
    unsafe {
      let stack: &mut Option<HeavyStack> = &mut *self.guard.inner.get();

      // Инициализация тяжелого стека происходит ТОЛЬКО при первом вызове alloc()
      if stack.is_none() {
        *stack = Some(HeavyStack{});
      }
    }

    // Выделение памяти через зиготу
    match sendRawRequest(FFIRequest::Alloc { length })? {
      Value::Pointer(address) => Ok(AllocatedMemory::new(address, length)),
      _ => Err(FFIError::Other("Alloc did not return a pointer".to_string())),
    }
  }
}

// =================================================================================================