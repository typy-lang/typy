//! Tests of `layout`.

use super::{INLINE_MAX, INLINE_MIN, IntEncoding, StrKind, encode_int, encode_str};

#[test]
fn small_integers_are_tagged_words() {
    assert_eq!(encode_int("0"), IntEncoding::Inline(1));
    assert_eq!(encode_int("3"), IntEncoding::Inline(7));
    assert_eq!(encode_int("-1"), IntEncoding::Inline(-1));
    assert_eq!(
        encode_int(&INLINE_MAX.to_string()),
        IntEncoding::Inline(i64::MAX)
    );
    assert_eq!(
        encode_int(&INLINE_MIN.to_string()),
        IntEncoding::Inline(i64::MIN + 1)
    );
}

#[test]
fn integers_past_the_inline_range_are_large() {
    assert_eq!(
        encode_int(&(INLINE_MAX + 1).to_string()),
        IntEncoding::Big {
            negative: false,
            limbs: vec![1 << 62]
        }
    );
    assert_eq!(
        encode_int(&(INLINE_MIN - 1).to_string()),
        IntEncoding::Big {
            negative: true,
            limbs: vec![(1 << 62) + 1]
        }
    );
    assert_eq!(
        encode_int("18446744073709551616"),
        IntEncoding::Big {
            negative: false,
            limbs: vec![0, 1]
        }
    );
    assert_eq!(
        encode_int("340282366920938463463374607431768211457"),
        IntEncoding::Big {
            negative: false,
            limbs: vec![1, 0, 1]
        }
    );
}

#[test]
fn strings_use_the_narrowest_kind() {
    assert_eq!(encode_str("abc"), (StrKind::Latin1, vec![97, 98, 99]));
    assert_eq!(encode_str("").0, StrKind::Latin1);
    assert_eq!(encode_str("caf\u{e9}").0, StrKind::Latin1);
    assert_eq!(encode_str("\u{3b1}").0, StrKind::Ucs2);
    assert_eq!(encode_str("\u{1f600}"), (StrKind::Ucs4, vec![0x1f600]));
}
