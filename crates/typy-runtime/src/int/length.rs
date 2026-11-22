//! Checks the `int` a class's `__len__` returns, as CPython's `len()` and truth tests check it
//! (CLS-07).

use super::value::Integer;
use crate::Raising;
use crate::exception::{ExceptionKind, raise};

/// Returns the `int` word `__len__` returned when it is a length: a negative one raises
/// `ValueError`, and one past `sys.maxsize` `OverflowError`, with CPython's messages.
///
/// # Safety
///
/// `word` is a live `int` word.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn typy_rt_int_length(word: i64) -> Raising<i64> {
    // SAFETY: the caller passes a live `int` word.
    let value = unsafe { Integer::from_word(word) };
    if value.negative {
        return Raising::error(
            1,
            raise(ExceptionKind::ValueError, "__len__() should return >= 0"),
        );
    }
    if value
        .to_i128()
        .is_none_or(|length| length > i128::from(i64::MAX))
    {
        return Raising::error(
            1,
            raise(
                ExceptionKind::OverflowError,
                "cannot fit 'int' into an index-sized integer",
            ),
        );
    }
    Raising::ok(word)
}

#[cfg(test)]
mod tests;
