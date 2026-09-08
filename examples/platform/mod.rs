// Это общее хранилище данных для примеров.
// Т.е. тут нет main.rs - оно игнорируется.

#[cfg(target_os = "linux")]
#[allow(dead_code)]
pub const LibcPath: &str = "libc.so.6";
#[cfg(target_os = "macos")]
#[allow(dead_code)]
pub const LibcPath: &str = "libSystem.dylib";

#[cfg(target_os = "linux")]
#[allow(dead_code)]
pub const LibmPath: &str = "libm.so.6";
#[cfg(target_os = "macos")]
#[allow(dead_code)]
pub const LibmPath: &str = "libSystem.dylib";