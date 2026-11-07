//! The stack check at the start of recursive functions, which raises `RecursionError` before the stack runs out (FUN-02).

use crate::Raising;
use crate::cell::ProgramCell;
use crate::exception::{ExceptionKind, ExceptionObject, raise};

/// The stack left unused when the check fires, for the code that reports the error.
const MARGIN: usize = 64 * 1024;
/// The stack size assumed when the system sets no limit.
const DEFAULT_STACK: u64 = 8 * 1024 * 1024;

static LIMIT: ProgramCell<usize> = ProgramCell::new(0);

/// Records the lowest address the stack may reach; `main` calls it first.
pub(crate) fn record_limit() {
    let marker = 0_u8;
    let base = core::ptr::addr_of!(marker) as usize;
    let size =
        usize::try_from(crate::libc::stack_limit().unwrap_or(DEFAULT_STACK)).unwrap_or(usize::MAX);
    LIMIT.with(|limit| *limit = base.saturating_sub(size.saturating_sub(MARGIN)));
}

/// Raises `RecursionError` when the stack is within the margin of its limit.
#[unsafe(no_mangle)]
pub extern "C" fn typy_rt_check_stack() -> *mut ExceptionObject {
    let marker = 0_u8;
    let here = core::ptr::addr_of!(marker) as usize;
    if here < LIMIT.with(|limit| *limit) {
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
    fn a_shallow_stack_passes_the_check() {
        record_limit();
        assert!(typy_rt_check_stack().is_null());
    }
}
