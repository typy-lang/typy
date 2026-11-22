//! Tests of reading code points and recognising whitespace.

use super::{Points, is_space};
use crate::string::from_str;

#[test]
fn points_are_read_by_index() {
    // SAFETY: the string is live for the test.
    let points = unsafe { Points::of(from_str("a\u{e9}")) };
    assert_eq!(
        (points.len(), points.get(0), points.get(1)),
        (2, 0x61, 0xe9)
    );
}

#[test]
fn whitespace_matches_str_isspace() {
    for point in [
        0x20, 0x09, 0x0a, 0x0b, 0x0c, 0x0d, 0x1c, 0x1f, 0x85, 0xa0, 0x3000,
    ] {
        assert!(is_space(point), "{point:#x}");
    }
    for point in [0x41, 0x5f, 0x200b, 0xfeff] {
        assert!(!is_space(point), "{point:#x}");
    }
}
