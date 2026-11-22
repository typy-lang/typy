//! Tests of `str` objects.

use super::{StrKind, code_point, from_str, length};

fn points(string: *const super::StrObject) -> Vec<u32> {
    // SAFETY: the string is live and the indexes are in bounds.
    (0..unsafe { length(string) })
        .map(|index| unsafe { code_point(string, index) })
        .collect()
}

#[test]
fn strings_hold_their_code_points_in_the_narrowest_kind() {
    for (text, kind) in [
        ("abc", StrKind::Latin1),
        ("caf\u{e9}", StrKind::Latin1),
        ("\u{3b1}x", StrKind::Ucs2),
        ("a\u{1f600}", StrKind::Ucs4),
        ("", StrKind::Latin1),
    ] {
        let string = from_str(text);
        // SAFETY: the string was just created.
        assert_eq!(unsafe { (*string).kind }, kind);
        assert_eq!(
            points(string),
            text.chars().map(u32::from).collect::<Vec<_>>()
        );
    }
}

#[test]
fn the_fixed_part_takes_thirty_two_bytes() {
    assert_eq!(super::DATA_OFFSET, 32);
}
