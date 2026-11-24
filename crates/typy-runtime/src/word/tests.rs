//! Tests of `int` words.

use super::{INLINE_MAX, INLINE_MIN, inline, inline_value, is_inline, small};

#[test]
fn inline_words_round_trip_at_the_limits() {
    for value in [0, 1, -1, 42, INLINE_MIN, INLINE_MAX] {
        let word = inline(value).unwrap();
        assert!(is_inline(word));
        assert_eq!(inline_value(word), value);
    }
    assert_eq!(inline(INLINE_MAX + 1), None);
    assert_eq!(inline(INLINE_MIN - 1), None);
    assert_eq!(small(3), 7);
    assert!(!is_inline(8));
}
