//! Tests that ratios round once, to nearest and ties to even, across the whole double range.

use super::ratio;
use crate::int::limbs::Limbs;
use crate::int::natural::{multiply_add_small, shift_left};

fn natural(value: u128) -> Limbs {
    Limbs::from_slice(&[
        u64::try_from(value & u128::from(u64::MAX)).unwrap(),
        u64::try_from(value >> 64).unwrap(),
    ])
}

fn power_of_two(bits: u64) -> Limbs {
    shift_left(&[1], bits)
}

fn power_of_ten(count: u32) -> Limbs {
    let mut value = Limbs::from_slice(&[1]);
    for _ in 0..count {
        value = multiply_add_small(value.as_slice(), 10, 0);
    }
    value
}

fn of(numerator: &Limbs, denominator: &Limbs) -> Option<f64> {
    ratio(numerator.as_slice(), denominator.as_slice())
}

/// A deterministic sequence of 64-bit values.
fn samples() -> impl Iterator<Item = u64> {
    let mut state = 0x9e37_79b9_7f4a_7c15_u64;
    core::iter::repeat_with(move || {
        state ^= state << 13;
        state ^= state >> 7;
        state ^= state << 17;
        state
    })
}

#[test]
fn ratios_of_exact_doubles_match_ieee_division() {
    let mut values = samples();
    for _ in 0..2000 {
        let numerator = values.next().unwrap() >> 11;
        let denominator = (values.next().unwrap() >> 11).max(1);
        #[allow(
            clippy::cast_precision_loss,
            reason = "both values have at most 53 bits"
        )]
        let expected = numerator as f64 / denominator as f64;
        let result = of(
            &natural(u128::from(numerator)),
            &natural(u128::from(denominator)),
        );
        assert_eq!(result, Some(expected), "{numerator} / {denominator}");
    }
}

#[test]
fn integers_round_as_rust_converts_them() {
    let mut values = samples();
    let one = natural(1);
    for _ in 0..2000 {
        let value = (u128::from(values.next().unwrap()) << 64) | u128::from(values.next().unwrap());
        let value = value >> (values.next().unwrap() % 100);
        #[allow(
            clippy::cast_precision_loss,
            reason = "the conversion under test rounds"
        )]
        let expected = value as f64;
        assert_eq!(of(&natural(value), &one), Some(expected), "{value}");
    }
}

#[test]
fn ties_round_to_even() {
    let one = natural(1);
    let above = (1_u128 << 53) + 1;
    assert_eq!(of(&natural(above), &one), Some(9_007_199_254_740_992.0));
    let next = (1_u128 << 53) + 3;
    assert_eq!(of(&natural(next), &one), Some(9_007_199_254_740_996.0));
}

#[test]
fn the_extremes_overflow_or_reach_subnormals_and_zero() {
    let one = natural(1);
    assert_eq!(of(&power_of_two(1024), &one), None);
    let largest = shift_left(&[(1 << 53) - 1], 971);
    assert_eq!(of(&largest, &one), Some(f64::MAX));
    let rounds_up = shift_left(&[(1 << 54) - 1], 970);
    assert_eq!(of(&rounds_up, &one), None);
    assert_eq!(of(&one, &power_of_two(1074)), Some(5e-324));
    assert_eq!(of(&one, &power_of_two(1075)), Some(0.0));
    assert_eq!(of(&natural(3), &power_of_two(1075)), Some(1e-323));
    assert_eq!(of(&one, &power_of_ten(400)), Some(0.0));
    assert_eq!(of(&natural(0), &one), Some(0.0));
    assert_eq!(of(&natural(1), &natural(3)), Some(1.0 / 3.0));
    assert_eq!(of(&power_of_ten(30), &one), Some(1e30));
    assert_eq!(of(&natural(1), &power_of_ten(22)), Some(1e-22));
}
