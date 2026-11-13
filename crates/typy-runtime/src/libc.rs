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
    #[link_name = "open"]
    fn open_path(path: *const core::ffi::c_char, flags: i32, ...) -> i32;
    #[link_name = "close"]
    fn close_descriptor(descriptor: i32) -> i32;
    #[link_name = "fstat"]
    fn fstat_descriptor(descriptor: i32, status: *mut [u64; 18]) -> i32;
    fn __errno_location() -> *mut i32;
}

/// `EINTR`: a signal interrupted the call, which is then made again, as CPython does.
pub(crate) const EINTR: i32 = 4;
/// `EISDIR`: the path names a directory.
pub(crate) const EISDIR: i32 = 21;
/// The flags of `open`, as Linux numbers them on `x86_64` and `AArch64`.
pub(crate) const O_WRONLY: i32 = 0o1;
pub(crate) const O_CREAT: i32 = 0o100;
pub(crate) const O_EXCL: i32 = 0o200;
pub(crate) const O_TRUNC: i32 = 0o1000;
pub(crate) const O_APPEND: i32 = 0o2000;
pub(crate) const O_CLOEXEC: i32 = 0o2_000_000;

/// Returns the error number the last failed call set.
pub(crate) fn errno() -> i32 {
    // SAFETY: the C library gives the address of the calling thread's `errno`.
    unsafe { *__errno_location() }
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

/// Returns whether a descriptor is a directory, from the `st_mode` of `fstat`.
pub(crate) fn is_directory(descriptor: i32) -> bool {
    const S_IFMT: u32 = 0o170_000;
    const S_IFDIR: u32 = 0o040_000;
    // `struct stat` is 144 bytes on x86-64 and 128 on AArch64; `st_mode` is the low half of
    // its fourth word on x86-64 and of its third on AArch64.
    #[cfg(target_arch = "aarch64")]
    const MODE_WORD: usize = 2;
    #[cfg(not(target_arch = "aarch64"))]
    const MODE_WORD: usize = 3;
    let mut status = [0_u64; 18];
    // SAFETY: `status` is writable and larger than `struct stat`.
    if unsafe { fstat_descriptor(descriptor, &raw mut status) } != 0 {
        return false;
    }
    #[allow(
        clippy::cast_possible_truncation,
        reason = "st_mode is the low 32 bits of the word"
    )]
    let mode = status[MODE_WORD] as u32;
    mode & S_IFMT == S_IFDIR
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

    #[test]
    #[cfg_attr(miri, ignore = "Miri does not call these on a closed descriptor")]
    fn calls_on_a_descriptor_that_is_not_open_give_ebadf() {
        const EBADF: i32 = 9;
        let mut buffer = [0; 4];
        assert_eq!(super::read_retrying(-1, &mut buffer), Err(EBADF));
        assert_eq!(super::write_retrying(-1, b"x"), Err(EBADF));
        assert_eq!(super::write_retrying(-1, b""), Ok(()));
        assert_eq!(super::close(-1), Err(EBADF));
        assert!(!super::is_directory(-1));
        assert_eq!(super::open(b"/no/such/directory/file\0", 0), Err(2));
    }
}
