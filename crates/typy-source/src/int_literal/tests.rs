//! Tests of `int_literal`.

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
