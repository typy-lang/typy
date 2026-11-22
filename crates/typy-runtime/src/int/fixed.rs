//! Converts `int` words to and from C's `int64_t`, for the exports of a library (FFI-05,
//! ADR-0032).

use crate::Raising;
use crate::exception::{ExceptionKind, raise};
use crate::word::{inline_value, is_inline};

use super::value::Integer;

/// Returns the `int` word of a C `int64_t`, which the caller owns.
#[unsafe(no_mangle)]
pub extern "C" fn typy_rt_int_from_int64(value: i64) -> i64 {
    Integer::from_i128(i128::from(value)).into_word()
}

/// Returns an `int` word as a C `int64_t` and drops the caller's reference to it, or raises
/// `OverflowError` when the value does not fit.
///
/// # Safety
///
/// `word` is an `int` word the caller owns.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn typy_rt_int_into_int64(word: i64) -> Raising<i64> {
    if is_inline(word) {
        return Raising::ok(inline_value(word));
    }
    // SAFETY: a word with its lowest bit clear points to a live large integer, the caller's.
    let value = unsafe { Integer::from_word(word) }
        .to_i128()
        .and_then(|value| i64::try_from(value).ok());
    // SAFETY: the caller owns the reference, which is dropped once.
    unsafe { crate::refcount::typy_rt_int_decrement(word) };
    match value {
        Some(value) => Raising::ok(value),
        None => Raising::error(
            0,
            raise(
                ExceptionKind::OverflowError,
                "Python int too large to convert to C int64_t",
            ),
        ),
    }
}

#[cfg(test)]
mod tests;
