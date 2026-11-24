//! The C library functions the runtime calls, with the constants of the system it runs on
//! from a module of its own (ADR-0057).

use core::ffi::c_void;

#[cfg(target_os = "linux")]
mod linux;
#[cfg(target_os = "macos")]
mod macos;
#[cfg(target_os = "linux")]
use linux as system;
#[cfg(target_os = "macos")]
use macos as system;

pub(crate) use system::{
    EAGAIN, EALREADY, ECONNABORTED, ECONNREFUSED, ECONNRESET, EINPROGRESS, ESHUTDOWN, ETIMEDOUT,
    O_APPEND, O_CLOEXEC, O_CREAT, O_EXCL, O_TRUNC,
};
#[cfg(target_os = "macos")]
pub(crate) use system::{error_text, stack_low};
#[cfg(target_os = "linux")]
pub(crate) use system::{
    is_main_thread, keep_thread_value, new_thread_key, thread_stack_low, thread_value,
};

unsafe extern "C" {
    // The unit tests allocate with these; a program allocates through its own hooks
    // (ADR-0043).
    #[cfg(test)]
    fn malloc(size: usize) -> *mut c_void;
    #[cfg(test)]
    fn realloc(pointer: *mut c_void, size: usize) -> *mut c_void;
    #[cfg(test)]
    fn free(pointer: *mut c_void);
    fn write(descriptor: i32, buffer: *const c_void, count: usize) -> isize;
    #[link_name = "read"]
    fn read_bytes(descriptor: i32, buffer: *mut c_void, count: usize) -> isize;
    fn abort() -> !;
    fn exit(status: i32) -> !;
    #[cfg(target_os = "linux")]
    fn getrlimit(resource: i32, limit: *mut [u64; 2]) -> i32;
    #[link_name = "open"]
    fn open_path(path: *const core::ffi::c_char, flags: i32, ...) -> i32;
    #[link_name = "close"]
    fn close_descriptor(descriptor: i32) -> i32;
    #[link_name = "fstat"]
    fn fstat_descriptor(descriptor: i32, status: *mut [u64; 18]) -> i32;
    fn lseek(descriptor: i32, offset: i64, whence: i32) -> i64;
    fn ftruncate(descriptor: i32, length: i64) -> i32;
    fn isatty(descriptor: i32) -> i32;
}

/// `EINTR`: a signal interrupted the call, which is then made again, as CPython does.
pub(crate) const EINTR: i32 = 4;
/// `EISDIR`: the path names a directory.
pub(crate) const EISDIR: i32 = 21;
/// The flags of `open` that every system numbers alike.
pub(crate) const O_WRONLY: i32 = 0o1;
pub(crate) const O_RDWR: i32 = 0o2;

/// Returns the error number the last failed call set.
pub(crate) fn errno() -> i32 {
    // SAFETY: the address is that of the calling thread's `errno`, which lives as long as the
    // thread.
    unsafe { *system::errno_address() }
}

/// Opens the path, a string ended by a zero byte, with `flags`, creating a file with the
/// permissions 0o666 less the umask, as CPython does; returns the descriptor or the error
/// number. An interrupted call is made again.
pub(crate) fn open(path: &[u8], flags: i32) -> Result<i32, i32> {
    debug_assert_eq!(path.last(), Some(&0));
    loop {
        // SAFETY: the path ends with a zero byte, and the mode is the third argument `open`
        // reads when `O_CREAT` is set.
        let descriptor = unsafe { open_path(path.as_ptr().cast(), flags, 0o666_u32) };
        if descriptor >= 0 {
            return Ok(descriptor);
        }
        let error = errno();
        if error != EINTR {
            return Err(error);
        }
    }
}

/// Closes a descriptor; an interrupted close has closed it, as Linux defines.
pub(crate) fn close(descriptor: i32) -> Result<(), i32> {
    // SAFETY: closing a descriptor has no memory effects.
    if unsafe { close_descriptor(descriptor) } == 0 {
        return Ok(());
    }
    match errno() {
        EINTR => Ok(()),
        error => Err(error),
    }
}

/// `EINVAL`: an argument is out of range.
pub(crate) const EINVAL: i32 = 22;
/// The `whence` of `lseek`.
pub(crate) const SEEK_SET: i32 = 0;
pub(crate) const SEEK_CUR: i32 = 1;
pub(crate) const SEEK_END: i32 = 2;

/// Moves the descriptor's position by `offset` from `whence`; returns the new position or the
/// error number.
pub(crate) fn seek(descriptor: i32, offset: i64, whence: i32) -> Result<i64, i32> {
    // SAFETY: moving a descriptor's position has no memory effects.
    let position = unsafe { lseek(descriptor, offset, whence) };
    if position < 0 {
        return Err(errno());
    }
    Ok(position)
}

/// Cuts or extends the descriptor's file to `length` bytes; returns the error number.
pub(crate) fn truncate(descriptor: i32, length: i64) -> Result<(), i32> {
    // SAFETY: resizing a descriptor's file has no memory effects.
    if unsafe { ftruncate(descriptor, length) } == 0 {
        return Ok(());
    }
    Err(errno())
}

/// Returns whether the descriptor is a terminal.
pub(crate) fn is_terminal(descriptor: i32) -> bool {
    // SAFETY: asking about a descriptor has no memory effects.
    unsafe { isatty(descriptor) == 1 }
}

/// Returns whether a descriptor is a directory, from the `st_mode` of `fstat`.
pub(crate) fn is_directory(descriptor: i32) -> bool {
    directory_status(descriptor).unwrap_or(false)
}

/// Returns whether an open descriptor is a directory, or the error number of `fstat`, which
/// is `EBADF` for a descriptor that is not open.
pub(crate) fn directory_status(descriptor: i32) -> Result<bool, i32> {
    const S_IFMT: u32 = 0o170_000;
    const S_IFDIR: u32 = 0o040_000;
    let mut status = [0_u64; 18];
    // SAFETY: `status` is writable and no smaller than `struct stat` on any system.
    if unsafe { fstat_descriptor(descriptor, &raw mut status) } != 0 {
        return Err(errno());
    }
    Ok(system::mode_of(&status) & S_IFMT == S_IFDIR)
}

/// Reads into `buffer`, making an interrupted call again; returns how many bytes it read, 0 at
/// the end, or the error number.
pub(crate) fn read_retrying(descriptor: i32, buffer: &mut [u8]) -> Result<usize, i32> {
    loop {
        // SAFETY: the buffer is writable for its length.
        let count = unsafe { read_bytes(descriptor, buffer.as_mut_ptr().cast(), buffer.len()) };
        if let Ok(count) = usize::try_from(count) {
            return Ok(count);
        }
        let error = errno();
        if error != EINTR {
            return Err(error);
        }
    }
}

/// Writes all of `bytes`, making an interrupted call again; returns the error number of a
/// write that fails.
pub(crate) fn write_retrying(descriptor: i32, mut bytes: &[u8]) -> Result<(), i32> {
    while !bytes.is_empty() {
        // SAFETY: the pointer and length describe a live slice.
        let written = unsafe { write(descriptor, bytes.as_ptr().cast(), bytes.len()) };
        match usize::try_from(written) {
            Ok(written) => bytes = &bytes[written.min(bytes.len())..],
            Err(_) if errno() == EINTR => {}
            Err(_) => return Err(errno()),
        }
    }
    Ok(())
}

/// Standard output.
pub(crate) const STDIN: i32 = 0;
pub(crate) const STDOUT: i32 = 1;
/// Standard error.
pub(crate) const STDERR: i32 = 2;

/// Allocates `size` bytes; null when the memory is not available.
#[cfg(test)]
pub(crate) fn allocate(size: usize) -> *mut u8 {
    // SAFETY: `malloc` has no preconditions; a null result is handled by the caller.
    unsafe { malloc(size).cast() }
}

/// Resizes an allocation; null when the memory is not available, and the old block is then unchanged.
#[cfg(test)]
pub(crate) fn reallocate(pointer: *mut u8, size: usize) -> *mut u8 {
    // SAFETY: `pointer` is null or came from `malloc` or `realloc`, which callers keep to.
    unsafe { realloc(pointer.cast(), size).cast() }
}

/// Frees an allocation made by `allocate` or `reallocate`.
#[cfg(test)]
pub(crate) fn release(pointer: *mut u8) {
    // SAFETY: callers pass a pointer from `allocate` or `reallocate` that is freed once.
    unsafe { free(pointer.cast()) }
}

/// Writes all of `bytes` to a file descriptor, retrying short writes; a failed write ends the attempt.
pub(crate) fn write_all(descriptor: i32, mut bytes: &[u8]) {
    while !bytes.is_empty() {
        // SAFETY: the pointer and length describe a live slice.
        let written = unsafe { write(descriptor, bytes.as_ptr().cast(), bytes.len()) };
        let Ok(written) = usize::try_from(written) else {
            return;
        };
        if written == 0 {
            return;
        }
        bytes = &bytes[written.min(bytes.len())..];
    }
}

/// Writes a message to standard error and aborts the process.
pub(crate) fn abort_with(message: &[u8]) -> ! {
    write_all(STDERR, message);
    // SAFETY: `abort` has no preconditions and does not return.
    unsafe { abort() }
}

/// Ends the process at once, as an uncaught signal does.
pub(crate) fn abort_process() -> ! {
    // SAFETY: `abort` has no preconditions and does not return.
    unsafe { abort() }
}

/// Ends the process with `status`, after the C library runs its exit handlers and the
/// destructors of the loaded objects.
pub(crate) fn exit_process(status: i32) -> ! {
    // SAFETY: `exit` has no preconditions and does not return.
    unsafe { exit(status) }
}

/// Returns the soft limit of the stack size in bytes, or `None` when there is no limit.
#[cfg(target_os = "linux")]
pub(crate) fn stack_limit() -> Option<u64> {
    const RLIMIT_STACK: i32 = 3;
    let mut limit = [0_u64; 2];
    // SAFETY: `limit` is a writable pair of 64-bit values, the layout of `struct rlimit` on
    // 64-bit Linux.
    let status = unsafe { getrlimit(RLIMIT_STACK, &raw mut limit) };
    (status == 0 && limit[0] != system::RLIM_INFINITY).then_some(limit[0])
}

#[cfg(test)]
mod tests;
