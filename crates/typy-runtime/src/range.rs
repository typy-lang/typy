//! The check `range` makes of its step, and the last value of a range, which `reversed`
//! starts from.

use crate::exception::{ExceptionKind, ExceptionObject, raise};
use crate::int::{typy_rt_int_add, typy_rt_int_compare};
use crate::refcount::typy_rt_int_decrement;
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

/// Returns `start + (n - 1) * step` for the `n` values of `range(start, stop, step)`, whose
/// step is not zero: the last value, or `start - step` for an empty range, so `reversed` reads
/// `range(last, start - step, -step)` as CPython's `range_reverse` builds it. Each argument is an
/// `int` word the caller keeps; the result is a new one.
///
/// # Safety
///
/// Each argument is an `int` word whose large integer is live.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn typy_rt_range_last(start: i64, end: i64, step: i64) -> i64 {
    let word = |value: i64| inline(value).unwrap_or(1);
    let (less, greater) = (word(2), word(4));
    // SAFETY: the caller passes live words; each new word made here is released once used.
    unsafe {
        let rising = typy_rt_int_compare(step, word(0), greater);
        let (low, high) = if rising { (start, end) } else { (end, start) };
        let count = if typy_rt_int_compare(low, high, less) {
            // CPython's `compute_range_length`: (high - low - 1) // |step| + 1.
            let span = crate::int::typy_rt_int_subtract(high, low);
            let gap = crate::int::typy_rt_int_subtract(span, word(1));
            let size = if rising {
                step
            } else {
                crate::int::typy_rt_int_negate(step)
            };
            let steps = crate::int::typy_rt_int_floor_divide(gap, size).value;
            for used in [span, gap] {
                typy_rt_int_decrement(used);
            }
            if !rising {
                typy_rt_int_decrement(size);
            }
            let count = typy_rt_int_add(steps, word(1));
            typy_rt_int_decrement(steps);
            count
        } else {
            word(0)
        };
        let before = crate::int::typy_rt_int_subtract(count, word(1));
        let offset = crate::int::typy_rt_int_multiply(before, step);
        let last = typy_rt_int_add(start, offset);
        for used in [count, before, offset] {
            typy_rt_int_decrement(used);
        }
        last
    }
}

#[cfg(test)]
mod tests {
    use super::{typy_rt_range_check_step, typy_rt_range_last};
    use crate::int::value::Integer;

    #[test]
    fn the_last_value_of_a_range_is_cpythons() {
        let cases: [(i128, i128, i128, i128); 9] = [
            (0, 5, 1, 4),
            (0, 10, 3, 9),
            (10, 0, -3, 1),
            (5, 5, 1, 4),
            (5, 0, 1, 4),
            (0, 5, -1, 1),
            (-3, 4, 2, 3),
            (0, 1 << 80, 1 << 70, 1023 << 70),
            (1 << 70, 0, -(1 << 69), 1 << 69),
        ];
        for (start, stop, step, last) in cases {
            let words = [start, stop, step].map(|value| Integer::from_i128(value).into_word());
            // SAFETY: the words are live; each is released once.
            unsafe {
                let found = typy_rt_range_last(words[0], words[1], words[2]);
                let expected = Integer::from_i128(last).into_word();
                assert!(
                    crate::int::typy_rt_int_compare(found, expected, inline(0).unwrap()),
                    "range({start}, {stop}, {step})"
                );
                for word in [found, expected].into_iter().chain(words) {
                    crate::refcount::typy_rt_int_decrement(word);
                }
            }
        }
    }
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
