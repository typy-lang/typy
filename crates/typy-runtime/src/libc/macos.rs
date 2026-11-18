//! The `errno` accessor and the C constants of macOS on Apple silicon, from the headers of
//! xnu and Apple's Libc (ADR-0057).

use core::ffi::c_char;

unsafe extern "C" {
    fn __error() -> *mut i32;
    fn strerror(errno: i32) -> *const c_char;
    fn pthread_self() -> *mut core::ffi::c_void;
    fn pthread_get_stackaddr_np(thread: *mut core::ffi::c_void) -> *mut core::ffi::c_void;
    fn pthread_get_stacksize_np(thread: *mut core::ffi::c_void) -> usize;
}

/// Returns the lowest address of the calling thread's stack. A thread a C host creates has
/// 512 KiB by default on macOS, where the main thread has 8 MiB, so the stack check measures
/// each thread's own stack (FUN-02).
pub(crate) fn stack_low() -> usize {
    // SAFETY: `pthread_self` names the calling thread, which is live while it asks about
    // itself; the address is the top of its stack, which grows down by the size.
    let (top, size) = unsafe {
        let thread = pthread_self();
        (
            pthread_get_stackaddr_np(thread).expose_provenance(),
            pthread_get_stacksize_np(thread),
        )
    };
    top.saturating_sub(size)
}

/// Returns the address of the calling thread's `errno`.
pub(crate) fn errno_address() -> *mut i32 {
    // SAFETY: the C library gives the address of the calling thread's `errno`.
    unsafe { __error() }
}

/// The flags of `open` that differ between systems.
pub(crate) const O_CREAT: i32 = 0x200;
pub(crate) const O_EXCL: i32 = 0x800;
pub(crate) const O_TRUNC: i32 = 0x400;
pub(crate) const O_APPEND: i32 = 0x8;
pub(crate) const O_CLOEXEC: i32 = 0x0100_0000;

/// Returns `st_mode` of a `struct stat` of 144 bytes: the `u16` at byte 4, after the 32-bit
/// `st_dev`.
pub(crate) const fn mode_of(status: &[u64; 18]) -> u32 {
    #[allow(
        clippy::cast_possible_truncation,
        reason = "st_mode is the 16 bits above the low 32 of the first word"
    )]
    let mode = (status[0] >> 32) as u16;
    mode as u32
}

/// The error numbers whose `OSError` subclass is not the same number on every system.
pub(crate) const EAGAIN: i32 = 35;
pub(crate) const EINPROGRESS: i32 = 36;
pub(crate) const EALREADY: i32 = 37;
pub(crate) const ECONNABORTED: i32 = 53;
pub(crate) const ECONNRESET: i32 = 54;
pub(crate) const ESHUTDOWN: i32 = 58;
pub(crate) const ETIMEDOUT: i32 = 60;
pub(crate) const ECONNREFUSED: i32 = 61;

/// Returns the bytes of the text the system's `strerror` gives `errno`, as CPython shows it on
/// macOS: "Unknown error: 500" for a number it does not name.
pub(crate) fn error_text(errno: i32, write: impl FnOnce(&[u8])) {
    // SAFETY: `strerror` returns a string ended by a zero byte, which stays as it is until
    // the next call; `write` copies it before any other call.
    let text = unsafe { core::ffi::CStr::from_ptr(strerror(errno)) };
    write(text.to_bytes());
}
