//! Slice positions, worked out as CPython's `PySlice_Unpack` and `PySlice_AdjustIndices` do,
//! for every sequence that can be sliced (DAT-12).

use crate::exception::{ExceptionKind, ExceptionObject, raise};
use crate::word::{inline_value, is_inline};

/// The bit of the presence mask set when the slice gives a start.
pub(crate) const START: i64 = 1;
/// The bit set when the slice gives a stop.
pub(crate) const STOP: i64 = 2;
/// The bit set when the slice gives a step.
pub(crate) const STEP: i64 = 4;

/// The items a slice selects: the first position, the step between positions, and how many.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Selection {
    /// The first position.
    pub(crate) start: i64,
    /// The distance from one position to the next; never zero.
    pub(crate) step: i64,
    /// The number of positions.
    pub(crate) count: usize,
}

impl Selection {
    /// Returns the `index`-th position the slice selects.
    pub(crate) fn position(self, index: usize) -> usize {
        let offset = i64::try_from(index).unwrap_or(i64::MAX) * self.step;
        usize::try_from(self.start + offset).unwrap_or(0)
    }
}

/// Returns the value of an `int` word, a large integer clamped to the range of `i64`, as
/// CPython clamps a slice bound to `Py_ssize_t`.
fn clamped(word: i64) -> i64 {
    if is_inline(word) {
        inline_value(word)
    } else {
        // SAFETY: a word that is not inline points to a live large integer.
        if unsafe { crate::int::is_negative(word) } {
            i64::MIN
        } else {
            i64::MAX
        }
    }
}

/// Returns the positions `[start:stop:stride]` selects in a sequence of `length` items. Each bound
/// is an `int` word, given when its bit of `present`, an `int` word of `START`, `STOP` and
/// `STEP`, is set; a step of zero raises `ValueError`.
pub(crate) fn select(
    start: i64,
    stop: i64,
    stride: i64,
    present: i64,
    length: usize,
) -> Result<Selection, *mut ExceptionObject> {
    let present = inline_value(present);
    let stride = if present & STEP == 0 {
        1
    } else {
        clamped(stride).max(-i64::MAX)
    };
    if stride == 0 {
        return Err(raise(
            ExceptionKind::ValueError,
            "slice step cannot be zero",
        ));
    }
    let backwards = stride < 0;
    let length = i64::try_from(length).unwrap_or(i64::MAX);
    let adjust = |bound: i64| {
        if bound < 0 {
            let from_end = bound.saturating_add(length);
            if from_end < 0 {
                if backwards { -1 } else { 0 }
            } else {
                from_end
            }
        } else if bound >= length {
            if backwards { length - 1 } else { length }
        } else {
            bound
        }
    };
    let start = if present & START == 0 {
        if backwards { length - 1 } else { 0 }
    } else {
        adjust(clamped(start))
    };
    let stop = if present & STOP == 0 {
        if backwards { -1 } else { length }
    } else {
        adjust(clamped(stop))
    };
    let count = if backwards {
        if stop < start {
            (start - stop - 1) / (-stride) + 1
        } else {
            0
        }
    } else if start < stop {
        (stop - start - 1) / stride + 1
    } else {
        0
    };
    Ok(Selection {
        start,
        step: stride,
        count: usize::try_from(count).unwrap_or(0),
    })
}

#[cfg(test)]
mod tests {
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
        let cases: [(Bounds, &[usize]); 10] = [
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
                count: 5
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
        assert_eq!(unsafe { (*zero).kind }, crate::ExceptionKind::ValueError);
    }
}
