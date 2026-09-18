//! Windows sys layer + named-pipe data channel for zygote clones.
//!
//! RtlCloneUserProcess does CoW cloning. ipc-channel cannot transfer
//! IpcSender/IpcReceiver handles across a cloned process (DuplicateHandle
//! / GetNamedPipeServerProcessId path breaks). Clone data IPC therefore
//! uses plain named pipes addressed by name — no handle passing.
// =================================================================================================
use crate::sys::ProcessId;
use std::ffi::c_void;
use std::ptr;
// =================================================================================================

pub type Handle = *mut c_void;

const ProcessTerminate: u32 = 0x0001;
const Synchronize: u32 = 0x0010_0000;
const Infinite: u32 = 0xFFFF_FFFF;
const SilentErrorMode: u32 = 0x0001 | 0x0002 | 0x8000;
const ModuleHandleFromAddress: u32 = 0x0002 | 0x0004;

const PIPE_ACCESS_DUPLEX: u32 = 0x00000003;
const PIPE_TYPE_BYTE: u32 = 0x00000000;
const PIPE_WAIT: u32 = 0x00000000;
const PIPE_READMODE_BYTE: u32 = 0x00000000;
const GENERIC_READ: u32 = 0x80000000;
const GENERIC_WRITE: u32 = 0x40000000;
const OPEN_EXISTING: u32 = 3;
const FILE_ATTRIBUTE_NORMAL: u32 = 0x80;

pub const STATUS_PROCESS_CLONED: i32 = 0x00000129;

#[repr(C)]
pub struct CLIENT_ID {
  pub UniqueProcess: *mut c_void,
  pub UniqueThread: *mut c_void,
}

#[repr(C)]
pub struct SECTION_IMAGE_INFORMATION {
  pub TransferAddress: *mut c_void,
  pub ZeroBits: usize,
  pub MaximumStackSize: usize,
  pub CommittedStackSize: usize,
  pub SubSystemType: u32,
  pub SubSystemVersion: u32,
  pub GpValue: u32,
  pub ImageCharacteristics: u16,
  pub DllCharacteristics: u16,
  pub Machine: u16,
  pub ImageContainsCode: u8,
  pub ImageFlags: u8,
  pub LoaderFlags: u32,
  pub ImageFileSize: u32,
  pub CheckSum: u32,
}

#[repr(C)]
pub struct RTL_USER_PROCESS_INFORMATION {
  pub Length: u32,
  pub ProcessHandle: Handle,
  pub ThreadHandle: Handle,
  pub ClientId: CLIENT_ID,
  pub ImageInformation: SECTION_IMAGE_INFORMATION,
}

// SYMBOL_INFO (dbghelp). SizeOfStruct must be set to the size of the struct
// without the trailing `Name` array — 88 bytes on x64. We allocate a
// 2000-byte Name buffer to be safe regardless of decoration length.
#[repr(C)]
struct SYMBOL_INFO {
  SizeOfStruct: u32,
  TypeIndex: u32,
  Reserved: [u64; 2],
  Index: u32,
  Size: u32,
  ModBase: u64,
  Flags: u32,
  Value: u64,
  Address: u64,
  Register: u32,
  Scope: u32,
  Tag: u32,
  NameLen: u32,
  MaxNameLen: u32,
  Name: [i8; 2000],
}

#[link(name = "kernel32")]
unsafe extern "system" {
  fn OpenProcess(desiredAccess: u32, inheritHandle: i32, processId: u32) -> Handle;
  fn TerminateProcess(process: Handle, exitCode: u32) -> i32;
  fn WaitForSingleObject(handle: Handle, milliseconds: u32) -> u32;
  fn CloseHandle(object: Handle) -> i32;
  fn GetLastError() -> u32;
  fn SetErrorMode(mode: u32) -> u32;
  fn GetModuleHandleExW(flags: u32, moduleName: *const u16, module: *mut Handle) -> i32;
  fn FreeConsole() -> i32;
  fn AttachConsole(dwProcessId: u32) -> i32;
  fn GetCurrentProcessId() -> u32;
  fn CreateNamedPipeW(
    lpName: *const u16,
    dwOpenMode: u32,
    dwPipeMode: u32,
    nMaxInstances: u32,
    nOutBufferSize: u32,
    nInBufferSize: u32,
    nDefaultTimeOut: u32,
    lpSecurityAttributes: *mut c_void,
  ) -> Handle;
  fn ConnectNamedPipe(hNamedPipe: Handle, lpOverlapped: *mut c_void) -> i32;
  fn CreateFileW(
    lpFileName: *const u16,
    dwDesiredAccess: u32,
    dwShareMode: u32,
    lpSecurityAttributes: *mut c_void,
    dwCreationDisposition: u32,
    dwFlagsAndAttributes: u32,
    hTemplateFile: Handle,
  ) -> Handle;
  fn ReadFile(
    hFile: Handle,
    lpBuffer: *mut u8,
    nNumberOfBytesToRead: u32,
    lpNumberOfBytesRead: *mut u32,
    lpOverlapped: *mut c_void,
  ) -> i32;
  fn WriteFile(
    hFile: Handle,
    lpBuffer: *const u8,
    nNumberOfBytesToWrite: u32,
    lpNumberOfBytesWritten: *mut u32,
    lpOverlapped: *mut c_void,
  ) -> i32;
  fn SetNamedPipeHandleState(
    hNamedPipe: Handle,
    lpMode: *mut u32,
    lpMaxCollectionCount: *mut u32,
    lpCollectDataTimeout: *mut u32,
  ) -> i32;
  fn GetCurrentProcess() -> Handle;
}

#[link(name = "dbghelp")]
unsafe extern "system" {
  fn SymInitializeW(hProcess: Handle, userSearchPath: *const u16, fInvadeProcess: i32) -> i32;
  fn SymFromName(hProcess: Handle, name: *const i8, symbol: *mut SYMBOL_INFO) -> i32;
  fn SymCleanup(hProcess: Handle) -> i32;
}

#[link(name = "ntdll")]
unsafe extern "system" {
  fn RtlCloneUserProcess(
    ProcessFlags: u32,
    ProcessSecurityDescriptor: *mut c_void,
    ThreadSecurityDescriptor: *mut c_void,
    DebugPort: Handle,
    ProcessInformation: *mut RTL_USER_PROCESS_INFORMATION,
  ) -> i32;
  // Undocumented. No-op if CsrPortHandle is non-NULL — that is exactly the
  // situation in a RtlCloneUserProcess clone, where the handle value (and the
  // CSR_PROCESS state on the csrss.exe side) is inherited stale from parent.
  // Signature reconstructed from ReactOS CsrClientConnectToServer; verify
  // against a real LdrpInitializeProcess disassembly if status < 0.
  fn CsrClientConnectToServer(
    ObjectDirectory: *const u16,
    ServerId: u32,
    ConnectionInfo: *mut c_void,
    ConnectionInfoLength: *mut u32,
    CalledFromServer: *mut u8,
  ) -> i32;
}

unsafe extern "C" {
  fn _errno() -> *mut i32;
}

// =================================================================================================

pub const fn ignoreChildExits() -> () {}

pub fn killProcess(pid: ProcessId) -> ()
{
  let process: Handle = unsafe { OpenProcess(ProcessTerminate, 0, pid) };
  if process.is_null() {
    return;
  }
  unsafe { TerminateProcess(process, 1) };
  unsafe { CloseHandle(process) };
}

pub fn waitProcess(pid: ProcessId) -> ()
{
  let process: Handle = unsafe { OpenProcess(Synchronize, 0, pid) };
  if process.is_null() {
    return;
  }
  unsafe { WaitForSingleObject(process, Infinite) };
  unsafe { CloseHandle(process) };
}

pub fn silenceCrashReporting() -> ()
{
  unsafe { SetErrorMode(SilentErrorMode) };
}

pub fn reattachConsole() -> ()
{
  unsafe {
    FreeConsole();
    AttachConsole(0xFFFF_FFFF);
  }
}

pub fn currentProcessId() -> u32
{
  unsafe { GetCurrentProcessId() }
}

pub fn closeHandle(h: Handle) -> ()
{
  if !h.is_null() && h as isize != -1 {
    unsafe { CloseHandle(h) };
  }
}

// =================================================================================================

pub struct CloneResult {
  pub pid: ProcessId,
  pub process_handle: Handle,
  pub thread_handle: Handle,
}

pub fn cloneProcess() -> Result<CloneResult, i32>
{
  let mut info: RTL_USER_PROCESS_INFORMATION = unsafe { std::mem::zeroed() };
  info.Length = std::mem::size_of::<RTL_USER_PROCESS_INFORMATION>() as u32;

  // No INHERIT_HANDLES — keeps Runtime↔Zygote control pipes intact.
  // No CREATE_SUSPENDED — some Win32/CSRSS init paths (filesystem APIs like
  // _stat64) are incomplete when the clone starts suspended and is resumed
  // later; ERROR_BROKEN_PIPE (109) on the data pipe was the symptom.
  let flags = 0u32;

  let status = unsafe {
    RtlCloneUserProcess(
      flags,
      ptr::null_mut(),
      ptr::null_mut(),
      ptr::null_mut(),
      &mut info,
    )
  };

  if status == STATUS_PROCESS_CLONED {
    return Err(STATUS_PROCESS_CLONED);
  }
  if status < 0 {
    return Err(status);
  }

  Ok(CloneResult {
    pid: info.ClientId.UniqueProcess as u32,
    process_handle: info.ProcessHandle,
    thread_handle: info.ThreadHandle,
  })
}


pub fn closeCloneHandles(result: &CloneResult) -> ()
{
  closeHandle(result.process_handle);
  closeHandle(result.thread_handle);
}

// =================================================================================================
// Named-pipe framed channel (length-prefixed messages). No handle passing.
// =================================================================================================

fn toWide(s: &str) -> Vec<u16>
{
  s.encode_utf16().chain(std::iter::once(0)).collect()
}

/// Unique pipe path for clone data IPC (one duplex pipe per clone).
pub fn cloneDataPipeName(clonePid: u32) -> String
{
  format!(r"\\.\pipe\chillffi-data-{}", clonePid)
}

/// Child side: create a duplex named-pipe server (does not wait for client yet).
pub fn createPipeServer(name: &str) -> Option<Handle>
{
  let wide = toWide(name);
  let h = unsafe {
    CreateNamedPipeW(
      wide.as_ptr(),
      PIPE_ACCESS_DUPLEX,
      PIPE_TYPE_BYTE | PIPE_READMODE_BYTE | PIPE_WAIT,
      1,
      64 * 1024,
      64 * 1024,
      5000,
      ptr::null_mut(),
    )
  };
  if h.is_null() || h as isize == -1 {
    return None;
  }
  Some(h)
}

/// Block until a client connects to a pipe created with [`createPipeServer`].
pub fn acceptPipeClient(h: Handle) -> bool
{
  let ok = unsafe { ConnectNamedPipe(h, ptr::null_mut()) };
  if ok == 0 {
    let err = unsafe { GetLastError() };
    // ERROR_PIPE_CONNECTED == 535
    return err == 535;
  }
  true
}

/// Parent/Runtime side: connect to an existing named-pipe server.
pub fn connectPipeClient(name: &str) -> Option<Handle>
{
  let wide = toWide(name);
  // Retry a few times — child may still be creating the server.
  for _ in 0..50 {
    let h = unsafe {
      CreateFileW(
        wide.as_ptr(),
        GENERIC_READ | GENERIC_WRITE,
        0,
        ptr::null_mut(),
        OPEN_EXISTING,
        FILE_ATTRIBUTE_NORMAL,
        ptr::null_mut(),
      )
    };
    if !h.is_null() && h as isize != -1 {
      // Ensure byte mode on the client end (matches server PIPE_READMODE_BYTE).
      let mut mode: u32 = PIPE_READMODE_BYTE;
      unsafe {
        SetNamedPipeHandleState(h, &mut mode, ptr::null_mut(), ptr::null_mut());
      }
      return Some(h);
    }
    std::thread::sleep(std::time::Duration::from_millis(10));
  }
  None
}

fn writeAll(h: Handle, buf: &[u8]) -> bool
{
  // Empty payload is valid (e.g. length prefix of a zero-byte body).
  if buf.is_empty() {
    return true;
  }
  let mut off = 0;
  while off < buf.len() {
    let mut written: u32 = 0;
    let ok = unsafe {
      WriteFile(
        h,
        buf[off..].as_ptr(),
        (buf.len() - off) as u32,
        &mut written,
        ptr::null_mut(),
      )
    };
    if ok == 0 || written == 0 {
      return false;
    }
    off += written as usize;
  }
  // Do NOT FlushFileBuffers on named pipes: it can block until the peer
  // reads, and with request/response on two pipes that risks deadlock.
  true
}

fn readExact(h: Handle, buf: &mut [u8]) -> bool
{
  let mut off = 0;
  while off < buf.len() {
    let mut read: u32 = 0;
    let ok = unsafe {
      ReadFile(
        h,
        buf[off..].as_mut_ptr(),
        (buf.len() - off) as u32,
        &mut read,
        ptr::null_mut(),
      )
    };
    if ok == 0 || read == 0 {
      return false;
    }
    off += read as usize;
  }
  true
}

/// Send a length-prefixed payload (u32 LE length + bytes) in **one** WriteFile
/// sequence so the peer never observes a torn frame.
pub fn pipeSend(h: Handle, payload: &[u8]) -> bool
{
  if payload.len() > u32::MAX as usize {
    return false;
  }
  let mut msg = Vec::with_capacity(4 + payload.len());
  msg.extend_from_slice(&(payload.len() as u32).to_le_bytes());
  msg.extend_from_slice(payload);
  writeAll(h, &msg)
}

/// Receive a length-prefixed payload.
/// On failure returns None; call [`lastPipeError`] for the Win32 code.
pub fn pipeRecv(h: Handle) -> Option<Vec<u8>>
{
  let mut lenBuf = [0u8; 4];
  if !readExact(h, &mut lenBuf) {
    return None;
  }
  let len = u32::from_le_bytes(lenBuf) as usize;
  // Sanity cap: 16 MiB
  if len > 16 * 1024 * 1024 {
    return None;
  }
  let mut buf = vec![0u8; len];
  if len > 0 && !readExact(h, &mut buf) {
    return None;
  }
  Some(buf)
}

/// Last `GetLastError` after a failed pipe op (best-effort).
pub fn lastPipeError() -> u32
{
  unsafe { GetLastError() }
}

// =================================================================================================
// CSRSS reconnect (undocumented). CsrPortHandle survives a CoW clone as a
// stale value — the lazy-connect guard inside CsrClientConnectToServer
// treats the port as live and does not re-establish the connection. As a
// result, Win32 calls that route through CSR / basesrv (e.g. _stat64,
// activation-context APIs, parts of the loader) AV or abort, surfacing as
// ERROR_BROKEN_PIPE (109) on the data pipe. ucrt-only paths (math, string)
// keep working because they never touch CSR.
//
// Strategy:
//   1. In the healthy main zygote, BEFORE the first clone, resolve the
//      address of `ntdll!CsrPortHandle` via dbghelp / symbol server.
//      The address is cached in a static OnceLock; children inherit it
//      through CoW for free.
//   2. In a freshly cloned child, NULL out the stale handle and call
//      CsrClientConnectToServer again, so ntdll re-establishes a real
//      connection to csrss.exe for this PID.
// =================================================================================================

static CsrPortHandleAddress: std::sync::OnceLock<usize> = std::sync::OnceLock::new();

/// Resolve `ntdll!CsrPortHandle` once, in the healthy main zygote, before
/// any clone is spawned. Safe to call multiple times — the second call is a
/// no-op. Stores 0 on failure (no network / no symbols); `reconnectCsr`
/// then returns `false` and the clone falls back to the broken-port path.
pub fn resolveCsrPortHandle() -> ()
{
  CsrPortHandleAddress.get_or_init(|| unsafe {
    let process: Handle = GetCurrentProcess();

    // If _NT_SYMBOL_PATH is already set, let dbghelp read it; otherwise
    // build a default cache + msdl path so first-run CI also works.
    let searchPath: Option<Vec<u16>> = if std::env::var("_NT_SYMBOL_PATH").is_ok() {
      None
    } else {
      let cache = std::env::temp_dir().join("chillffi-symbols");
      Some(
        format!(
          "srv*{}*https://msdl.microsoft.com/download/symbols\0",
          cache.display()
        )
          .encode_utf16()
          .collect(),
      )
    };
    let searchPathPtr = searchPath.as_ref().map_or(std::ptr::null(), |v| v.as_ptr());

    if SymInitializeW(process, searchPathPtr, 1) == 0 {
      return 0;
    }

    let mut info: SYMBOL_INFO = std::mem::zeroed();
    info.SizeOfStruct = 88; // sizeof(SYMBOL_INFO) without Name, x64
    info.MaxNameLen = 2000;

    let name = b"ntdll!CsrPortHandle\0";
    let found = SymFromName(process, name.as_ptr() as *const i8, &mut info) != 0;
    SymCleanup(process);

    if found { info.Address as usize } else { 0 }
  });
}

/// Child-side: NULL the stale CsrPortHandle and re-run CsrClientConnectToServer
/// against `\Windows` so ntdll opens a fresh ALPC connection to csrss.exe for
/// this cloned PID.
///
/// Returns `true` on success, `false` if the address was never resolved
/// (resolveCsrPortHandle failed in the main zygote) or the reconnect call
/// returned a negative NTSTATUS.
pub fn reconnectCsr() -> bool
{
  let address = match CsrPortHandleAddress.get() {
    Some(&a) if a != 0 => a,
    _ => return false,
  };

  unsafe {
    *(address as *mut Handle) = std::ptr::null_mut();

    let objectDirectory: Vec<u16> = "\\Windows\0".encode_utf16().collect();
    let mut calledFromServer: u8 = 0;
    let status = CsrClientConnectToServer(
      objectDirectory.as_ptr(),
      1,
      std::ptr::null_mut(),
      std::ptr::null_mut(),
      &mut calledFromServer,
    );
    status >= 0
  }
}

// =================================================================================================

pub fn moduleBase() -> usize
{
  let mut module: Handle = ptr::null_mut();
  let found = unsafe {
    GetModuleHandleExW(
      ModuleHandleFromAddress,
      moduleBase as *const () as *const u16,
      &mut module,
    )
  };
  if found == 0 {
    return 0;
  }
  module as usize
}

pub fn readErrno() -> i32
{
  unsafe { *_errno() }
}

pub fn readOsError() -> Option<u32>
{
  Some(unsafe { GetLastError() })
}

// =================================================================================================

const MallocAlignment: usize = crate::sys::MinAlignment * 2;

thread_local! {
  static AlignedAllocations: std::cell::RefCell<std::collections::HashSet<usize>> =
    std::cell::RefCell::new(std::collections::HashSet::new());
}

pub fn allocate(length: usize) -> *mut c_void
{
  unsafe { libc::malloc(length) }
}

pub fn allocateAligned(length: usize, alignment: usize) -> Result<*mut c_void, String>
{
  if alignment <= MallocAlignment {
    let pointer = unsafe { libc::malloc(length) };
    if pointer.is_null() {
      return Err(format!("malloc failed for {} bytes", length));
    }
    return Ok(pointer);
  }

  let pointer = unsafe { libc::aligned_malloc(length, alignment) };
  if pointer.is_null() {
    return Err(format!(
      "_aligned_malloc failed for {} bytes at alignment {}",
      length, alignment
    ));
  }
  AlignedAllocations.with(|set| {
    set.borrow_mut().insert(pointer as usize);
  });
  Ok(pointer)
}

pub fn deallocate(pointer: *mut c_void) -> ()
{
  let wasAligned =
    AlignedAllocations.with(|set| set.borrow_mut().remove(&(pointer as usize)));
  if wasAligned {
    unsafe { libc::aligned_free(pointer) };
  } else {
    unsafe { libc::free(pointer) };
  }
}

// =================================================================================================