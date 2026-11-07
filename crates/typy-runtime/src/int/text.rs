//! Converts `int` to and from `str` (LIB-01), with CPython's limit of 4300 decimal digits.

use super::natural;
use super::value::Integer;
use crate::Raising;
use crate::exception::{ExceptionKind, raise};
use crate::message::MessageBuffer;
use crate::string::{
    Points, StrKind, StrObject, decimal_value, is_space, new_string, set_code_point,
};
use crate::word::{inline_value, is_inline};

/// The most decimal digits CPython converts by default (`sys.int_info.default_max_str_digits`).
const MAX_DIGITS: usize = 4300;
/// The largest power of ten in a limb: 10^19.
const CHUNK: u64 = 10_000_000_000_000_000_000;
const CHUNK_DIGITS: usize = 19;

/// Returns `str(value)`: its decimal digits with a leading `-` when negative.
///
/// # Safety
///
/// `value` is an `int` word whose large integer is live.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn typy_rt_int_to_str(value: i64) -> Raising<*mut StrObject> {
    let integer = if is_inline(value) {
        Integer::from_i128(i128::from(inline_value(value)))
    } else {
        // SAFETY: the caller passes an `int` word whose large integer is live.
        unsafe { Integer::from_word(value) }
    };
    let mut rest = super::limbs::Limbs::from_slice(integer.magnitude.as_slice());
    let mut pieces = super::limbs::Limbs::new();
    loop {
        let (quotient, remainder) = natural::divide_small(rest.as_slice(), CHUNK);
        pieces.push(remainder);
        rest = quotient;
        if rest.is_zero() {
            break;
        }
    }
    let pieces = pieces.as_slice();
    let top = pieces[pieces.len() - 1];
    let top_digits = if top == 0 {
        1
    } else {
        usize::try_from(top.ilog10()).unwrap_or(0) + 1
    };
    let digits = top_digits + (pieces.len() - 1) * CHUNK_DIGITS;
    if digits > MAX_DIGITS {
        let message = "Exceeds the limit (4300 digits) for integer string conversion; use sys.set_int_max_str_digits() to increase the limit";
        return Raising::error(
            core::ptr::null_mut(),
            raise(ExceptionKind::ValueError, message),
        );
    }
    let sign = usize::from(integer.negative);
    let string = new_string(digits + sign, StrKind::Latin1);
    let mut position = digits + sign;
    for (index, piece) in pieces.iter().enumerate() {
        let width = if index + 1 == pieces.len() {
            top_digits
        } else {
            CHUNK_DIGITS
        };
        let mut piece = *piece;
        for _ in 0..width {
            position -= 1;
            // SAFETY: the position is within the new string, and a digit fits Latin-1.
            unsafe {
                set_code_point(
                    string,
                    position,
                    u32::from(b'0') + u32::try_from(piece % 10).unwrap_or(0),
                );
            }
            piece /= 10;
        }
    }
    if integer.negative {
        // SAFETY: position 0 is within the new string.
        unsafe { set_code_point(string, 0, u32::from(b'-')) };
    }
    Raising::ok(string)
}

fn invalid(text: *const StrObject, base: i64) -> Raising<i64> {
    let mut message = MessageBuffer::new();
    message.push_str("invalid literal for int() with base ");
    message.push_number(base);
    message.push_str(": ");
    // SAFETY: the caller of `typy_rt_int_from_str` passes a live string.
    unsafe { message.push_repr(text) };
    Raising::error(1, message.raise(ExceptionKind::ValueError))
}

/// Returns `int(text, base)`, as Python parses it: surrounding whitespace, a sign, a base prefix where the base allows one, and single underscores between digits.
///
/// # Safety
///
/// `text` points to a live `str` object and `base` is an `int` word.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn typy_rt_int_from_str(text: *const StrObject, base: i64) -> Raising<i64> {
    let base_value = if is_inline(base) {
        inline_value(base)
    } else {
        -1
    };
    if base_value != 0 && !(2..=36).contains(&base_value) {
        return Raising::error(
            1,
            raise(
                ExceptionKind::ValueError,
                "int() base must be >= 2 and <= 36, or 0",
            ),
        );
    }
    // SAFETY: the caller passes a live string; every index stays below its length.
    let points: Points = unsafe { Points::of(text) };
    let mut start = 0;
    let mut end = points.len();
    while start < end && is_space(points.get(start)) {
        start += 1;
    }
    while end > start && is_space(points.get(end - 1)) {
        end -= 1;
    }
    let mut negative = false;
    if start < end && (points.get(start) == u32::from(b'+') || points.get(start) == u32::from(b'-'))
    {
        negative = points.get(start) == u32::from(b'-');
        start += 1;
    }
    let mut radix = u32::try_from(base_value).unwrap_or(10);
    let mut after_prefix = false;
    if end - start >= 2 && points.get(start) == u32::from(b'0') {
        let marker = points.get(start + 1) | 0x20;
        let prefixed = match marker {
            0x78 => Some(16),
            0x6f => Some(8),
            0x62 => Some(2),
            _ => None,
        };
        if let Some(prefixed) = prefixed.filter(|prefixed| radix == 0 || radix == *prefixed) {
            radix = prefixed;
            start += 2;
            after_prefix = true;
        }
    }
    let decimal_zero_rule = radix == 0;
    if radix == 0 {
        radix = 10;
    }
    let mut digits = 0_usize;
    let mut value = super::limbs::Limbs::new();
    let mut previous_underscore = false;
    let mut first_digit = None;
    let mut leading_nonzero_zero = false;
    for index in start..end {
        let point = points.get(index);
        if point == u32::from(b'_') {
            // An underscore separates two digits, or follows a base prefix directly.
            let follows_prefix = after_prefix && index == start;
            if previous_underscore || (digits == 0 && !follows_prefix) {
                return invalid(text, base_value);
            }
            previous_underscore = true;
            continue;
        }
        let Some(digit) = digit_value(point).filter(|digit| *digit < radix) else {
            return invalid(text, base_value);
        };
        match first_digit {
            None => first_digit = Some(digit),
            Some(0) if digit != 0 => leading_nonzero_zero = true,
            Some(_) => {}
        }
        previous_underscore = false;
        digits += 1;
        value = natural::multiply_add_small(value.as_slice(), u64::from(radix), u64::from(digit));
    }
    if digits == 0 || previous_underscore || (decimal_zero_rule && leading_nonzero_zero) {
        return invalid(text, base_value);
    }
    if !radix.is_power_of_two() && digits > MAX_DIGITS {
        let mut message = MessageBuffer::new();
        message
            .push_str("Exceeds the limit (4300 digits) for integer string conversion: value has ");
        message.push_number(i64::try_from(digits).unwrap_or(i64::MAX));
        message.push_str(" digits; use sys.set_int_max_str_digits() to increase the limit");
        return Raising::error(1, message.raise(ExceptionKind::ValueError));
    }
    Raising::ok(
        Integer {
            negative: negative && !value.is_zero(),
            magnitude: value,
        }
        .into_word(),
    )
}

/// Returns the value of a digit in bases up to 36: a decimal digit of any script, as CPython
/// reads it, or an ASCII letter, `a` and `A` being 10.
fn digit_value(point: u32) -> Option<u32> {
    decimal_value(point).or_else(|| {
        char::from_u32(point)
            .filter(char::is_ascii_alphabetic)
            .and_then(|letter| letter.to_digit(36))
    })
}

#[cfg(test)]
mod tests;
