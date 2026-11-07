//! Tests of magnitude arithmetic against Rust's `u128` and against identities on larger values.

use super::{
    add, bit_length, compare, divide, multiply, multiply_add_small, shift_left, shift_right,
    subtract,
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
            }
        }
        assert_eq!(bit_length(&limbs(a)), 128 - u64::from(a.leading_zeros()));
        assert_eq!(
            value(multiply_add_small(&limbs(a / 20), 10, 9).as_slice()),
            (a / 20) * 10 + 9
        );
    }
}
