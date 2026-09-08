// Это общее хранилище данных для примеров.
// Т.е. тут нет main.rs - оно игнорируется.

#[cfg(target_os = "linux")]
pub const LibcPath: &str = "libc.so.6";
#[cfg(target_os = "macos")]
pub const LibcPath: &str = "libSystem.dylib";

#[cfg(target_os = "linux")]
pub const LibmPath: &str = "libm.so.6";
#[cfg(target_os = "macos")]
pub const LibmPath: &str = "libSystem.dylib";