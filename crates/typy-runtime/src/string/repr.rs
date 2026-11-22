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
mod tests;
