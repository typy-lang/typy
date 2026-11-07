//! `repr` of a string: quoted, with the escapes Python uses (LIB-01).

use super::{StrObject, code_point, length};

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
        _ if super::printable::is_printable(point) => push(point),
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
    use super::repr_into;
    use crate::string::from_str;

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
            ("", "''"),
        ];
        for (text, expected) in cases {
            assert_eq!(repr(text), expected, "{text:?}");
        }
    }
}
