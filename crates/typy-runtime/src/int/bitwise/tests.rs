//! Tests of shifts and bitwise operations on `int`.

use super::{
    typy_rt_int_and, typy_rt_int_invert, typy_rt_int_left_shift, typy_rt_int_or,
    typy_rt_int_right_shift, typy_rt_int_xor,
};
use crate::int::value::Integer;

fn word(value: i128) -> i64 {
    Integer::from_i128(value).into_word()
}

fn value(word: i64) -> i128 {
    // SAFETY: every word here is made by `Integer::into_word`.
    unsafe { Integer::from_word(word) }.to_i128().unwrap()
}

const SAMPLES: [i128; 10] = [
    0,
    1,
    -1,
    6,
    -6,
    3,
    (1 << 62) - 1,
    -(1 << 62),
    1 << 80,
    -(1 << 81) + 5,
];

#[test]
fn bitwise_operations_match_twos_complement_on_i128() {
    for a in SAMPLES {
        for b in SAMPLES {
            // SAFETY: the words are live `int` words.
            unsafe {
                assert_eq!(value(typy_rt_int_and(word(a), word(b))), a & b, "{a} & {b}");
                assert_eq!(value(typy_rt_int_or(word(a), word(b))), a | b, "{a} | {b}");
                assert_eq!(value(typy_rt_int_xor(word(a), word(b))), a ^ b, "{a} ^ {b}");
            }
        }
        // SAFETY: the word is a live `int` word.
        assert_eq!(value(unsafe { typy_rt_int_invert(word(a)) }), !a, "~{a}");
    }
}

#[test]
fn shifts_round_toward_negative_infinity() {
    let cases: [(i128, u32); 7] = [
        (1, 0),
        (1, 70),
        (-1, 3),
        (5, 1),
        (-5, 1),
        (-(1 << 80) - 1, 70),
        (1 << 90, 100),
    ];
    for (a, bits) in cases {
        // SAFETY: the words are live `int` words.
        unsafe {
            if bits < 40 || a.unsigned_abs() < 1 << 40 {
                assert_eq!(
                    value(typy_rt_int_left_shift(word(a), word(i128::from(bits))).value),
                    a << bits,
                    "{a} << {bits}"
                );
            }
            assert_eq!(
                value(typy_rt_int_right_shift(word(a), word(i128::from(bits))).value),
                a >> bits.min(127),
                "{a} >> {bits}"
            );
        }
    }
}

#[test]
fn negative_and_huge_shift_counts_follow_python() {
    // SAFETY: the words are live `int` words, and the exceptions are live.
    unsafe {
        let negative = typy_rt_int_left_shift(word(1), word(-1));
        assert_eq!(
            (*negative.exception).kind(),
            crate::ExceptionKind::ValueError
        );
        assert!(
            !typy_rt_int_right_shift(word(1), word(-1))
                .exception
                .is_null()
        );
        let huge = typy_rt_int_left_shift(word(1), word(1 << 70));
        assert_eq!(
            (*huge.exception).kind(),
            crate::ExceptionKind::OverflowError
        );
        assert_eq!(
            value(typy_rt_int_left_shift(word(0), word(1 << 70)).value),
            0
        );
        assert_eq!(
            value(typy_rt_int_right_shift(word(-7), word(1 << 70)).value),
            -1
        );
        assert_eq!(
            value(typy_rt_int_right_shift(word(7), word(1 << 70)).value),
            0
        );
    }
}
