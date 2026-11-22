//! Tests of slice positions.

use super::{START, STEP, STOP, Selection, select};
use crate::int::value::Integer;
use crate::word::inline;

/// A slice's start, stop and stride, `None` standing for a missing one.
type Bounds = (Option<i64>, Option<i64>, Option<i64>);

fn word(value: i64) -> i64 {
    inline(value).unwrap()
}

/// The positions `[start:stop:stride]` selects in five items, `None` standing for a missing bound.
fn positions(start: Option<i64>, stop: Option<i64>, stride: Option<i64>) -> Vec<usize> {
    let present = (i64::from(start.is_some()) * START)
        | (i64::from(stop.is_some()) * STOP)
        | (i64::from(stride.is_some()) * STEP);
    let selection = select(
        word(start.unwrap_or(0)),
        word(stop.unwrap_or(0)),
        word(stride.unwrap_or(0)),
        word(present),
        5,
    )
    .unwrap();
    (0..selection.count)
        .map(|index| selection.position(index))
        .collect()
}

#[test]
fn slices_select_the_positions_cpython_selects() {
    let cases: [(Bounds, &[usize]); 11] = [
        ((None, None, None), &[0, 1, 2, 3, 4]),
        ((Some(1), Some(3), None), &[1, 2]),
        ((Some(-2), None, None), &[3, 4]),
        ((None, None, Some(-1)), &[4, 3, 2, 1, 0]),
        ((None, None, Some(2)), &[0, 2, 4]),
        ((Some(4), Some(1), Some(-2)), &[4, 2]),
        ((Some(9), None, None), &[]),
        ((Some(-9), Some(2), None), &[0, 1]),
        ((Some(3), Some(1), None), &[]),
        ((Some(9), Some(-9), Some(-1)), &[4, 3, 2, 1, 0]),
        ((Some(1), Some(3), Some(-1)), &[]),
    ];
    for ((start, stop, stride), expected) in cases {
        assert_eq!(
            positions(start, stop, stride),
            expected,
            "{start:?}:{stop:?}:{stride:?}"
        );
    }
}

#[test]
fn large_bounds_are_clamped_and_a_zero_step_raises() {
    let huge = Integer::from_i128(1 << 70).into_word();
    let negative = Integer::from_i128(-(1 << 70)).into_word();
    let all = word(START | STOP);
    let selection = select(negative, huge, word(0), all, 5).unwrap();
    assert_eq!(
        selection,
        Selection {
            start: 0,
            step: 1,
            count: 5,
            stop: 5,
        }
    );
    assert_eq!(
        select(huge, word(0), word(0), word(START), 5)
            .unwrap()
            .count,
        0
    );
    let zero = select(word(0), word(0), word(0), word(STEP), 5).unwrap_err();
    // SAFETY: the exception is live.
    assert_eq!(unsafe { (*zero).kind() }, crate::ExceptionKind::ValueError);
}
