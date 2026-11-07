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
        let object = word as *const BigIntObject;
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
        object as i64
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

    pub(crate) fn add(&self, other: &Self) -> Self {
        let (left, right) = (self.magnitude.as_slice(), other.magnitude.as_slice());
        if self.negative == other.negative {
            return Self::signed(self.negative, natural::add(left, right));
        }
        match natural::compare(left, right) {
            Ordering::Less => Self::signed(other.negative, natural::subtract(right, left)),
            _ => Self::signed(self.negative, natural::subtract(left, right)),
        }
    }

    pub(crate) fn subtract(&self, other: &Self) -> Self {
        let flipped = Self {
            negative: !other.negative && !other.is_zero(),
            magnitude: Limbs::from_slice(other.magnitude.as_slice()),
        };
        self.add(&flipped)
    }

    pub(crate) fn multiply(&self, other: &Self) -> Self {
        Self::signed(
            self.negative != other.negative,
            natural::multiply(self.magnitude.as_slice(), other.magnitude.as_slice()),
        )
    }

    /// Returns the quotient rounded toward negative infinity and the remainder with the divisor's sign, as Python's `//` and `%` give; `other` is not zero.
    pub(crate) fn divide_floor(&self, other: &Self) -> (Self, Self) {
        let (quotient, remainder) =
            natural::divide(self.magnitude.as_slice(), other.magnitude.as_slice());
        let quotient = Self::signed(self.negative != other.negative, quotient);
        let remainder = Self::signed(self.negative, remainder);
        if !remainder.is_zero() && remainder.negative != other.negative {
            let one = Self::from_i128(1);
            (quotient.subtract(&one), remainder.add(other))
        } else {
            (quotient, remainder)
        }
    }

    pub(crate) fn compare(&self, other: &Self) -> Ordering {
        match (self.negative, other.negative) {
            (false, true) => Ordering::Greater,
            (true, false) => Ordering::Less,
            (false, false) => {
                natural::compare(self.magnitude.as_slice(), other.magnitude.as_slice())
            }
            (true, true) => natural::compare(other.magnitude.as_slice(), self.magnitude.as_slice()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Integer;

    const SAMPLES: [i128; 12] = [
        0,
        1,
        -1,
        7,
        -7,
        1 << 62,
        -(1 << 62) - 1,
        i64::MAX as i128,
        i64::MIN as i128,
        1 << 100,
        -(1 << 100) + 3,
        i128::MAX / 2,
    ];

    #[test]
    fn values_round_trip_through_words() {
        for value in SAMPLES {
            let word = Integer::from_i128(value).into_word();
            // SAFETY: the word was just made, and a large integer it points to is live.
            assert_eq!(unsafe { Integer::from_word(word) }.to_i128(), Some(value));
        }
    }

    #[test]
    fn arithmetic_and_floor_division_agree_with_python_rules() {
        for a in SAMPLES {
            for b in SAMPLES {
                let (x, y) = (Integer::from_i128(a), Integer::from_i128(b));
                if let Some(sum) = a.checked_add(b) {
                    assert_eq!(x.add(&y).to_i128(), Some(sum));
                }
                if let Some(difference) = a.checked_sub(b) {
                    assert_eq!(x.subtract(&y).to_i128(), Some(difference));
                }
                if let Some(product) = a.checked_mul(b) {
                    assert_eq!(x.multiply(&y).to_i128(), Some(product));
                }
                if b != 0 {
                    let (quotient, remainder) = x.divide_floor(&y);
                    assert_eq!(
                        quotient.to_i128(),
                        Some(a.div_euclid(b) - i128::from(b < 0 && a.rem_euclid(b) != 0)),
                        "{a} // {b}"
                    );
                    let expected =
                        a.rem_euclid(b) + if b < 0 && a.rem_euclid(b) != 0 { b } else { 0 };
                    assert_eq!(remainder.to_i128(), Some(expected), "{a} % {b}");
                }
                assert_eq!(x.compare(&y), a.cmp(&b));
            }
        }
    }

    #[test]
    fn zero_is_never_negative() {
        let zero = Integer::from_i128(0).negate();
        assert!(!zero.negative && zero.is_zero());
        assert!(
            !Integer::from_i128(5)
                .subtract(&Integer::from_i128(5))
                .negative
        );
    }
}
