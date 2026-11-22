//! The check `range` makes of its step, the last value of a range, which `reversed` starts
//! from, and `range` as a value (DAT-17).

// `start` and `stop` are the names CPython and Python give a range's bounds.
#![allow(
    clippy::similar_names,
    reason = "start and stop are a range's own names"
)]

pub(crate) mod object;

pub(crate) use object::{RANGE, count, destroy, fits_long, item};

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
    // SAFETY: the caller passes live words; each new word made here is released once used.
    unsafe {
        let count = length(start, end, step);
        let before = crate::int::typy_rt_int_subtract(count, word(1));
        let offset = crate::int::typy_rt_int_multiply(before, step);
        let last = typy_rt_int_add(start, offset);
        for used in [count, before, offset] {
            typy_rt_int_decrement(used);
        }
        last
    }
}

/// Returns the number of values of `range(start, stop, step)`, a new `int` word, as CPython's
/// `compute_range_length` does: `(high - low - 1) // |step| + 1`, or 0.
///
/// # Safety
///
/// Each argument is an `int` word whose large integer is live, and `step` is not zero.
pub(crate) unsafe fn length(start: i64, stop: i64, step: i64) -> i64 {
    let word = |value: i64| inline(value).unwrap_or(1);
    let (less, greater) = (word(2), word(4));
    // SAFETY: the caller passes live words; each new word made here is released once used.
    unsafe {
        let rising = typy_rt_int_compare(step, word(0), greater);
        let (low, high) = if rising { (start, stop) } else { (stop, start) };
        if !typy_rt_int_compare(low, high, less) {
            return word(0);
        }
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
    }
}

#[cfg(test)]
mod tests;
