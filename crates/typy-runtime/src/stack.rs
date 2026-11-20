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
/// thread (ADR-0044). On Linux a thread's stack is as large as `RLIMIT_STACK` unless its
/// creator chose otherwise, so the limit is read once from `getrlimit`.
#[cfg(target_os = "linux")]
pub(crate) fn record_limit() {
    let base = stack_pointer();
    let size = SIZE.with(|size| {
        if *size == 0 {
            *size = usize::try_from(crate::libc::stack_limit().unwrap_or(DEFAULT_STACK))
                .unwrap_or(usize::MAX);
        }
        *size
    });
    LIMIT.with(|limit| *limit = base.saturating_sub(size.saturating_sub(MARGIN)));
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
    #[cfg(target_os = "macos")]
    fn the_limit_lies_within_a_small_threads_own_stack() {
        const SIZE: usize = 256 * 1024;
        let (stack, limit) = std::thread::Builder::new()
            .stack_size(SIZE)
            .spawn(|| {
                record_limit();
                (super::stack_pointer(), super::LIMIT.with(|limit| *limit))
            })
            .unwrap()
            .join()
            .unwrap();
        assert!(limit < stack, "{limit:#x} {stack:#x}");
        assert!(stack - limit < SIZE, "{limit:#x} {stack:#x}");
    }
}
