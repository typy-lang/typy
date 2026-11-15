//! The `newline` argument of `open` (LIB-14): how reading ends and translates lines, and how
//! writing translates `\n`, as CPython's `TextIOWrapper` does.

use crate::exception::{ExceptionKind, ExceptionObject};
use crate::message::MessageBuffer;
use crate::string::{StrBuilder, StrObject, code_point, length};

/// How a file ends and translates lines.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum Newline {
    /// `None`: reading ends a line at `\n`, `\r\n` or `\r` and gives each as `\n`; writing
    /// leaves `\n`, which is `os.linesep` on Linux.
    Universal,
    /// `''`: reading ends a line at `\n`, `\r\n` or `\r` and gives it as it is; writing
    /// translates nothing.
    Untranslated,
    /// `'\n'`: a line ends at `\n`; nothing is translated.
    Lf,
    /// `'\r'`: a line ends at `\r`; writing gives each `\n` as `\r`.
    Cr,
    /// `'\r\n'`: a line ends at `\r\n`; writing gives each `\n` as `\r\n`.
    CrLf,
}

impl Newline {
    /// Returns the newline `value` names, null for `None`, or CPython's
    /// `ValueError: illegal newline value: x`.
    ///
    /// # Safety
    ///
    /// `value` is null or points to a live `str`.
    pub(crate) unsafe fn of(value: *const StrObject) -> Result<Self, *mut ExceptionObject> {
        if value.is_null() {
            return Ok(Self::Universal);
        }
        // SAFETY: the caller's guarantee; every index is below the length.
        let points: [Option<u32>; 3] = unsafe {
            let count = length(value);
            if count > 2 {
                [None, None, Some(0)]
            } else {
                [
                    (count > 0).then(|| code_point(value, 0)),
                    (count > 1).then(|| code_point(value, 1)),
                    None,
                ]
            }
        };
        let (cr, lf) = (Some(u32::from(b'\r')), Some(u32::from(b'\n')));
        match points {
            [None, None, None] => Ok(Self::Untranslated),
            [first, None, None] if first == lf => Ok(Self::Lf),
            [first, None, None] if first == cr => Ok(Self::Cr),
            [first, second, None] if first == cr && second == lf => Ok(Self::CrLf),
            _ => {
                let mut message = MessageBuffer::new();
                message.push_str("illegal newline value: ");
                // SAFETY: the caller's guarantee.
                unsafe { message.push_text(value) };
                Err(message.raise(ExceptionKind::ValueError))
            }
        }
    }

    /// Returns the text a write gives the encoder: `text` with each `\n` written as this
    /// newline says, as a new reference.
    ///
    /// # Safety
    ///
    /// `text` points to a live `str`.
    pub(crate) unsafe fn translated(self, text: *const StrObject) -> *mut StrObject {
        let ending: &[u32] = match self {
            Self::Cr => &[0x0d],
            Self::CrLf => &[0x0d, 0x0a],
            _ => &[],
        };
        // SAFETY: the caller's guarantee; every index is below the length.
        unsafe {
            let count = length(text);
            let has_lf = (0..count).any(|index| code_point(text, index) == 0x0a);
            if ending.is_empty() || !has_lf {
                crate::refcount::typy_rt_increment(text.cast_mut().cast());
                return text.cast_mut();
            }
            let mut out = StrBuilder::with_capacity(count + count / 8);
            for index in 0..count {
                let point = code_point(text, index);
                if point == 0x0a {
                    for each in ending {
                        out.push(*each);
                    }
                } else {
                    out.push(point);
                }
            }
            out.finish()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Newline;
    use crate::string::from_str;

    fn parsed(text: &str) -> Result<Newline, String> {
        let value = from_str(text);
        // SAFETY: the string is live and released once.
        unsafe {
            let found = Newline::of(value).map_err(|error| {
                let class = (*error).class();
                let shown = class.str.unwrap()(error.cast()).value;
                let text = (0..crate::string::length(shown))
                    .map(|index| char::from_u32(crate::string::code_point(shown, index)).unwrap())
                    .collect();
                crate::refcount::typy_rt_decrement(shown.cast());
                crate::refcount::typy_rt_decrement(error.cast());
                text
            });
            crate::refcount::typy_rt_decrement(value.cast());
            found
        }
    }

    #[test]
    fn each_legal_newline_is_read_and_another_is_refused() {
        assert_eq!(
            // SAFETY: null stands for `None`.
            unsafe { Newline::of(core::ptr::null()) },
            Ok(Newline::Universal)
        );
        assert_eq!(parsed(""), Ok(Newline::Untranslated));
        assert_eq!(parsed("\n"), Ok(Newline::Lf));
        assert_eq!(parsed("\r"), Ok(Newline::Cr));
        assert_eq!(parsed("\r\n"), Ok(Newline::CrLf));
        for bad in ["x", "\n\r", "\n\n", "\rx", "\r\n\r", "abcd"] {
            assert_eq!(parsed(bad), Err(format!("illegal newline value: {bad}")));
        }
    }

    #[test]
    fn writing_translates_newlines_only_for_cr_and_crlf() {
        let cases = [
            (Newline::Universal, "a\nb\r", "a\nb\r"),
            (Newline::Untranslated, "a\nb", "a\nb"),
            (Newline::Lf, "a\nb", "a\nb"),
            (Newline::Cr, "x\ny\r\nz", "x\ry\r\rz"),
            (Newline::CrLf, "x\ny\r\nz", "x\r\ny\r\r\nz"),
            (Newline::CrLf, "none", "none"),
        ];
        for (newline, text, expected) in cases {
            let value = from_str(text);
            // SAFETY: the strings are live and released once.
            unsafe {
                let out = newline.translated(value);
                let found: String = (0..crate::string::length(out))
                    .map(|index| char::from_u32(crate::string::code_point(out, index)).unwrap())
                    .collect();
                assert_eq!(found, expected, "{newline:?}");
                crate::refcount::typy_rt_decrement(out.cast());
                crate::refcount::typy_rt_decrement(value.cast());
            }
        }
    }
}
