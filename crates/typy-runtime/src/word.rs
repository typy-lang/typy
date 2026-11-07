//! `int` words: an inline integer has its lowest bit set and holds the value shifted left by one (DAT-01).

/// The smallest inline integer: -2^62.
pub(crate) const INLINE_MIN: i64 = -(1 << 62);
/// The largest inline integer: 2^62 - 1.
pub(crate) const INLINE_MAX: i64 = (1 << 62) - 1;

/// Returns true when the word holds its integer inline.
pub(crate) const fn is_inline(word: i64) -> bool {
    word & 1 == 1
}

/// Returns the integer an inline word holds.
pub(crate) const fn inline_value(word: i64) -> i64 {
    word >> 1
}

/// Returns the inline word of `value`, when it is in the inline range.
pub(crate) const fn inline(value: i64) -> Option<i64> {
    if value >= INLINE_MIN && value <= INLINE_MAX {
        Some((value << 1) | 1)
    } else {
        None
    }
}

/// Returns the inline word of a value known to be in range, such as a length.
pub(crate) fn small(value: usize) -> i64 {
    i64::try_from(value)
        .ok()
        .and_then(inline)
        .unwrap_or_else(|| crate::libc::abort_with(b"typy runtime: a length is out of range\n"))
}

#[cfg(test)]
mod tests {
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
}
