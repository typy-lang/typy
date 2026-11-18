//! The `errno` accessor and the C constants of Linux on x86-64 and AArch64, from glibc's
//! headers (ADR-0057).

unsafe extern "C" {
    fn __errno_location() -> *mut i32;
}

/// Returns the address of the calling thread's `errno`.
pub(crate) fn errno_address() -> *mut i32 {
    // SAFETY: the C library gives the address of the calling thread's `errno`.
    unsafe { __errno_location() }
}

/// The flags of `open` that differ between systems.
pub(crate) const O_CREAT: i32 = 0o100;
pub(crate) const O_EXCL: i32 = 0o200;
pub(crate) const O_TRUNC: i32 = 0o1000;
pub(crate) const O_APPEND: i32 = 0o2000;
pub(crate) const O_CLOEXEC: i32 = 0o2_000_000;

/// The soft limit of a resource that has none.
pub(crate) const RLIM_INFINITY: u64 = u64::MAX;

/// Returns `st_mode` of a `struct stat`, which is 144 bytes on x86-64 and 128 on AArch64:
/// the low half of its fourth word on x86-64 and of its third on AArch64.
pub(crate) const fn mode_of(status: &[u64; 18]) -> u32 {
    #[cfg(target_arch = "aarch64")]
    const MODE_WORD: usize = 2;
    #[cfg(not(target_arch = "aarch64"))]
    const MODE_WORD: usize = 3;
    #[allow(
        clippy::cast_possible_truncation,
        reason = "st_mode is the low 32 bits of the word"
    )]
    let mode = status[MODE_WORD] as u32;
    mode
}

/// The error numbers whose `OSError` subclass is not the same number on every system.
pub(crate) const EAGAIN: i32 = 11;
pub(crate) const ECONNABORTED: i32 = 103;
pub(crate) const ECONNRESET: i32 = 104;
pub(crate) const ESHUTDOWN: i32 = 108;
pub(crate) const ETIMEDOUT: i32 = 110;
pub(crate) const ECONNREFUSED: i32 = 111;
pub(crate) const EALREADY: i32 = 114;
pub(crate) const EINPROGRESS: i32 = 115;
/// Linux has no `ENOTCAPABLE`, which CPython makes a `PermissionError` where it exists.
pub(crate) const ENOTCAPABLE: Option<i32> = None;
