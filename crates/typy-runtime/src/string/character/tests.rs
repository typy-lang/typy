//! Tests of `chr` and `ord`.

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
