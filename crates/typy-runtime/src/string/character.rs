//! `chr` and `ord`: between a code point and a string of one character (LIB-01).

use super::{StrKind, StrObject, code_point, length, new_string, set_code_point};
use crate::Raising;
use crate::exception::{ExceptionKind, raise};
use crate::message::MessageBuffer;
use crate::word::{inline_value, is_inline, small};

const LAST_CODE_POINT: i64 = 0x10_FFFF;

/// Returns `chr(value)`, a new string of one character. A value outside Unicode raises
/// `ValueError`; CPython 3.13 and later say so also for an `int` too large for C (OI-14).
///
/// # Safety
///
/// `value` is an `int` word.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn typy_rt_chr(value: i64) -> Raising<*mut StrObject> {
    let point = is_inline(value)
        .then(|| inline_value(value))
        .filter(|point| (0..=LAST_CODE_POINT).contains(point))
        .and_then(|point| u32::try_from(point).ok());
    let Some(point) = point else {
        return Raising::error(
            core::ptr::null_mut(),
            raise(
                ExceptionKind::ValueError,
                "chr() arg not in range(0x110000)",
            ),
        );
    };
    if let Some(shared) = super::character(point) {
        return Raising::ok(shared);
    }
    let result = new_string(1, StrKind::of(point));
    // SAFETY: the string is new and holds one code point of its kind.
    unsafe { set_code_point(result, 0, point) };
    Raising::ok(result)
}

/// Returns `ord(text)`, the code point of a string of one character.
///
/// # Safety
///
/// `text` points to a live `str` object.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn typy_rt_ord(text: *const StrObject) -> Raising<i64> {
    // SAFETY: the caller passes a live string.
    let count = unsafe { length(text) };
    if count == 1 {
        // SAFETY: the string is live and has one code point.
        let point = unsafe { code_point(text, 0) };
        return Raising::ok(small(point as usize));
    }
    let mut message = MessageBuffer::new();
    message.push_str("ord() expected a character, but string of length ");
    message.push_number(i64::try_from(count).unwrap_or(i64::MAX));
    message.push_str(" found");
    Raising::error(1, message.raise(ExceptionKind::TypeError))
}

#[cfg(test)]
mod tests;
