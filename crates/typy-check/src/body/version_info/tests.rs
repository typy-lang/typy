//! Tests of `body::version_info`.

use super::{CompareOperator, compare};

#[test]
fn version_info_compares_as_cpython_compares_tuples() {
    use CompareOperator::{Eq, Gt, GtE, Lt, LtE, NotEq};
    let cases: &[(CompareOperator, &[i64], bool, Option<bool>)] = &[
        (GtE, &[3, 12], false, Some(true)),
        (GtE, &[3, 15], false, Some(true)),
        (Gt, &[3, 15], false, Some(true)),
        (Lt, &[3, 16], false, Some(true)),
        (LtE, &[3, 15, 0], false, Some(false)),
        (Eq, &[3, 15, 0], false, Some(false)),
        (NotEq, &[3, 15], false, Some(true)),
        (Lt, &[3, 12], true, Some(true)),
        (GtE, &[4], true, Some(true)),
        (GtE, &[], false, Some(true)),
        (GtE, &[3, 15, 0, 0], false, None),
        (Eq, &[3, 15, 0, 0], false, Some(false)),
        (Gt, &[3, 14, 0, 0], false, Some(true)),
    ];
    for (operator, other, reversed, expected) in cases {
        assert_eq!(
            compare(*operator, other, *reversed),
            *expected,
            "{operator:?} {other:?} {reversed}"
        );
    }
}
