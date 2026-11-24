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

#[test]
fn limbs_move_from_their_place_to_the_heap_past_1024_bits_and_keep_their_values() {
    let mut limbs = Limbs::new();
    for value in 1..=super::INLINE_LIMBS as u64 {
        limbs.push(value);
    }
    assert!(limbs.heap.is_null());
    limbs.push(17);
    assert!(!limbs.heap.is_null());
    for value in 18..=40 {
        limbs.push(value);
    }
    let expected: Vec<u64> = (1..=40).collect();
    assert_eq!(limbs.as_slice(), expected.as_slice());
    let wide: Vec<u64> = (0..33).map(|value| value * 3 + 1).collect();
    assert_eq!(Limbs::from_slice(&wide).as_slice(), wide.as_slice());
    assert_eq!(Limbs::zeros(20).as_slice(), [0; 20]);
    let mut moved = Limbs::from_slice(&[5, 6]);
    moved.as_mut_slice()[1] = 9;
    let kept = moved;
    assert_eq!(kept.as_slice(), [5, 9]);
}
