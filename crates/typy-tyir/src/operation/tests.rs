//! Tests of `operation`.

use super::Comparison;

#[test]
fn comparison_codes_are_distinct_and_dense() {
    let codes: Vec<i64> = [
        Comparison::Equal,
        Comparison::NotEqual,
        Comparison::Less,
        Comparison::LessEqual,
        Comparison::Greater,
        Comparison::GreaterEqual,
    ]
    .iter()
    .map(|comparison| comparison.code())
    .collect();
    assert_eq!(codes, [0, 1, 2, 3, 4, 5]);
}
