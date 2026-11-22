//! Tests of magnitude arithmetic against Rust's `u128` and against identities on larger values.

use super::{
    add, any_low_bits, bit_length, compare, divide, multiply, multiply_add_small, shift_left,
    shift_right, subtract,
};

fn value(limbs: &[u64]) -> u128 {
    limbs
        .iter()
        .rev()
        .fold(0_u128, |total, limb| (total << 64) | u128::from(*limb))
}

fn limbs(value: u128) -> Vec<u64> {
    let mut result = vec![
        u64::try_from(value & u128::from(u64::MAX)).unwrap(),
        u64::try_from(value >> 64).unwrap(),
    ];
    while result.last() == Some(&0) {
        result.pop();
    }
    result
}

const SAMPLES: [u128; 9] = [
    0,
    1,
    2,
    7,
    u64::MAX as u128,
    1 << 64,
    (1 << 64) + 12345,
    u128::MAX / 3,
    u128::MAX >> 1,
];

#[test]
fn small_values_agree_with_u128() {
    for a in SAMPLES {
        for b in SAMPLES {
            if let Some(sum) = a.checked_add(b) {
                assert_eq!(value(add(&limbs(a), &limbs(b)).as_slice()), sum);
            }
            if a >= b {
                assert_eq!(value(subtract(&limbs(a), &limbs(b)).as_slice()), a - b);
            }
            if let Some(product) = a.checked_mul(b) {
                assert_eq!(value(multiply(&limbs(a), &limbs(b)).as_slice()), product);
            }
            if b != 0 {
                let (quotient, remainder) = divide(&limbs(a), &limbs(b));
                assert_eq!(
                    (value(quotient.as_slice()), value(remainder.as_slice())),
                    (a / b, a % b),
                    "{a} / {b}"
                );
            }
            assert_eq!(compare(&limbs(a), &limbs(b)), a.cmp(&b));
        }
    }
}

#[test]
fn division_of_large_values_satisfies_the_division_identity() {
    let dividend = multiply(&[u64::MAX, 3, 9, 1 << 63], &[12345, u64::MAX - 7]);
    for divisor in [
        vec![3_u64],
        vec![0, 1],
        vec![u64::MAX, u64::MAX],
        vec![5, 6, 7],
        vec![1, 0, 0, 1 << 62],
    ] {
        let (quotient, remainder) = divide(dividend.as_slice(), &divisor);
        assert_eq!(
            compare(remainder.as_slice(), &divisor),
            core::cmp::Ordering::Less
        );
        let rebuilt = add(
            multiply(quotient.as_slice(), &divisor).as_slice(),
            remainder.as_slice(),
        );
        assert_eq!(rebuilt.as_slice(), dividend.as_slice(), "{divisor:?}");
    }
}

#[test]
fn shifts_and_bit_lengths_agree_with_u128() {
    for a in SAMPLES {
        for bits in [0_u64, 1, 13, 63, 64, 65, 100] {
            if bits < 128 && u64::from(a.leading_zeros()) >= bits {
                assert_eq!(value(shift_left(&limbs(a), bits).as_slice()), a << bits);
            }
            if bits < 128 {
                assert_eq!(value(shift_right(&limbs(a), bits).as_slice()), a >> bits);
                assert_eq!(
                    any_low_bits(&limbs(a), bits),
                    bits > 0 && a & ((1_u128 << bits) - 1) != 0
                );
            }
        }
        assert_eq!(bit_length(&limbs(a)), 128 - u64::from(a.leading_zeros()));
        assert_eq!(
            value(multiply_add_small(&limbs(a / 20), 10, 9).as_slice()),
            (a / 20) * 10 + 9
        );
    }
}

#[test]
fn division_cases_that_correct_the_quotient_estimate_satisfy_the_identity() {
    // The Hacker's Delight test cases for Knuth's algorithm D, scaled to 64-bit limbs: the
    // first estimates a quotient digit of the base or more, the second must add the divisor
    // back after subtracting one product too many.
    let cases: [(&[u64], &[u64]); 3] = [
        (&[u64::MAX - 1, 0, 1 << 63], &[u64::MAX, 1 << 63]),
        (&[0, 0, 1 << 63, (1 << 63) - 1], &[1, 0, 1 << 63]),
        (&[0, 0, 0, u64::MAX], &[1, 0, u64::MAX]),
    ];
    for (dividend, divisor) in cases {
        let (quotient, remainder) = divide(dividend, divisor);
        assert_eq!(
            compare(remainder.as_slice(), divisor),
            core::cmp::Ordering::Less
        );
        let rebuilt = add(
            multiply(quotient.as_slice(), divisor).as_slice(),
            remainder.as_slice(),
        );
        assert_eq!(rebuilt.as_slice(), dividend, "{dividend:?} / {divisor:?}");
    }
}

/// Returns the next value of a fixed pseudo-random sequence, so every run checks the same cases.
fn next(state: &mut u64) -> u64 {
    *state = state
        .wrapping_mul(6_364_136_223_846_793_005)
        .wrapping_add(1_442_695_040_888_963_407);
    *state ^ (*state >> 29)
}

#[test]
fn a_reciprocal_divides_two_limbs_as_a_128_bit_division_does() {
    let mut state = 7;
    let mut divisors = vec![1_u64 << 63, (1 << 63) + 1, u64::MAX, u64::MAX - 1];
    divisors.extend((0..200).map(|_| next(&mut state) | (1 << 63)));
    for divisor in divisors {
        let reciprocal = super::Reciprocal::of(divisor);
        let mut highs = vec![0, 1, divisor - 1, divisor / 2];
        highs.extend((0..20).map(|_| next(&mut state) % divisor));
        for high in highs {
            for low in [0, 1, u64::MAX, next(&mut state)] {
                let value = (u128::from(high) << 64) | u128::from(low);
                let expected = (
                    u64::try_from(value / u128::from(divisor)).unwrap(),
                    u64::try_from(value % u128::from(divisor)).unwrap(),
                );
                assert_eq!(
                    reciprocal.divide(high, low),
                    expected,
                    "{high} {low} {divisor}"
                );
            }
        }
    }
}

#[test]
fn long_division_gives_a_quotient_and_remainder_that_recompose_the_dividend() {
    let mut state = 11;
    for (dividend_length, divisor_length) in [(1, 1), (3, 1), (8, 2), (16, 8), (20, 3), (33, 17)] {
        for round in 0..40 {
            let dividend: Vec<u64> = (0..dividend_length).map(|_| next(&mut state)).collect();
            let mut divisor: Vec<u64> = (0..divisor_length).map(|_| next(&mut state)).collect();
            // Small top limbs and divisors equal to the top of the dividend reach the rare
            // corrections.
            if round % 4 == 0 {
                divisor[divisor_length - 1] = 1 + round;
            }
            if round % 5 == 0 && divisor_length <= dividend_length {
                divisor[divisor_length - 1] = dividend[dividend_length - 1].max(1);
            }
            let (quotient, remainder) = super::divide(&dividend, &divisor);
            assert_eq!(
                super::compare(remainder.as_slice(), &divisor),
                core::cmp::Ordering::Less
            );
            let product = super::multiply(quotient.as_slice(), &divisor);
            let recomposed = super::add(product.as_slice(), remainder.as_slice());
            let mut trimmed = dividend.clone();
            while trimmed.last() == Some(&0) {
                trimmed.pop();
            }
            assert_eq!(
                recomposed.as_slice(),
                trimmed.as_slice(),
                "{dividend:?} {divisor:?}"
            );
        }
    }
    for divisor in [1, 2, 3, 10, u64::MAX, 1 << 63] {
        let value = [u64::MAX, 12_345, 1 << 40];
        let (quotient, remainder) = super::divide_small(&value, divisor);
        let recomposed = super::multiply_add_small(quotient.as_slice(), divisor, remainder);
        assert_eq!(recomposed.as_slice(), value, "{divisor}");
        assert!(remainder < divisor);
    }
}
