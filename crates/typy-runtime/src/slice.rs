//! Slice positions, worked out as CPython's `PySlice_Unpack` and `PySlice_AdjustIndices` do,
//! for every sequence that can be sliced (DAT-12).

pub(crate) mod object;

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
    /// The stop, adjusted to the sequence as `slice.indices` gives it.
    pub(crate) stop: i64,
}

impl Selection {
    /// Returns where a slice of step 1 starts, which is also where nothing is selected.
    pub(crate) fn start_position(self) -> usize {
        usize::try_from(self.start).unwrap_or(0)
    }

    /// Returns the `index`-th position the slice selects.
    pub(crate) fn position(self, index: usize) -> usize {
        let offset = i64::try_from(index).unwrap_or(i64::MAX) * self.step;
        usize::try_from(self.start + offset).unwrap_or(0)
    }
}

/// Returns the value of an `int` word, a large integer clamped to the range of `i64`, as
/// CPython clamps a slice bound to `Py_ssize_t`.
pub(crate) fn clamped(word: i64) -> i64 {
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
/// `STEP`, is set and the word is not 0, which no `int` is and which stands for `None`; a step
/// of zero raises `ValueError`.
pub(crate) fn select(
    start: i64,
    stop: i64,
    stride: i64,
    present: i64,
    length: usize,
) -> Result<Selection, *mut ExceptionObject> {
    let present = inline_value(present)
        & [(START, start), (STOP, stop), (STEP, stride)]
            .iter()
            .filter(|(_, word)| *word != 0)
            .fold(0, |bits, (bit, _)| bits | bit);
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
        stop,
    })
}

#[cfg(test)]
mod tests;
