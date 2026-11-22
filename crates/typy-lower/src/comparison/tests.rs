//! Tests of `comparison`.

use super::mirrored;
use typy_tyir::Comparison;

#[test]
fn mirroring_swaps_the_direction_and_keeps_equality() {
    assert_eq!(mirrored(Comparison::Less), Comparison::Greater);
    assert_eq!(mirrored(Comparison::GreaterEqual), Comparison::LessEqual);
    assert_eq!(mirrored(Comparison::Equal), Comparison::Equal);
    assert_eq!(mirrored(Comparison::NotEqual), Comparison::NotEqual);
}
