//! Shifts and bitwise operations on `int`, with Python's results for negative values: two's complement over infinitely many bits (EXP-05).

use super::limbs::Limbs;
use super::natural;
use super::value::Integer;
use crate::Raising;
use crate::exception::{ExceptionKind, raise};
use crate::word::{inline, inline_value, is_inline};

/// Returns the value's two's complement in `width` limbs; `width` is more than its magnitude needs.
fn twos_complement(value: &Integer, width: usize) -> Limbs {
    let magnitude = if value.negative {
        natural::subtract(value.magnitude.as_slice(), &[1])
    } else {
        Limbs::from_slice(value.magnitude.as_slice())
    };
    let mut limbs = Limbs::new();
    for index in 0..width {
        let bits = magnitude.as_slice().get(index).copied().unwrap_or(0);
        limbs.push(if value.negative { !bits } else { bits });
    }
    limbs
}

/// Returns the integer a two's complement array holds; its top bit is the sign.
fn from_twos_complement(limbs: &Limbs) -> Integer {
    let negative = limbs.as_slice().last().is_some_and(|top| top >> 63 == 1);
    if !negative {
        return Integer {
            negative: false,
            magnitude: Limbs::from_slice(limbs.as_slice()),
        };
    }
    let mut inverted = Limbs::new();
    for limb in limbs.as_slice() {
        inverted.push(!limb);
    }
    inverted.trim();
    Integer {
        negative: true,
        magnitude: natural::add(inverted.as_slice(), &[1]),
    }
}

/// Applies a bitwise operation; inline words need no allocation.
fn bitwise(left: i64, right: i64, operation: fn(u64, u64) -> u64) -> i64 {
    if is_inline(left) && is_inline(right) {
        #[allow(
            clippy::cast_sign_loss,
            clippy::cast_possible_wrap,
            reason = "the operation works on the bits of the values"
        )]
        let value = operation(inline_value(left) as u64, inline_value(right) as u64) as i64;
        if let Some(word) = inline(value) {
            return word;
        }
    }
    // SAFETY: the callers pass `int` words whose large integers are live.
    let (left, right) = unsafe { (Integer::from_word(left), Integer::from_word(right)) };
    let width = left
        .magnitude
        .as_slice()
        .len()
        .max(right.magnitude.as_slice().len())
        + 1;
    let (a, b) = (
        twos_complement(&left, width),
        twos_complement(&right, width),
    );
    let mut result = Limbs::new();
    for index in 0..width {
        result.push(operation(a.as_slice()[index], b.as_slice()[index]));
    }
    from_twos_complement(&result).into_word()
}

/// Returns `left & right`.
///
/// # Safety
///
/// Both arguments are `int` words whose large integers are live.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn typy_rt_int_and(left: i64, right: i64) -> i64 {
    bitwise(left, right, |a, b| a & b)
}

/// Returns `left | right`.
///
/// # Safety
///
/// Both arguments are `int` words whose large integers are live.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn typy_rt_int_or(left: i64, right: i64) -> i64 {
    bitwise(left, right, |a, b| a | b)
}

/// Returns `left ^ right`.
///
/// # Safety
///
/// Both arguments are `int` words whose large integers are live.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn typy_rt_int_xor(left: i64, right: i64) -> i64 {
    bitwise(left, right, |a, b| a ^ b)
}

/// Returns `~value`, which is `-value - 1`.
///
/// # Safety
///
/// `value` is an `int` word whose large integer is live.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn typy_rt_int_invert(value: i64) -> i64 {
    if is_inline(value) {
        // Flipping every bit except the tag inverts an inline value, and the result is always inline.
        return value ^ !1;
    }
    // SAFETY: the caller passes an `int` word whose large integer is live.
    let value = unsafe { Integer::from_word(value) };
    value.negate().subtract(&Integer::from_i128(1)).into_word()
}

/// Returns the shift count of a word, or the exception Python raises for it.
fn shift_count(count: i64) -> Result<Option<u64>, Raising<i64>> {
    let negative = if is_inline(count) {
        inline_value(count) < 0
    } else {
        // SAFETY: a word with its lowest bit clear points to a live large integer.
        unsafe { super::is_negative(count) }
    };
    if negative {
        return Err(Raising::error(
            1,
            raise(ExceptionKind::ValueError, "negative shift count"),
        ));
    }
    Ok(if is_inline(count) {
        u64::try_from(inline_value(count)).ok()
    } else {
        None
    })
}

/// Returns `value << count`; a negative count raises `ValueError`.
///
/// # Safety
///
/// Both arguments are `int` words whose large integers are live.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn typy_rt_int_left_shift(value: i64, count: i64) -> Raising<i64> {
    let bits = match shift_count(count) {
        Ok(bits) => bits,
        Err(error) => return error,
    };
    // SAFETY: the caller passes an `int` word whose large integer is live.
    let value = unsafe { Integer::from_word(value) };
    if value.is_zero() {
        return Raising::ok(inline(0).unwrap_or(1));
    }
    let Some(bits) = bits.filter(|bits| *bits <= 1 << 40) else {
        return Raising::error(
            1,
            raise(ExceptionKind::OverflowError, "too many digits in integer"),
        );
    };
    let magnitude = natural::shift_left(value.magnitude.as_slice(), bits);
    Raising::ok(
        Integer {
            negative: value.negative,
            magnitude,
        }
        .into_word(),
    )
}

/// Returns `value >> count`, rounded toward negative infinity; a negative count raises `ValueError`.
///
/// # Safety
///
/// Both arguments are `int` words whose large integers are live.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn typy_rt_int_right_shift(value: i64, count: i64) -> Raising<i64> {
    let bits = match shift_count(count) {
        Ok(bits) => bits.unwrap_or(u64::MAX),
        Err(error) => return error,
    };
    // SAFETY: the caller passes an `int` word whose large integer is live.
    let value = unsafe { Integer::from_word(value) };
    let magnitude = value.magnitude.as_slice();
    let result = if value.negative {
        // -(m) >> n is -(((m - 1) >> n) + 1), which rounds toward negative infinity.
        let reduced = natural::shift_right(natural::subtract(magnitude, &[1]).as_slice(), bits);
        Integer {
            negative: true,
            magnitude: natural::add(reduced.as_slice(), &[1]),
        }
    } else {
        Integer {
            negative: false,
            magnitude: natural::shift_right(magnitude, bits),
        }
    };
    Raising::ok(result.into_word())
}

#[cfg(test)]
mod tests;
