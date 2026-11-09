//! The binary layout of values and static objects that generated code shares with the runtime (detailed design section 10.2).

/// The reference count of an object that is never freed; the runtime does not write it.
pub const IMMORTAL: u32 = u32::MAX;
/// The type ID of `str` objects.
pub const STR_TYPE_ID: u32 = 1;
/// The type ID of a `bytes` object, a `str` object of one byte per code point.
pub const BYTES_TYPE_ID: u32 = 6;
/// The type ID of exception objects.
pub const EXCEPTION_TYPE_ID: u32 = 2;
/// The type ID of large integers.
pub const BIG_INT_TYPE_ID: u32 = 3;
/// The smallest integer held inline: -2^62.
pub const INLINE_MIN: i128 = -(1 << 62);
/// The largest integer held inline: 2^62 - 1.
pub const INLINE_MAX: i128 = (1 << 62) - 1;

/// How an integer constant is held.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum IntEncoding {
    /// A tagged word: the value shifted left by one, with the lowest bit set.
    Inline(i64),
    /// A large integer: its sign and its magnitude as 64-bit limbs, least significant first.
    Big {
        /// True for a negative value.
        negative: bool,
        /// The magnitude; the last limb is not zero.
        limbs: Vec<u64>,
    },
}

/// Returns the encoding of an integer written as decimal digits with an optional leading `-`.
#[must_use]
pub fn encode_int(text: &str) -> IntEncoding {
    let (negative, digits) = text
        .strip_prefix('-')
        .map_or((false, text), |rest| (true, rest));
    if let Ok(magnitude) = digits.parse::<i128>() {
        let value = if negative { -magnitude } else { magnitude };
        if (INLINE_MIN..=INLINE_MAX).contains(&value) {
            let word = i64::try_from(value).unwrap_or(0);
            return IntEncoding::Inline(word.wrapping_shl(1) | 1);
        }
    }
    let mut limbs: Vec<u64> = vec![0];
    for digit in digits.bytes().filter(u8::is_ascii_digit) {
        let mut carry = u128::from(digit - b'0');
        for limb in &mut limbs {
            let product = u128::from(*limb) * 10 + carry;
            *limb = u64::try_from(product & u128::from(u64::MAX)).unwrap_or(0);
            carry = product >> 64;
        }
        if carry > 0 {
            limbs.push(u64::try_from(carry).unwrap_or(0));
        }
    }
    while limbs.len() > 1 && limbs.last() == Some(&0) {
        limbs.pop();
    }
    IntEncoding::Big { negative, limbs }
}

/// The width of each code point of a string, chosen as the narrowest that holds every code point (PEP 393).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StrKind {
    /// One byte: code points up to U+00FF.
    Latin1 = 1,
    /// Two bytes: code points up to U+FFFF.
    Ucs2 = 2,
    /// Four bytes.
    Ucs4 = 4,
}

/// Returns a string's kind and its code points.
#[must_use]
pub fn encode_str(text: &str) -> (StrKind, Vec<u32>) {
    let points: Vec<u32> = text.chars().map(u32::from).collect();
    let largest = points.iter().copied().max().unwrap_or(0);
    let kind = if largest <= 0xFF {
        StrKind::Latin1
    } else if largest <= 0xFFFF {
        StrKind::Ucs2
    } else {
        StrKind::Ucs4
    };
    (kind, points)
}

#[cfg(test)]
mod tests {
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
}
