//! Tests of `body::slice`.

use super::positions;

/// A slice's lower bound, upper bound and step, `None` standing for a missing one.
type Bounds = (Option<i128>, Option<i128>, Option<i128>);

#[test]
fn tuple_slices_select_what_cpython_selects() {
    let cases: [(Bounds, Option<&[usize]>); 8] = [
        ((None, None, None), Some(&[0, 1, 2, 3])),
        ((Some(1), None, None), Some(&[1, 2, 3])),
        ((None, None, Some(-1)), Some(&[3, 2, 1, 0])),
        ((Some(-2), Some(100), None), Some(&[2, 3])),
        ((Some(9), Some(-9), Some(-2)), Some(&[3, 1])),
        ((Some(3), Some(1), None), Some(&[])),
        ((Some(-100), Some(1), None), Some(&[0])),
        ((None, None, Some(0)), None),
    ];
    for ((lower, upper, step), expected) in cases {
        assert_eq!(
            positions(lower, upper, step, 4).as_deref(),
            expected,
            "{lower:?}:{upper:?}:{step:?}"
        );
    }
}
