//! A signed integer of any size, read from an `int` word and written back to one.

use core::cmp::Ordering;

use super::BigIntObject;
use super::limbs::Limbs;
use super::natural;
use crate::allocator::{allocate, array_size};
use crate::header::{BIG_INT_TYPE_ID, ObjectHeader};
use crate::word::{inline, inline_value, is_inline};

const LIMBS_OFFSET: usize = core::mem::size_of::<BigIntObject>();

/// A signed integer: its sign and its magnitude. Zero is never negative.
pub(crate) struct Integer {
    pub(crate) negative: bool,
    pub(crate) magnitude: Limbs,
}

impl Integer {
    /// Returns the integer of a machine value.
    pub(crate) fn from_i128(value: i128) -> Self {
        let magnitude = value.unsigned_abs();
        let low = u64::try_from(magnitude & u128::from(u64::MAX)).unwrap_or(0);
        let high = u64::try_from(magnitude >> 64).unwrap_or(0);
        Self {
            negative: value < 0,
            magnitude: Limbs::from_slice(&[low, high]),
        }
    }

    /// Returns the integer an `int` word holds.
    ///
    /// # Safety
    ///
    /// A word with its lowest bit clear points to a live large integer.
    pub(crate) unsafe fn from_word(word: i64) -> Self {
        if is_inline(word) {
            return Self::from_i128(i128::from(inline_value(word)));
        }
        let object = super::object_of(word);
        // SAFETY: the word points to a live large integer, whose limbs follow its fixed part.
        unsafe {
            let sign_and_length = (*object).sign_and_length;
            let count = usize::try_from(sign_and_length.unsigned_abs()).unwrap_or(0);
            #[allow(
                clippy::cast_ptr_alignment,
                reason = "the limbs follow a 16-byte fixed part, so they are 8-byte aligned"
            )]
            let limbs = object.cast::<u8>().add(LIMBS_OFFSET).cast::<u64>();
            Self {
                negative: sign_and_length < 0,
                magnitude: Limbs::from_slice(core::slice::from_raw_parts(limbs, count)),
            }
        }
    }

    /// Returns the value as a machine integer, when it fits.
    pub(crate) fn to_i128(&self) -> Option<i128> {
        let limbs = self.magnitude.as_slice();
        if limbs.len() > 2 {
            return None;
        }
        let magnitude = limbs
            .iter()
            .rev()
            .fold(0_u128, |total, limb| (total << 64) | u128::from(*limb));
        if self.negative {
            magnitude
                .checked_sub(1)
                .and_then(|less| i128::try_from(less).ok())
                .map(|less| -less - 1)
        } else {
            i128::try_from(magnitude).ok()
        }
    }

    /// Returns the `int` word of the value: inline when it fits, otherwise a new large integer the caller owns.
    pub(crate) fn into_word(self) -> i64 {
        if let Some(word) = self
            .to_i128()
            .and_then(|value| i64::try_from(value).ok())
            .and_then(inline)
        {
            return word;
        }
        let limbs = self.magnitude.as_slice();
        let size = array_size(limbs.len(), core::mem::size_of::<u64>(), LIMBS_OFFSET);
        #[allow(
            clippy::cast_ptr_alignment,
            reason = "malloc returns memory aligned for every type"
        )]
        let object = allocate(size).cast::<BigIntObject>();
        let length = i32::try_from(limbs.len())
            .unwrap_or_else(|_| crate::libc::abort_with(b"MemoryError\n"));
        // SAFETY: `object` points to `size` writable bytes: the fixed part, then the limbs.
        unsafe {
            object.write(BigIntObject::new(
                ObjectHeader::new(BIG_INT_TYPE_ID),
                if self.negative { -length } else { length },
            ));
            #[allow(
                clippy::cast_ptr_alignment,
                reason = "the limbs follow a 16-byte fixed part, so they are 8-byte aligned"
            )]
            let destination = object.cast::<u8>().add(LIMBS_OFFSET).cast::<u64>();
            core::ptr::copy_nonoverlapping(limbs.as_ptr(), destination, limbs.len());
        }
        super::word_of(object)
    }

    fn signed(negative: bool, magnitude: Limbs) -> Self {
        let negative = negative && !magnitude.is_zero();
        Self {
            negative,
            magnitude,
        }
    }

    pub(crate) fn is_zero(&self) -> bool {
        self.magnitude.is_zero()
    }

    pub(crate) fn negate(self) -> Self {
        Self::signed(!self.negative, self.magnitude)
    }

    /// Returns the integer as a view of its limbs.
    pub(crate) fn view(&self) -> View<'_> {
        View {
            negative: self.negative,
            magnitude: self.magnitude.as_slice(),
        }
    }

    pub(crate) fn add(&self, other: &Self) -> Self {
        self.view().add(other.view())
    }

    pub(crate) fn subtract(&self, other: &Self) -> Self {
        self.view().subtract(other.view())
    }

    pub(crate) fn multiply(&self, other: &Self) -> Self {
        self.view().multiply(other.view())
    }

    /// Returns the quotient rounded toward negative infinity and the remainder with the divisor's sign, as Python's `//` and `%` give; `other` is not zero.
    pub(crate) fn divide_floor(&self, other: &Self) -> (Self, Self) {
        self.view().divide_floor(other.view())
    }

    pub(crate) fn compare(&self, other: &Self) -> Ordering {
        self.view().compare(other.view())
    }
}

/// The sign and limbs of an integer, borrowed: from a large integer's object, which the
/// arithmetic then reads without copying (ADR-0103), or from an `Integer`.
#[derive(Clone, Copy)]
pub(crate) struct View<'a> {
    pub(crate) negative: bool,
    pub(crate) magnitude: &'a [u64],
}

impl<'a> View<'a> {
    /// Returns the view of an `int` word: its large integer's limbs in place, or an inline
    /// value's magnitude held in `held`.
    ///
    /// # Safety
    ///
    /// A word with its lowest bit clear points to a live large integer, which outlives the view.
    pub(crate) unsafe fn of(word: i64, held: &'a mut [u64; 1]) -> Self {
        if is_inline(word) {
            let value = inline_value(word);
            held[0] = value.unsigned_abs();
            let length = usize::from(value != 0);
            return Self {
                negative: value < 0,
                magnitude: &held[..length],
            };
        }
        let object = super::object_of(word);
        // SAFETY: the word points to a live large integer, whose limbs follow its fixed part.
        unsafe {
            let sign_and_length = (*object).sign_and_length;
            let count = usize::try_from(sign_and_length.unsigned_abs()).unwrap_or(0);
            #[allow(
                clippy::cast_ptr_alignment,
                reason = "the limbs follow a 16-byte fixed part, so they are 8-byte aligned"
            )]
            let limbs = object.cast::<u8>().add(LIMBS_OFFSET).cast::<u64>();
            Self {
                negative: sign_and_length < 0,
                magnitude: core::slice::from_raw_parts(limbs, count),
            }
        }
    }

    /// Returns the value with the other sign; zero stays zero.
    fn negated(self) -> Self {
        Self {
            negative: !self.negative && !self.magnitude.is_empty(),
            magnitude: self.magnitude,
        }
    }

    pub(crate) fn add(self, other: View<'_>) -> Integer {
        let (left, right) = (self.magnitude, other.magnitude);
        if self.negative == other.negative {
            return Integer::signed(self.negative, natural::add(left, right));
        }
        match natural::compare(left, right) {
            Ordering::Less => Integer::signed(other.negative, natural::subtract(right, left)),
            _ => Integer::signed(self.negative, natural::subtract(left, right)),
        }
    }

    pub(crate) fn subtract(self, other: View<'_>) -> Integer {
        self.add(other.negated())
    }

    pub(crate) fn multiply(self, other: View<'_>) -> Integer {
        Integer::signed(
            self.negative != other.negative,
            natural::multiply(self.magnitude, other.magnitude),
        )
    }

    /// Returns the quotient rounded toward negative infinity and the remainder with the divisor's sign, as Python's `//` and `%` give; `other` is not zero.
    pub(crate) fn divide_floor(self, other: View<'_>) -> (Integer, Integer) {
        let (quotient, remainder) = natural::divide(self.magnitude, other.magnitude);
        let quotient = Integer::signed(self.negative != other.negative, quotient);
        let remainder = Integer::signed(self.negative, remainder);
        if !remainder.is_zero() && remainder.negative != other.negative {
            // Rounding toward negative infinity takes one from the quotient and moves the
            // remainder to the divisor's side.
            let one = View {
                negative: true,
                magnitude: &[1],
            };
            (quotient.view().add(one), remainder.view().add(other))
        } else {
            (quotient, remainder)
        }
    }

    pub(crate) fn compare(self, other: View<'_>) -> Ordering {
        match (self.negative, other.negative) {
            (false, true) => Ordering::Greater,
            (true, false) => Ordering::Less,
            (false, false) => natural::compare(self.magnitude, other.magnitude),
            (true, true) => natural::compare(other.magnitude, self.magnitude),
        }
    }
}

#[cfg(test)]
mod tests;
