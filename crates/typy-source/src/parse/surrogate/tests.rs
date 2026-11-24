//! Tests of `parse::surrogate`.

use super::replacement_sources;

#[test]
fn each_item_that_gives_a_replacement_is_found_in_order() {
    let cases: &[(&str, &[Option<u32>])] = &[
        ("a\\ud800b", &[Some(0xd800)]),
        ("\\U0000dfff\\ufffd\u{fffd}", &[Some(0xdfff), None, None]),
        ("\\N{replacement character}\\udc80", &[None, Some(0xdc80)]),
        // An escaped backslash, an escape of another code point, and digits that are not
        // hexadecimal give no replacement.
        ("\\\\ud800 \\u00e9 \\u+d80 \\N{BULLET}", &[]),
        ("\\ud8", &[]),
        ("ends with \\", &[]),
    ];
    for (text, expected) in cases {
        assert_eq!(replacement_sources(text), *expected, "{text}");
    }
}
