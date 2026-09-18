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
  fn GetModuleHandleA(moduleName: *const u8) -> Handle;
  fn GetProcAddress(module: Handle, name: *const u8) -> *mut c_void;
  fn FreeConsole() -> i32;
  fn AttachConsole(dwProcessId: u32) -> i32;
  fn GetCurrentProcessId() -> u32;
  fn ProcessIdToSessionId(processId: u32, sessionId: *mut u32) -> i32;
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
  // Undocumented. Re-establishes the ALPC connection to csrss.exe. Lazy: if
  // CsrInitOnceDone is already set (which is always the case in a CoW clone,
  // because the parent already went through CsrpConnectToServer), the call
  // becomes a no-op. The whole CSR data block must be zeroed first.
  //
  // NOTE on the signature: ReactOS documents ConnectionInfoLength as a
  // PULONG (in-out), but on Win10 1809+ Microsoft reshaped the API so that
  // it is a plain ULONG value. We follow the Win10/11 shape used by WINNIE
  // (NDSS'21, forklib/fork.cpp), cross-checked by reverse-engineering
  // ntdll!CsrClientConnectToServer.
  fn CsrClientConnectToServer(
    ObjectDirectory: *const u16,
    ServerId: u32,
    ConnectionInfo: *mut c_void,
    ConnectionInfoLength: u32,
    CalledFromServer: *mut u8,
  ) -> i32;
  // Undocumented. Registers the current thread with CSRSS (CSR_THREAD
  // allocation on the csrss.exe side). Without it, the first Win32 call
  // that does CsrClientCallServer can blow up because the thread is unknown
  // to the subsystem.
  fn RtlRegisterThreadWithCsrss() -> i32;
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
// CSRSS reconnect (undocumented).
//
// After RtlCloneUserProcess the child inherits the parent's CSR data block
// in ntdll.dll verbatim:
//
//   CsrServerApiRoutine, CsrClientProcess, CsrInitOnceDone, CsrPortName,
//   CsrProcessId, CsrReadOnlySharedMemorySize, CsrPortMemoryRemoteDelta,
//   CsrPortHandle, CsrPortHeap, CsrPortBaseTag, CsrHeap, RtlpCurDirRef,
//   ... up to RtlpEnvironLookupTable.
//
// Because `CsrInitOnceDone` is already 1, CsrClientConnectToServer bails
// out immediately (lazy-init guard). The handle/heap/ports are stale (they
// reference the parent's CSR_PROCESS on the csrss.exe side). Any Win32 call
// that does CsrClientCallServer (file APIs, _stat64, activation contexts,
// console, etc.) then crashes the clone — observed as ERROR_BROKEN_PIPE
// (109) on the data pipe. ucrt-only paths (math, string) keep working
// because they never touch CSR.
//
// Fix (cross-checked against WINNIE, NDSS'21, forklib/fork.cpp):
//   1. Resolve the address range [CsrServerApiRoutine .. RtlpEnvironLookupTable)
//      in the healthy zygote, BEFORE the first clone. The address is cached
//      in a static OnceLock; children inherit it through CoW for free.
//   2. In the clone: zero out the whole block — not just CsrPortHandle — so
//      CsrInitOnceDone goes back to 0 and CsrClientConnectToServer will run.
//   3. Call CsrClientConnectToServer TWICE:
//        a) BASESRV  (ServerId=1), ConnectionInfo = &kernelbase!CtrlRoutine,
//           ConnectionInfoLength = 8.
//        b) USERSRV  (ServerId=3), ConnectionInfo = zeroed 0x240-byte buffer,
//           ConnectionInfoLength = 0x240.
//      ObjectDirectory MUST be `\Sessions\{sid}\Windows`, not `\Windows`
//      (the latter resolves to session 0 = services).
//   4. Call RtlRegisterThreadWithCsrss so the new thread is registered with
//      the subsystem. Without it, the first CsrClientCallServer can AV
//      because the TID is not in the CSR_THREAD table.
//
// Notes:
//   - NotifyCsrssParent (CsrClientCallServer with BasepCreateProcess from
//     the parent) is optional per WINNIE — the child works without it.
//   - Hardcoded offsets are NOT used: we resolve two RVAs via dbghelp at
//     zygote startup so the same binary works across Win10/Win11 builds.
//   - The first resolve pulls ntdll.pdb from msdl (or _NT_SYMBOL_PATH if
//     set). In air-gapped CI, pre-populate the symbol cache.
// =================================================================================================

#[derive(Clone, Copy)]
struct CsrDataBlock {
  base: usize,
  size: usize,
  // kernelbase!CtrlRoutine address (passed as BASESRV ConnectionInfo).
  // Stored as `usize` so the struct is Send + Sync — raw pointers are
  // neither. The address is read once from dbghelp and only ever
  // dereferenced inside `unsafe` blocks in `reconnectCsr`.
  ctrl_routine: usize,
}

static CsrDataBlockAddress: std::sync::OnceLock<Option<CsrDataBlock>> =
  std::sync::OnceLock::new();

/// Look up a single symbol in the current process via dbghelp. Returns the
/// VA (not RVA) of the symbol on success.
unsafe fn lookupSymbol(process: Handle, names: &[&std::ffi::CStr]) -> Option<u64>
{
  for name in names {
    let mut info: SYMBOL_INFO = unsafe { std::mem::zeroed() };
    info.SizeOfStruct = 88; // sizeof(SYMBOL_INFO) with Name[1], x64
    info.MaxNameLen = 2000;
    let ok = unsafe { SymFromName(process, name.as_ptr(), &mut info) };
    if ok != 0 {
      return Some(info.Address);
    }
  }
  None
}

/// Resolve the CSR data block in ntdll + the CtrlRoutine entry in
/// kernelbase. Idempotent. Safe to call multiple times. Stores `None` on
/// failure (no network / no symbols); `reconnectCsr` then returns `false`
/// and the clone falls back to the broken-port path.
pub fn resolveCsrPortHandle() -> ()
{
  CsrDataBlockAddress.get_or_init(|| unsafe {
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
      eprintln!("[csr] SymInitializeW failed: {}", GetLastError());
      return None;
    }

    let csrBegin = lookupSymbol(process, &[
      c"ntdll!CsrServerApiRoutine",
      c"CsrServerApiRoutine",
      c"_CsrServerApiRoutine",
    ]);
    let csrEnd = lookupSymbol(process, &[
      c"ntdll!RtlpEnvironLookupTable",
      c"RtlpEnvironLookupTable",
      c"_RtlpEnvironLookupTable",
    ]);
    // Fallback end marker: RtlpCurDirRef sits immediately before
    // RtlpEnvironLookupTable in ntdll's .data. It is a qword itself, so
    // the block end is (RtlpCurDirRef + 8).
    let csrEndFallback = if csrEnd.is_none() {
      lookupSymbol(process, &[
        c"ntdll!RtlpCurDirRef",
        c"RtlpCurDirRef",
        c"_RtlpCurDirRef",
      ])
    } else {
      None
    };
    SymCleanup(process);

    let Some(begin) = csrBegin else {
      eprintln!("[csr] symbol lookup failed: CsrServerApiRoutine not found");
      return None;
    };

    let (size, via) = if let Some(e) = csrEnd {
      if e <= begin {
        eprintln!(
          "[csr] unexpected symbol order: begin={:#x} end={:#x}",
          begin, e
        );
        return None;
      }
      ((e - begin) as usize, "via symbol")
    } else if let Some(f) = csrEndFallback {
      if f <= begin {
        eprintln!(
          "[csr] unexpected fallback symbol order: begin={:#x} fallback={:#x}",
          begin, f
        );
        return None;
      }
      (((f - begin) + 8) as usize, "via symbol")
    } else {
      eprintln!(
        "[csr] RtlpEnvironLookupTable/RtlpCurDirRef not found, using hardcoded fallback size 128"
      );
      (128usize, "via fallback")
    };

    // kernelbase!CtrlRoutine — passed as BASESRV ConnectionInfo. If not
    // found, fall back to NULL (some Win10 builds expose it under a
    // different name; BASESRV is tolerant of a NULL pointer).
    let kernelbase = GetModuleHandleA(c"kernelbase.dll".as_ptr().cast());
    let ctrlRoutine = if !kernelbase.is_null() {
      GetProcAddress(kernelbase, c"CtrlRoutine".as_ptr().cast())
    } else {
      std::ptr::null_mut()
    };

    eprintln!(
      "[csr] resolved block: base={:#x} size={} ({}) ctrl_routine={:p}",
      begin,
      size,
      via,
      ctrlRoutine
    );

    Some(CsrDataBlock {
      base: begin as usize,
      size,
      ctrl_routine: ctrlRoutine as usize,
    })
  });
}

/// Child-side: zero the stale CSR data block, then re-run
/// CsrClientConnectToServer for BASESRV (ServerId=1) and USERSRV
/// (ServerId=3) against `\Sessions\{sid}\Windows`, and finally
/// RtlRegisterThreadWithCsrss.
///
/// Returns `true` only if every step succeeded. The caller should still
/// proceed even on `false` — the failure mode is the same as without this
/// fix (ERROR_BROKEN_PIPE on the data pipe), but logging the exact step
/// that failed helps diagnosing version-specific issues.
pub fn reconnectCsr() -> bool
{
  let block = match CsrDataBlockAddress.get() {
    Some(Some(b)) => *b,
    _ => {
      eprintln!("[csr] reconnectCsr: block was never resolved");
      return false;
    }
  };

  // Step 1: zero the entire CSR data block so CsrInitOnceDone goes back to 0.
  unsafe {
    std::ptr::write_bytes(block.base as *mut u8, 0, block.size);
  }

  // Step 2: build the per-session object directory `\Sessions\{sid}\Windows`.
  // CSRSS is session-local; using `\Windows` connects to session 0 (services)
  // and any subsequent Win32 call in an interactive session will fail.
  let mut sessionId: u32 = 0;
  if unsafe { ProcessIdToSessionId(currentProcessId(), &mut sessionId) } == 0 {
    eprintln!(
      "[csr] ProcessIdToSessionId failed: {}",
      unsafe { GetLastError() }
    );
    return false;
  }
  let objectDirectory: Vec<u16> =
    format!("\\Sessions\\{}\\Windows\0", sessionId)
      .encode_utf16()
      .collect();

  // Step 3a: connect to BASESRV (ServerId=1). ConnectionInfo is the address
  // of kernelbase!CtrlRoutine (8 bytes on x64) — passed verbatim, no capture
  // buffer.
  let mut baseSrvInfo: *mut c_void = block.ctrl_routine as *mut c_void;
  let mut calledFromServer: u8 = 0;
  let status1 = unsafe {
    CsrClientConnectToServer(
      objectDirectory.as_ptr(),
      1,
      &mut baseSrvInfo as *mut _ as *mut c_void,
      8,
      &mut calledFromServer,
    )
  };
  if status1 < 0 {
    eprintln!("[csr] CsrClientConnectToServer(BASESRV) failed: ntstatus={:#x}", status1 as i32);
    return false;
  }

  // Step 3b: connect to USERSRV (ServerId=3). ConnectionInfo is a zeroed
  // 0x240-byte buffer (matches what kernel32!BasepConnect does on first
  // connect — WINNIE fork.cpp uses the same shape).
  let mut userSrvInfo = [0u8; 0x240];
  let status2 = unsafe {
    CsrClientConnectToServer(
      objectDirectory.as_ptr(),
      3,
      userSrvInfo.as_mut_ptr() as *mut c_void,
      0x240,
      &mut calledFromServer,
    )
  };
  if status2 < 0 {
    eprintln!("[csr] CsrClientConnectToServer(USERSRV) failed: ntstatus={:#x}", status2 as i32);
    return false;
  }

  // Step 4: register the current thread with CSRSS. This is the piece that
  // was completely missing from the first attempt.
  let status3 = unsafe { RtlRegisterThreadWithCsrss() };
  if status3 < 0 {
    eprintln!(
      "[csr] RtlRegisterThreadWithCsrss failed: ntstatus={:#x}",
      status3
    );
    return false;
  }

  eprintln!(
    "[csr] reconnectCsr OK (sid={} base={:#x} size={} ctrl={:#x})",
    sessionId,
    block.base,
    block.size,
    block.ctrl_routine
  );
  true
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