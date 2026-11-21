//! The stack check at the start of recursive functions, which raises `RecursionError` before the stack runs out (FUN-02).

use crate::Raising;
use crate::cell::ProgramCell;
use crate::exception::{ExceptionKind, ExceptionObject, raise};

/// The stack left unused when the check fires, for the code that reports the error.
const MARGIN: usize = 64 * 1024;
/// The stack size assumed when the system sets no limit.
#[cfg(target_os = "linux")]
const DEFAULT_STACK: u64 = 8 * 1024 * 1024;

static LIMIT: ProgramCell<usize> = ProgramCell::new(0);

/// The stack size the limit allows, read from `getrlimit` once; zero until then.
#[cfg(target_os = "linux")]
static SIZE: ProgramCell<usize> = ProgramCell::new(0);

/// The key under which each thread other than the main one keeps the limit of its own stack,
/// made by the first export call on such a thread (ADR-0075).
#[cfg(target_os = "linux")]
static THREAD_KEY: ProgramCell<Option<u32>> = ProgramCell::new(None);

/// Returns the stack pointer. The register is read, not the address of a local, because a
/// sanitizer may keep locals on a separate fake stack, where the check would never fire.
#[cfg(not(miri))]
#[inline]
fn stack_pointer() -> usize {
    let pointer: usize;
    // SAFETY: copying the stack pointer into a register reads no memory and changes nothing
    // else.
    #[cfg(target_arch = "x86_64")]
    unsafe {
        core::arch::asm!(
            "mov {}, rsp",
            out(reg) pointer,
            options(nomem, nostack, preserves_flags)
        );
    }
    // SAFETY: as above.
    #[cfg(target_arch = "aarch64")]
    unsafe {
        core::arch::asm!(
            "mov {}, sp",
            out(reg) pointer,
            options(nomem, nostack, preserves_flags)
        );
    }
    pointer
}

/// Returns the address of a local, which stands for the stack pointer under Miri: it runs no
/// inline assembly and keeps no fake stack.
#[cfg(miri)]
#[inline]
fn stack_pointer() -> usize {
    let marker = 0_u8;
    core::ptr::addr_of!(marker).addr()
}

/// Records the lowest address the stack may reach below the current stack pointer: `main`
/// calls it first, and a library each time an export is called, which may be from another
/// thread (ADR-0044). On macOS it is the margin above the bottom of the calling thread's own
/// stack, whose size depends on the thread.
#[cfg(target_os = "macos")]
pub(crate) fn record_limit() {
    let low = crate::libc::stack_low();
    LIMIT.with(|limit| *limit = low.saturating_add(MARGIN));
}

/// Records the lowest address the stack may reach below the current stack pointer: `main`
/// calls it first, and a library each time an export is called, which may be from another
/// thread (ADR-0044).
#[cfg(target_os = "linux")]
pub(crate) fn record_limit() {
    record_limit_on(crate::libc::is_main_thread());
}

/// Records the limit of the main thread's stack, where `main` runs: a program links no code
/// that reads another thread's stack.
#[cfg(all(target_os = "linux", not(test)))]
pub(crate) fn record_main_limit() {
    LIMIT.with(|kept| *kept = main_limit());
}

/// Records the limit of the calling thread's stack, which on macOS is always the thread's
/// own.
#[cfg(all(target_os = "macos", not(test)))]
pub(crate) fn record_main_limit() {
    record_limit();
}

/// Records the limit of the main thread's stack, or of another thread's own (ADR-0075).
#[cfg(target_os = "linux")]
fn record_limit_on(main: bool) {
    let own = if main { None } else { thread_limit() };
    let limit = own.unwrap_or_else(main_limit);
    LIMIT.with(|kept| *kept = limit);
}

/// Returns the limit of the main thread's stack, which is as large as `RLIMIT_STACK`, below
/// the current stack pointer; the size is read once from `getrlimit`.
#[cfg(target_os = "linux")]
fn main_limit() -> usize {
    let base = stack_pointer();
    let size = SIZE.with(|size| {
        if *size == 0 {
            *size = usize::try_from(crate::libc::stack_limit().unwrap_or(DEFAULT_STACK))
                .unwrap_or(usize::MAX);
        }
        *size
    });
    base.saturating_sub(size.saturating_sub(MARGIN))
}

/// Returns the limit of the calling thread's own stack, which its creator may have made
/// smaller than `RLIMIT_STACK`: read with `pthread_getattr_np` on the thread's first export
/// call and kept for the thread, so `/proc/self/maps`, which glibc reads for the main thread,
/// is never read. `None` when the C library does not give the stack.
#[cfg(target_os = "linux")]
fn thread_limit() -> Option<usize> {
    let key = THREAD_KEY.with(|key| {
        *key = key.or_else(crate::libc::new_thread_key);
        *key
    })?;
    let kept = crate::libc::thread_value(key);
    if kept != 0 {
        return Some(kept);
    }
    let limit = crate::libc::thread_stack_low()?.saturating_add(MARGIN);
    crate::libc::keep_thread_value(key, limit);
    Some(limit)
}

/// Raises `RecursionError` when the stack is within the margin of its limit.
#[unsafe(no_mangle)]
pub extern "C" fn typy_rt_check_stack() -> *mut ExceptionObject {
    if stack_pointer() < LIMIT.with(|limit| *limit) {
        raise(
            ExceptionKind::RecursionError,
            "maximum recursion depth exceeded",
        )
    } else {
        Raising::ok(()).exception
    }
}

#[cfg(test)]
mod tests {
    use super::{record_limit, typy_rt_check_stack};

    #[test]
    #[cfg_attr(
        miri,
        ignore = "Miri cannot run the inline assembly that reads the stack pointer"
    )]
    fn a_stack_past_its_limit_raises_recursion_error() {
        super::LIMIT.with(|limit| *limit = usize::MAX);
        let exception = typy_rt_check_stack();
        super::LIMIT.with(|limit| *limit = 0);
        assert!(!exception.is_null());
        // SAFETY: the exception was just raised and nothing else refers to it.
        unsafe {
            assert_eq!((*exception).kind(), crate::ExceptionKind::RecursionError);
            crate::refcount::typy_rt_decrement(exception.cast());
        }
    }

    #[test]
    #[cfg_attr(miri, ignore = "Miri cannot call `getrlimit`")]
    fn a_shallow_stack_passes_the_check() {
        record_limit();
        assert!(typy_rt_check_stack().is_null());
    }

    #[test]
    #[cfg_attr(miri, ignore = "Miri cannot read a thread's stack")]
    fn the_limit_lies_within_a_small_threads_own_stack() {
        const SIZE: usize = 256 * 1024;
        let (stack, limit, again) = std::thread::Builder::new()
            .stack_size(SIZE)
            .spawn(|| {
                record_limit();
                let limit = super::LIMIT.with(|limit| *limit);
                // A second export call on the thread finds the limit it kept.
                record_limit();
                (
                    super::stack_pointer(),
                    limit,
                    super::LIMIT.with(|limit| *limit),
                )
            })
            .unwrap()
            .join()
            .unwrap();
        assert!(limit < stack, "{limit:#x} {stack:#x}");
        assert!(stack - limit < SIZE, "{limit:#x} {stack:#x}");
        assert_eq!(again, limit);
    }

    #[test]
    #[cfg(target_os = "linux")]
    #[cfg_attr(miri, ignore = "Miri cannot call `getrlimit`")]
    fn the_main_threads_limit_is_rlimit_stack_below_the_stack_pointer() {
        super::record_limit_on(true);
        let (stack, limit) = (super::stack_pointer(), super::LIMIT.with(|limit| *limit));
        let size =
            usize::try_from(crate::libc::stack_limit().unwrap_or(super::DEFAULT_STACK)).unwrap();
        // The limit was taken a few frames deeper than `stack`, so it lies at least that far
        // below it.
        assert!(
            limit <= stack.saturating_sub(size.saturating_sub(super::MARGIN)),
            "{limit:#x} {stack:#x}"
        );
        assert!(typy_rt_check_stack().is_null());
    }
}
