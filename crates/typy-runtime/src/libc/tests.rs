//! Tests of the C library functions the runtime calls.

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

#[test]
#[cfg_attr(miri, ignore = "Miri does not call write on a closed descriptor")]
fn write_all_stops_at_a_failed_write() {
    super::write_all(-1, b"lost");
}

unsafe extern "C" {
    #[cfg(target_os = "linux")]
    fn setrlimit(resource: i32, limit: *const [u64; 2]) -> i32;
    fn sigaction(signal: i32, action: *const Action, old: *mut Action) -> i32;
    fn pthread_kill(thread: std::os::unix::thread::RawPthread, signal: i32) -> i32;
    #[cfg(target_os = "linux")]
    fn mkfifo(path: *const core::ffi::c_char, mode: u32) -> i32;
}

/// `struct sigaction` of glibc on 64-bit Linux.
#[cfg(target_os = "linux")]
#[repr(C)]
struct Action {
    handler: extern "C" fn(i32),
    mask: [u64; 16],
    flags: i32,
    restorer: usize,
}

/// `struct sigaction` of macOS, whose signal set is 32 bits.
#[cfg(target_os = "macos")]
#[repr(C)]
struct Action {
    handler: extern "C" fn(i32),
    mask: u32,
    flags: i32,
}

#[cfg(target_os = "linux")]
const SIGUSR1: i32 = 10;
#[cfg(target_os = "macos")]
const SIGUSR1: i32 = 30;

/// Returns an action that runs `ignore` and does not restart an interrupted call.
#[cfg(target_os = "linux")]
const fn ignoring() -> Action {
    Action {
        handler: ignore,
        mask: [0; 16],
        flags: 0,
        restorer: 0,
    }
}

#[cfg(target_os = "macos")]
const fn ignoring() -> Action {
    Action {
        handler: ignore,
        mask: 0,
        flags: 0,
    }
}

extern "C" fn ignore(_: i32) {}

/// Runs `blocking` on a thread of its own and interrupts it with signals whose handler
/// does not restart the call, so the call fails with `EINTR`; then `release` lets the
/// call finish, and its result is returned.
fn interrupted<T: Send + 'static>(
    blocking: impl FnOnce() -> T + Send + 'static,
    release: impl FnOnce(),
) -> T {
    use std::os::unix::thread::JoinHandleExt;
    let action = ignoring();
    // SAFETY: `action` is a valid `struct sigaction`, and the handler does nothing.
    let installed = unsafe { sigaction(SIGUSR1, &raw const action, core::ptr::null_mut()) };
    assert_eq!(installed, 0);
    let thread = std::thread::spawn(blocking);
    for _ in 0..10 {
        std::thread::sleep(std::time::Duration::from_millis(5));
        // SAFETY: the thread is not joined yet, so its identifier is valid.
        unsafe { pthread_kill(thread.as_pthread_t(), SIGUSR1) };
    }
    release();
    thread.join().unwrap()
}

#[test]
#[cfg_attr(miri, ignore = "Miri does not deliver signals")]
fn a_read_interrupted_by_a_signal_is_retried() {
    let (reading, mut writing) = std::io::pipe().unwrap();
    let descriptor = std::os::fd::AsRawFd::as_raw_fd(&reading);
    let count = interrupted(
        move || {
            let mut buffer = [0; 4];
            super::read_retrying(descriptor, &mut buffer)
        },
        || std::io::Write::write_all(&mut writing, b"x").unwrap(),
    );
    assert_eq!(count, Ok(1));
    drop(reading);
}

#[test]
#[cfg_attr(miri, ignore = "Miri does not deliver signals")]
fn a_write_interrupted_by_a_signal_is_retried() {
    // More than a pipe holds, so the write waits for the reader.
    const SIZE: usize = 1 << 18;
    let (mut reading, writing) = std::io::pipe().unwrap();
    let descriptor = std::os::fd::AsRawFd::as_raw_fd(&writing);
    let mut found = vec![0; SIZE];
    let written = interrupted(
        move || super::write_retrying(descriptor, &vec![7; SIZE]),
        || std::io::Read::read_exact(&mut reading, &mut found).unwrap(),
    );
    assert_eq!(written, Ok(()));
    assert!(found.iter().all(|byte| *byte == 7));
    drop(writing);
}

// On macOS 15 the writer's `open` of a FIFO, interrupted by these signals, did not return
// once the reader opened, so the test runs where the call returns; `open` retries `EINTR`
// with the same code on every system.
#[test]
#[cfg(target_os = "linux")]
#[cfg_attr(miri, ignore = "Miri does not deliver signals")]
fn an_open_interrupted_by_a_signal_is_retried() {
    let directory = std::env::temp_dir().join(format!("typy-libc-fifo-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&directory);
    std::fs::create_dir_all(&directory).unwrap();
    let path = directory.join("fifo");
    let mut bytes = path.to_str().unwrap().as_bytes().to_vec();
    bytes.push(0);
    // SAFETY: the path ends with a zero byte.
    assert_eq!(unsafe { mkfifo(bytes.as_ptr().cast(), 0o600) }, 0);
    // Opening one end of a FIFO waits until the other end is opened.
    let opened = interrupted(
        move || super::open(&bytes, super::O_WRONLY | super::O_CLOEXEC),
        || drop(std::fs::File::open(&path).unwrap()),
    );
    let descriptor = opened.unwrap();
    assert_eq!(super::close(descriptor), Ok(()));
    std::fs::remove_dir_all(&directory).unwrap();
}

#[test]
#[cfg(target_os = "linux")]
#[cfg_attr(miri, ignore = "Miri does not change resource limits")]
fn a_stack_without_a_limit_gives_none() {
    const RLIMIT_STACK: i32 = 3;
    let mut kept = [0_u64; 2];
    // SAFETY: `kept` is a writable `struct rlimit` on 64-bit Linux.
    assert_eq!(unsafe { super::getrlimit(RLIMIT_STACK, &raw mut kept) }, 0);
    // The soft limit rises to the hard one, which is unlimited where the tests run.
    let raised = [kept[1], kept[1]];
    // SAFETY: `raised` is a readable `struct rlimit`.
    assert_eq!(unsafe { setrlimit(RLIMIT_STACK, &raw const raised) }, 0);
    let expected = (kept[1] != super::system::RLIM_INFINITY).then_some(kept[1]);
    assert_eq!(super::stack_limit(), expected);
    // SAFETY: `kept` is a readable `struct rlimit`.
    assert_eq!(unsafe { setrlimit(RLIMIT_STACK, &raw const kept) }, 0);
}
