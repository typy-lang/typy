//! The `math` module (LIB-05, ADR-0018): the C maths library behind CPython's checks, and
//! the functions CPython computes itself, with CPython 3.15's results and messages (OI-14).

mod c;
mod integer;
mod log;
mod real;
mod special;
#[cfg(test)]
mod tests;

use crate::Raising;
use crate::exception::ExceptionKind;
use crate::message::MessageBuffer;

/// Returns the integer of an `int` word, inline or large.
///
/// # Safety
///
/// `word` is an `int` word whose large integer, if any, is live.
unsafe fn integer(word: i64) -> crate::int::value::Integer {
    if crate::word::is_inline(word) {
        crate::int::value::Integer::from_i128(i128::from(crate::word::inline_value(word)))
    } else {
        // SAFETY: the caller's guarantee.
        unsafe { crate::int::value::Integer::from_word(word) }
    }
}

/// Returns a result that is not an error.
const fn ok(value: f64) -> Raising<f64> {
    Raising::ok(value)
}

/// Returns `ValueError: math domain error`.
fn domain_error() -> Raising<f64> {
    Raising::error(
        0.0,
        crate::exception::raise(ExceptionKind::ValueError, "math domain error"),
    )
}

/// Returns `OverflowError: math range error`.
fn range_error() -> Raising<f64> {
    Raising::error(
        0.0,
        crate::exception::raise(ExceptionKind::OverflowError, "math range error"),
    )
}

/// Returns `ValueError` with `message`, whose `%s` becomes `repr(x)`, as CPython 3.15 words
/// the domain error of most functions; without a message, `math domain error`.
fn domain_error_of(x: f64, message: Option<&str>) -> Raising<f64> {
    let Some(message) = message else {
        return domain_error();
    };
    let (before, after) = message.split_once("%s").unwrap_or((message, ""));
    let mut text = MessageBuffer::new();
    text.push_str(before);
    let written = crate::float::typy_rt_float_to_str(x);
    // SAFETY: the runtime returned a live string, released once it is copied.
    unsafe {
        text.push_text(written);
        crate::refcount::typy_rt_decrement(written.cast());
    }
    text.push_str(after);
    Raising::error(0.0, text.raise(ExceptionKind::ValueError))
}
