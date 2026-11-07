//! `repr` of a string: quoted, with the escapes Python uses (LIB-01).

use super::{StrKind, StrObject, code_point, length, new_string, set_code_point};

/// Calls `push` with each code point of `repr(text)`.
///
/// # Safety
///
/// `text` points to a live `str` object.
pub(crate) unsafe fn repr_into(text: *const StrObject, mut push: impl FnMut(u32)) {
    // SAFETY: the caller passes a live string, and every index stays below its length.
    let points = || (0..unsafe { length(text) }).map(|index| unsafe { code_point(text, index) });
    let has_single = points().any(|point| point == u32::from(b'\''));
    let has_double = points().any(|point| point == u32::from(b'"'));
    // Python quotes with `'` unless the text holds `'` and no `"`.
    let quote = if has_single && !has_double {
        u32::from(b'"')
    } else {
        u32::from(b'\'')
    };
    push(quote);
    for point in points() {
        escape(point, quote, &mut push);
    }
    push(quote);
}

/// Returns `repr(text)`, a new string owned by the caller.
///
/// # Safety
///
/// `text` points to a live `str` object.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn typy_rt_str_repr(text: *const StrObject) -> *mut StrObject {
    // The first pass sizes the result and picks the narrowest kind; the second writes it.
    let mut count = 0_usize;
    let mut kind = StrKind::Latin1;
    // SAFETY: the caller passes a live string.
    unsafe {
        repr_into(text, |point| {
            count += 1;
            kind = kind.max(StrKind::of(point));
        });
    }
    let result = new_string(count, kind);
    let mut index = 0;
    // SAFETY: the caller passes a live string; the result holds `count` points of `kind`.
    unsafe {
        repr_into(text, |point| {
            set_code_point(result, index, point);
            index += 1;
        });
    }
    result
}

fn escape(point: u32, quote: u32, push: &mut impl FnMut(u32)) {
    let mut push_ascii = |text: &[u8]| text.iter().for_each(|byte| push(u32::from(*byte)));
    match point {
        0x5c => push_ascii(b"\\\\"),
        0x0a => push_ascii(b"\\n"),
        0x0d => push_ascii(b"\\r"),
        0x09 => push_ascii(b"\\t"),
        _ if point == quote => {
            push_ascii(b"\\");
            push(point);
        }
        _ if super::unicode::is_printable(point) => push(point),
        _ => {
            let (prefix, width): (&[u8], u32) = if point < 0x100 {
                (b"\\x", 2)
            } else if point < 0x10000 {
                (b"\\u", 4)
            } else {
                (b"\\U", 8)
            };
            push_ascii(prefix);
            for shift in (0..width).rev() {
                let digit = (point >> (shift * 4)) & 0xf;
                push(u32::from(b"0123456789abcdef"[digit as usize]));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{repr_into, typy_rt_str_repr};
    use crate::string::{StrKind, code_point, from_str, length};

    fn repr(text: &str) -> String {
        let mut out = String::new();
        // SAFETY: the string is live.
        unsafe {
            repr_into(from_str(text), |point| {
                out.push(char::from_u32(point).unwrap());
            });
        };
        out
    }

    #[test]
    fn strings_are_quoted_and_escaped_as_python_does() {
        let cases = [
            ("abc", "'abc'"),
            ("it's", "\"it's\""),
            ("say \"hi\"", "'say \"hi\"'"),
            ("both ' and \"", "'both \\' and \"'"),
            ("tab\tnew\nret\rback\\", "'tab\\tnew\\nret\\rback\\\\'"),
            ("\u{0}\u{7f}", "'\\x00\\x7f'"),
            ("caf\u{e9}", "'caf\u{e9}'"),
            (
                "\u{85}\u{ad}\u{200b}\u{1fae8}\u{e0001}\u{378}\u{10ffff}\u{3000} \u{61c}\u{2028}\u{11f00}",
                "'\\x85\\xad\\u200b\u{1fae8}\\U000e0001\\u0378\\U0010ffff\\u3000 \\u061c\\u2028\u{11f00}'",
            ),
            ("", "''"),
        ];
        for (text, expected) in cases {
            assert_eq!(repr(text), expected, "{text:?}");
        }
    }

    #[test]
    fn the_exported_repr_is_a_new_string_of_the_narrowest_kind() {
        for (text, kind) in [
            ("a'b", StrKind::Latin1),
            ("\u{3b1}", StrKind::Ucs2),
            ("\u{e0001}", StrKind::Latin1),
        ] {
            // SAFETY: the strings are live.
            unsafe {
                let result = typy_rt_str_repr(from_str(text));
                let written: String = (0..length(result))
                    .map(|index| char::from_u32(code_point(result, index)).unwrap())
                    .collect();
                assert_eq!(written, repr(text));
                assert_eq!((*result).kind, kind, "{text:?}");
            }
        }
    }
}
