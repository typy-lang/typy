//! The C library functions the runtime calls.

use core::ffi::c_void;

unsafe extern "C" {
    fn malloc(size: usize) -> *mut c_void;
    fn realloc(pointer: *mut c_void, size: usize) -> *mut c_void;
    fn free(pointer: *mut c_void);
    fn write(descriptor: i32, buffer: *const c_void, count: usize) -> isize;
    #[link_name = "read"]
    fn read_bytes(descriptor: i32, buffer: *mut c_void, count: usize) -> isize;
    fn abort() -> !;
    fn getrlimit(resource: i32, limit: *mut [u64; 2]) -> i32;
}

/// Standard output.
pub(crate) const STDIN: i32 = 0;
pub(crate) const STDOUT: i32 = 1;
/// Standard error.
pub(crate) const STDERR: i32 = 2;

/// Allocates `size` bytes; null when the memory is not available.
pub(crate) fn allocate(size: usize) -> *mut u8 {
    // SAFETY: `malloc` has no preconditions; a null result is handled by the caller.
    unsafe { malloc(size).cast() }
}

/// Resizes an allocation; null when the memory is not available, and the old block is then unchanged.
pub(crate) fn reallocate(pointer: *mut u8, size: usize) -> *mut u8 {
    // SAFETY: `pointer` is null or came from `malloc` or `realloc`, which callers keep to.
    unsafe { realloc(pointer.cast(), size).cast() }
}

/// Frees an allocation made by `allocate` or `reallocate`.
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
/// Reads up to `buffer.len()` bytes from `descriptor`, and returns how many it read: 0 at the
/// end of the input, and after an error, which a reader of standard input treats as its end.
pub(crate) fn read(descriptor: i32, buffer: &mut [u8]) -> usize {
    // SAFETY: the buffer is writable for its length.
    let count = unsafe { read_bytes(descriptor, buffer.as_mut_ptr().cast(), buffer.len()) };
    usize::try_from(count).unwrap_or(0)
}

pub(crate) fn abort_with(message: &[u8]) -> ! {
    write_all(STDERR, message);
    // SAFETY: `abort` has no preconditions and does not return.
    unsafe { abort() }
}

/// Returns the soft limit of the stack size in bytes, or `None` when there is no limit.
pub(crate) fn stack_limit() -> Option<u64> {
    const RLIMIT_STACK: i32 = 3;
    const RLIM_INFINITY: u64 = u64::MAX;
    let mut limit = [0_u64; 2];
    // SAFETY: `limit` is a writable pair of 64-bit values, the layout of `struct rlimit` on 64-bit Linux.
    let status = unsafe { getrlimit(RLIMIT_STACK, &raw mut limit) };
    (status == 0 && limit[0] != RLIM_INFINITY).then_some(limit[0])
}

#[cfg(test)]
mod tests {
    #[test]
    #[cfg_attr(miri, ignore = "Miri does not read a closed descriptor")]
    fn reading_a_descriptor_that_fails_gives_nothing() {
        let mut buffer = [0; 4];
        assert_eq!(super::read(-1, &mut buffer), 0);
    }
}
