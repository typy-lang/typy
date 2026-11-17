//! Integers without a size limit (DAT-01): inline words, and large integers on the heap.

mod arithmetic;
mod bitwise;
mod fixed;
mod length;
pub(crate) mod limbs;
pub(crate) mod natural;
mod text;
pub(crate) mod value;

#[cfg(test)]
pub(crate) use arithmetic::typy_rt_int_power;
pub(crate) use arithmetic::{
    ZERO_DIVISION, typy_rt_int_add, typy_rt_int_compare, typy_rt_int_floor_divide,
    typy_rt_int_modulo, typy_rt_int_multiply, typy_rt_int_negate, typy_rt_int_subtract,
};
#[cfg(test)]
pub(crate) use text::typy_rt_int_from_str;
pub(crate) use text::typy_rt_int_to_str;

use crate::header::ObjectHeader;

/// The fixed part of a large integer; the limbs follow it, least significant first (detailed design section 10.2).
#[repr(C)]
#[derive(Debug)]
pub struct BigIntObject {
    /// The object header.
    pub header: ObjectHeader,
    /// The number of limbs, negated for a negative value.
    pub sign_and_length: i32,
    padding: u32,
}

impl BigIntObject {
    pub(crate) const fn new(header: ObjectHeader, sign_and_length: i32) -> Self {
        Self {
            header,
            sign_and_length,
            padding: 0,
        }
    }
}

/// Returns true when a large integer word is negative.
///
/// # Safety
///
/// `word` points to a live large integer.
pub(crate) unsafe fn is_negative(word: i64) -> bool {
    // SAFETY: the caller passes the address of a live large integer.
    unsafe { (*object_of(word)).sign_and_length < 0 }
}

/// Returns how many times `sequence * count` repeats: none for a count below one, or `None`
/// for one past an index-sized integer, which CPython refuses with `OverflowError`.
///
/// # Safety
///
/// `count` is a live `int` word.
pub(crate) unsafe fn repeat_count(count: i64) -> Option<usize> {
    // SAFETY: the caller passes a live `int` word.
    let value = unsafe { value::Integer::from_word(count) };
    if value.negative {
        return Some(0);
    }
    value
        .to_i128()
        .and_then(|count| i64::try_from(count).ok())
        .and_then(|count| usize::try_from(count).ok())
}

/// Returns the `int` word of a large integer: its address, whose lowest bit is clear. The
/// address is exposed, so `object_of` may turn the word back into a pointer.
#[allow(
    clippy::cast_possible_wrap,
    reason = "a user-space address is below 2^63, so it is a positive word"
)]
pub(crate) fn word_of(object: *mut BigIntObject) -> i64 {
    object.expose_provenance() as i64
}

/// Returns the large integer an `int` word with its lowest bit clear points to.
#[allow(
    clippy::cast_sign_loss,
    clippy::cast_possible_truncation,
    reason = "the word is an address that `word_of` made from a 64-bit pointer"
)]
pub(crate) fn object_of(word: i64) -> *mut BigIntObject {
    core::ptr::with_exposed_provenance_mut(word as usize)
}

#[cfg(test)]
mod tests {
    use super::BigIntObject;

    #[test]
    fn the_fixed_part_takes_sixteen_bytes() {
        assert_eq!(core::mem::size_of::<BigIntObject>(), 16);
    }

    #[test]
    fn a_repeat_count_is_none_below_one_and_refused_past_an_index_sized_integer() {
        use super::repeat_count;
        use super::value::Integer;
        let cases = [
            (-(1_i128 << 70), Some(0)),
            (-1, Some(0)),
            (0, Some(0)),
            (3, Some(3)),
            (1 << 62, Some(1 << 62)),
            (
                i128::from(i64::MAX),
                Some(usize::try_from(i64::MAX).unwrap()),
            ),
            (i128::from(i64::MAX) + 1, None),
            (1 << 70, None),
        ];
        for (count, wanted) in cases {
            let word = Integer::from_i128(count).into_word();
            // SAFETY: the word is a live `int` word.
            assert_eq!(unsafe { repeat_count(word) }, wanted, "{count}");
        }
    }
}
