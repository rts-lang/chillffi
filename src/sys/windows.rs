//! Windows implementation of the [`crate::sys`] layer.
//!
//! `kernel32` is already linked into every Rust binary on Windows, so the
//! handful of Win32 calls below are declared directly rather than pulling
//! in `windows-sys`.
// =================================================================================================
use crate::sys::ProcessId;
use std::ffi::c_void;
// =================================================================================================

type Handle = *mut c_void;

const ProcessTerminate: u32 = 0x0001;
const Synchronize: u32 = 0x0010_0000;
const Infinite: u32 = 0xFFFF_FFFF;
const SilentErrorMode: u32 = 0x0001 | 0x0002 | 0x8000; // SEM_FAILCRITICALERRORS | SEM_NOGPFAULTERRORBOX | SEM_NOOPENFILEERRORBOX
const ModuleHandleFromAddress: u32 = 0x0002 | 0x0004; // GET_MODULE_HANDLE_EX_FLAG_UNCHANGED_REFCOUNT | ..._FROM_ADDRESS

#[link(name = "kernel32")]
unsafe extern "system"
{
  fn OpenProcess(desiredAccess: u32, inheritHandle: i32, processId: u32) -> Handle;
  fn TerminateProcess(process: Handle, exitCode: u32) -> i32;
  fn WaitForSingleObject(handle: Handle, milliseconds: u32) -> u32;
  fn CloseHandle(object: Handle) -> i32;
  fn GetLastError() -> u32;
  fn SetErrorMode(mode: u32) -> u32;
  fn GetModuleHandleExW(flags: u32, moduleName: *const u16, module: *mut Handle) -> i32;
}

unsafe extern "C"
{
  fn _errno() -> *mut i32;
}

// =================================================================================================

/// No-op: Windows has neither SIGCHLD nor zombie processes.
pub const fn ignoreChildExits() -> () {}

/// A clone is a crash domain, not a cooperating peer — kill it outright.
pub fn killProcess(pid: ProcessId) -> ()
{
  let process: Handle = unsafe{ OpenProcess(ProcessTerminate, 0, pid) };
  if process.is_null() { return; }

  unsafe{ TerminateProcess(process, 1) };
  unsafe{ CloseHandle(process) };
}

/// Blocks until the process terminates.
pub fn waitProcess(pid: ProcessId) -> ()
{
  let process: Handle = unsafe{ OpenProcess(Synchronize, 0, pid) };
  if process.is_null() { return; }

  unsafe{ WaitForSingleObject(process, Infinite) };
  unsafe{ CloseHandle(process) };
}

/// Disables Windows Error Reporting for the calling process. Called once at
/// clone startup — a crash must kill the clone immediately, not open a WER
/// dialog the Runtime would sit blocked waiting on.
pub fn silenceCrashReporting() -> ()
{
  unsafe{ SetErrorMode(SilentErrorMode) };
}

// =================================================================================================

/// Base load address of the module containing this function.
pub fn moduleBase() -> usize
{
  let mut module: Handle = std::ptr::null_mut();
  let found: i32 = unsafe{
    GetModuleHandleExW(
      ModuleHandleFromAddress,
      moduleBase as *const () as *const u16,
      &mut module
    )
  };
  if found == 0 { return 0; }

  module as usize
}

// =================================================================================================

/// Reads `errno` of the calling thread. Per-CRT-instance, not per-process:
/// a library statically linked against its own CRT keeps its own copy.
pub fn readErrno() -> i32
{
  unsafe{ *_errno() }
}

/// `GetLastError` of the calling thread, captured alongside [`readErrno`].
/// Most Win32 functions report failure here, not through `errno`.
pub fn readOsError() -> Option<u32>
{
  Some(unsafe{ GetLastError() })
}

// =================================================================================================

/// Every allocation, including plain [`allocate`], comes from
/// `_aligned_malloc`: Windows requires `_aligned_free` for those and `free`
/// for `malloc`'s, so one allocator for everything avoids tracking which is
/// which. A pointer from `Scope::alloc` must be released through chillffi,
/// not by C code calling `free()`.
pub fn allocate(length: usize) -> *mut c_void
{
  unsafe{ libc::aligned_malloc(length, crate::sys::MinAlignment * 2) }
}

/// `_aligned_malloc`. Argument order is reversed from `posix_memalign`'s,
/// and failure is a null return rather than a status code.
pub fn allocateAligned(length: usize, alignment: usize) -> Result<*mut c_void, String>
{
  let pointer: *mut c_void = unsafe{ libc::aligned_malloc(length, alignment) };
  if pointer.is_null()
  {
    return Err(format!(
      "_aligned_malloc failed for {} bytes at alignment {}", length, alignment
    ));
  }
  Ok(pointer)
}

/// `_aligned_free` — see [`allocate`].
pub fn deallocate(pointer: *mut c_void) -> ()
{
  unsafe{ libc::aligned_free(pointer) };
}

// =================================================================================================
