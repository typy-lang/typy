//! The `errno` accessor and the C constants of Linux on x86-64 and AArch64, from glibc's
//! headers (ADR-0057).

unsafe extern "C" {
    fn __errno_location() -> *mut i32;
    fn getpid() -> i32;
    fn syscall(number: i64, ...) -> i64;
    fn pthread_self() -> usize;
    fn pthread_getattr_np(thread: usize, attributes: *mut [u64; 8]) -> i32;
    fn pthread_attr_getstack(
        attributes: *const [u64; 8],
        stack: *mut *mut u8,
        size: *mut usize,
    ) -> i32;
    fn pthread_attr_destroy(attributes: *mut [u64; 8]) -> i32;
    fn pthread_key_create(
        key: *mut u32,
        destructor: Option<unsafe extern "C" fn(*mut core::ffi::c_void)>,
    ) -> i32;
    fn pthread_getspecific(key: u32) -> *mut core::ffi::c_void;
    fn pthread_setspecific(key: u32, value: *const core::ffi::c_void) -> i32;
}

/// The number of `gettid`, which C libraries before glibc 2.30 and musl 1.2.2 do not wrap.
#[cfg(target_arch = "aarch64")]
const SYS_GETTID: i64 = 178;
#[cfg(not(target_arch = "aarch64"))]
const SYS_GETTID: i64 = 186;

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

/// Returns true on the process's main thread, the one thread whose ID is the process ID.
pub(crate) fn is_main_thread() -> bool {
    // SAFETY: neither call has preconditions.
    unsafe { syscall(SYS_GETTID) == i64::from(getpid()) }
}

/// Returns the lowest address of the calling thread's stack, from the attributes it was
/// created with, or `None` when the C library does not give them.
pub(crate) fn thread_stack_low() -> Option<usize> {
    // `pthread_attr_t` is 56 bytes on x86-64 and 64 on AArch64, in glibc and in musl.
    let mut attributes = [0_u64; 8];
    // SAFETY: `attributes` is as large as `pthread_attr_t`, filled by the first call and
    // destroyed once read.
    let read = unsafe { pthread_getattr_np(pthread_self(), &raw mut attributes) } == 0;
    read.then(|| {
        let (mut stack, mut size) = (core::ptr::null_mut(), 0);
        // SAFETY: as above; the two outputs are writable.
        unsafe {
            let status =
                pthread_attr_getstack(&raw const attributes, &raw mut stack, &raw mut size);
            pthread_attr_destroy(&raw mut attributes);
            (status == 0).then(|| stack.addr())
        }
    })
    .flatten()
}

/// Returns a new key for a value each thread keeps, or `None` when the C library has none
/// left.
pub(crate) fn new_thread_key() -> Option<u32> {
    let mut key = 0;
    // SAFETY: `key` is writable; the values kept under it are numbers, which need no
    // destructor.
    (unsafe { pthread_key_create(&raw mut key, None) } == 0).then_some(key)
}

/// Returns the number the calling thread keeps under `key`, 0 before it keeps one.
pub(crate) fn thread_value(key: u32) -> usize {
    // SAFETY: `key` came from `new_thread_key`.
    unsafe { pthread_getspecific(key) }.addr()
}

/// Keeps `value` for the calling thread under `key`.
pub(crate) fn keep_thread_value(key: u32, value: usize) {
    // SAFETY: as above; the value is a number held as a pointer, never followed.
    unsafe { pthread_setspecific(key, core::ptr::without_provenance(value)) };
}
