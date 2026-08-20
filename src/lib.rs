#[cfg(not(target_os = "linux"))]
compile_error!("chillffi supports only Linux operating systems.");
// Currently available only on Linux, although it should work on UNIX in general.
// But I have not tested it on macOS.

// =================================================================================================

pub mod worker;
pub mod zygote;
pub mod ffi;

// =================================================================================================

use std::{env, io};
use crate::zygote::{initZygote, runAsZygote, ZygoteFlag};

// =================================================================================================

/// Single entry point for zygote initialization in any binary (including tests).
/// Checks whether the process is running as a zygote; if so — switches to daemon mode,
/// otherwise — initializes the parent side.
#[ctor::ctor(unsafe)]
fn zygoteEntrypoint() -> ()
{
  let mut args = env::args_os();
  args.next();
  if let Some(arg) = args.next() 
  {
    if arg == ZygoteFlag 
    {
      runAsZygote();
    }
  }
  //
}

/// todo desc
pub fn setupZygote() -> io::Result<()>
{
  initZygote()
}

// =================================================================================================

/// todo desc
/// 
/// Важно: Он заберет на себя Library - поэтому коду внутри, придется указывать
/// это иначе при совпадении этого типа данных. todo В целом, это можно исправить в будущем.
#[macro_export]
macro_rules! ffi 
{
  ($($body:tt)*) => 
  {
    (|| -> Result<_, $crate::ffi::library::FFIError> 
    {
      use $crate::ffi::library::__FFILibrary as Library;
      use $crate::zygote::ClonedZygote;
      use $crate::zygote::ZygoteGuard;

      // Создание клон-зиготу от основной
      let mut zygote: ClonedZygote = ClonedZygote::getMeClone()
        .map_err(|e| $crate::ffi::library::FFIError::Other(e.to_string()))?;
      
      // Регистрируем клон-зиготу в ZygoteStack текущего потока
      let _guard: ZygoteGuard = ZygoteGuard::enter(zygote);

      // Выполнение тела
      $($body)*
    })()
  };
}

// =================================================================================================