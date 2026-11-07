//! The check `range` makes of its step.

use crate::exception::{ExceptionKind, ExceptionObject, raise};
use crate::word::inline;

/// Raises `ValueError` for a zero step, as `range()` does.
#[unsafe(no_mangle)]
pub extern "C" fn typy_rt_range_check_step(step: i64) -> *mut ExceptionObject {
    if Some(step) == inline(0) {
        raise(ExceptionKind::ValueError, "range() arg 3 must not be zero")
    } else {
        core::ptr::null_mut()
    }
}

#[cfg(test)]
mod tests {
    use super::typy_rt_range_check_step;
    use crate::word::inline;

    #[test]
    fn only_a_zero_step_raises() {
        assert!(typy_rt_range_check_step(inline(1).unwrap()).is_null());
        assert!(typy_rt_range_check_step(inline(-2).unwrap()).is_null());
        let exception = typy_rt_range_check_step(inline(0).unwrap());
        assert!(!exception.is_null());
        // SAFETY: the exception is live and its reference is dropped here.
        unsafe { crate::refcount::typy_rt_decrement(exception.cast()) };
    }
}
