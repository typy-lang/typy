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
mod tests {
    use super::{typy_rt_chr, typy_rt_ord};
    use crate::int::value::Integer;
    use crate::string::{code_point, from_str, length};
    use crate::word::{inline, inline_value};

    fn message(exception: *mut crate::ExceptionObject) -> String {
        assert!(!exception.is_null(), "no exception");
        // SAFETY: the exception and its message are live.
        unsafe {
            let text = (*exception).message;
            (0..length(text))
                .map(|index| char::from_u32(code_point(text, index)).unwrap())
                .collect()
        }
    }

    #[test]
    fn chr_and_ord_are_inverse_over_unicode() {
        for point in [
            0_i64, 0x41, 0xff, 0x100, 0xd800, 0xffff, 0x1_0000, 0x10_ffff,
        ] {
            // SAFETY: the word is an `int` word, and the string it returns is live.
            unsafe {
                let text = typy_rt_chr(inline(point).unwrap());
                assert_eq!(length(text.value), 1);
                assert_eq!(inline_value(typy_rt_ord(text.value).value), point);
            }
        }
    }

    #[test]
    fn chr_refuses_values_outside_unicode() {
        let huge = Integer::from_i128(1 << 100).into_word();
        for value in [inline(-1).unwrap(), inline(0x11_0000).unwrap(), huge] {
            // SAFETY: the word is an `int` word.
            let result = unsafe { typy_rt_chr(value) };
            assert_eq!(
                message(result.exception),
                "chr() arg not in range(0x110000)"
            );
        }
    }

    #[test]
    fn ord_refuses_strings_that_are_not_one_character() {
        for (text, count) in [("", 0), ("ab", 2)] {
            // SAFETY: the string is live.
            let result = unsafe { typy_rt_ord(from_str(text)) };
            assert_eq!(
                message(result.exception),
                format!("ord() expected a character, but string of length {count} found")
            );
        }
    }
}
