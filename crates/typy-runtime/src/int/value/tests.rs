//! Tests of signed integers of any size.

use super::Integer;

const SAMPLES: [i128; 14] = [
    0,
    1,
    -1,
    7,
    -7,
    1 << 62,
    -(1 << 62) - 1,
    i64::MAX as i128,
    i64::MIN as i128,
    1 << 100,
    -(1 << 100) + 3,
    i128::MAX / 2,
    i128::MAX,
    -i128::MAX,
];

#[test]
fn values_round_trip_through_words() {
    for value in SAMPLES {
        let word = Integer::from_i128(value).into_word();
        // SAFETY: the word was just made, and a large integer it points to is live.
        assert_eq!(unsafe { Integer::from_word(word) }.to_i128(), Some(value));
    }
}

#[test]
fn arithmetic_and_floor_division_agree_with_python_rules() {
    for a in SAMPLES {
        for b in SAMPLES {
            let (x, y) = (Integer::from_i128(a), Integer::from_i128(b));
            if let Some(sum) = a.checked_add(b) {
                assert_eq!(x.add(&y).to_i128(), Some(sum));
            }
            if let Some(difference) = a.checked_sub(b) {
                assert_eq!(x.subtract(&y).to_i128(), Some(difference));
            }
            if let Some(product) = a.checked_mul(b) {
                assert_eq!(x.multiply(&y).to_i128(), Some(product));
            }
            if b != 0 {
                let (quotient, remainder) = x.divide_floor(&y);
                assert_eq!(
                    quotient.to_i128(),
                    Some(a.div_euclid(b) - i128::from(b < 0 && a.rem_euclid(b) != 0)),
                    "{a} // {b}"
                );
                let expected = a.rem_euclid(b) + if b < 0 && a.rem_euclid(b) != 0 { b } else { 0 };
                assert_eq!(remainder.to_i128(), Some(expected), "{a} % {b}");
            }
            assert_eq!(x.compare(&y), a.cmp(&b));
        }
    }
}

#[test]
fn zero_is_never_negative() {
    let zero = Integer::from_i128(0).negate();
    assert!(!zero.negative);
    assert!(zero.is_zero());
    assert!(
        !Integer::from_i128(5)
            .subtract(&Integer::from_i128(5))
            .negative
    );
}
