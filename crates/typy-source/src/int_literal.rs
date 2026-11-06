//! Converts Python integer literals of any size to decimal digits (LEX-03, DAT-01).

use std::fmt::Write as _;

/// Returns the decimal digits of a literal such as `0x_ff` or `1_000`, or `None` when it is malformed.
pub(crate) fn decimal_digits(literal: &str) -> Option<String> {
    let cleaned: String = literal.chars().filter(|c| *c != '_').collect();
    let lower = cleaned.to_ascii_lowercase();
    let (radix, digits) = match lower.get(..2) {
        Some("0x") => (16, &lower[2..]),
        Some("0o") => (8, &lower[2..]),
        Some("0b") => (2, &lower[2..]),
        _ => (10, lower.as_str()),
    };
    if digits.is_empty() {
        return None;
    }
    // Little-endian limbs of nine decimal digits each.
    let mut limbs: Vec<u64> = vec![0];
    for character in digits.chars() {
        let mut carry = u64::from(character.to_digit(radix)?);
        for limb in &mut limbs {
            let value = *limb * u64::from(radix) + carry;
            *limb = value % 1_000_000_000;
            carry = value / 1_000_000_000;
        }
        while carry > 0 {
            limbs.push(carry % 1_000_000_000);
            carry /= 1_000_000_000;
        }
    }
    let mut text = limbs.last().copied().unwrap_or(0).to_string();
    for limb in limbs.iter().rev().skip(1) {
        let _ = write!(text, "{limb:09}");
    }
    Some(text)
}

#[cfg(test)]
mod tests {
    use super::decimal_digits;
    use proptest::prelude::*;

    #[test]
    fn literals_in_every_base_give_their_decimal_value() {
        let cases = [
            ("0", "0"),
            ("000", "0"),
            ("1_000_000", "1000000"),
            ("0xff", "255"),
            ("0XFF", "255"),
            ("0o17", "15"),
            ("0b1010", "10"),
            ("0x_1", "1"),
            ("18446744073709551616", "18446744073709551616"),
            ("0x1_0000_0000_0000_0000_0000", "1208925819614629174706176"),
        ];
        for (literal, expected) in cases {
            assert_eq!(
                decimal_digits(literal).as_deref(),
                Some(expected),
                "{literal}"
            );
        }
    }

    #[test]
    fn malformed_literals_are_refused() {
        for literal in ["", "0x", "0b2", "0o8", "12a", "0x_"] {
            assert_eq!(decimal_digits(literal), None, "{literal:?}");
        }
    }

    proptest! {
        #[test]
        fn hexadecimal_and_decimal_forms_agree(value in any::<u128>()) {
            let expected = value.to_string();
            prop_assert_eq!(decimal_digits(&format!("{value:#x}")), Some(expected.clone()));
            prop_assert_eq!(decimal_digits(&format!("{value:#b}")), Some(expected.clone()));
            prop_assert_eq!(decimal_digits(&expected), Some(expected));
        }
    }
}
