//! Tests of the growable array of limbs.

use super::Limbs;

#[test]
fn limbs_grow_and_trim_leading_zeros() {
    let mut limbs = Limbs::new();
    assert!(limbs.is_zero());
    for value in 0..10 {
        limbs.push(value);
    }
    limbs.push(0);
    limbs.trim();
    assert_eq!(limbs.as_slice().len(), 10);
    limbs.as_mut_slice()[0] = 7;
    assert_eq!(Limbs::from_slice(&[7, 0, 0]).as_slice(), [7]);
    assert_eq!(Limbs::zeros(3).as_slice(), [0, 0, 0]);
    assert!(Limbs::zeros(2).is_zero());
}
