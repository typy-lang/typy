//! The stack check at the start of recursive functions, which raises `RecursionError` before the stack runs out (FUN-02).

use crate::Raising;
use crate::cell::ProgramCell;
use crate::exception::{ExceptionKind, ExceptionObject, raise};

/// The stack left unused when the check fires, for the code that reports the error.
const MARGIN: usize = 64 * 1024;
/// The stack size assumed when the system sets no limit.
const DEFAULT_STACK: u64 = 8 * 1024 * 1024;

static LIMIT: ProgramCell<usize> = ProgramCell::new(0);

/// Returns the stack pointer. The register is read, not the address of a local, because a
/// sanitizer may keep locals on a separate fake stack, where the check would never fire.
#[inline]
fn stack_pointer() -> usize {
    let pointer: usize;
    // SAFETY: copying `rsp` into a register reads no memory and changes nothing else.
    unsafe {
        core::arch::asm!(
            "mov {}, rsp",
            out(reg) pointer,
            options(nomem, nostack, preserves_flags)
        );
    }
    pointer
}

/// Records the lowest address the stack may reach; `main` calls it first.
pub(crate) fn record_limit() {
    let base = stack_pointer();
    let size =
        usize::try_from(crate::libc::stack_limit().unwrap_or(DEFAULT_STACK)).unwrap_or(usize::MAX);
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
    #[cfg_attr(miri, ignore = "Miri cannot call `getrlimit`")]
    fn a_shallow_stack_passes_the_check() {
        record_limit();
        assert!(typy_rt_check_stack().is_null());
    }
}
