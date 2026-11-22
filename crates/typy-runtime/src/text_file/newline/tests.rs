//! Tests of the `newline` argument of `open`.

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
