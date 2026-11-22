//! Tests of `repr` of a string.

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
