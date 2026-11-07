//! `float(text)` (LIB-01): Python's grammar for a float string, read exactly and rounded once.

use super::ratio::ratio;
use crate::Raising;
use crate::exception::ExceptionKind;
use crate::int::limbs::Limbs;
use crate::int::natural::multiply_add_small;
use crate::message::MessageBuffer;
use crate::string::{Points, StrObject, decimal_value, is_space};

/// A decimal exponent beyond which a non-zero value is infinite.
const INFINITE_ABOVE: i64 = 310;
/// A decimal exponent below which a value rounds to zero.
const ZERO_BELOW: i64 = -330;
/// Exponent digits past this size cannot change the result, so the value is saturated.
const EXPONENT_LIMIT: i64 = 1_000_000_000;

/// A reader over the code points of the stripped text.
struct Cursor<'text> {
    points: &'text Points,
    index: usize,
    end: usize,
}

impl Cursor<'_> {
    fn peek(&self) -> Option<u32> {
        (self.index < self.end).then(|| self.points.get(self.index))
    }

    fn digit_at(&self, index: usize) -> Option<u32> {
        if index >= self.end {
            return None;
        }
        decimal_value(self.points.get(index))
    }

    /// Takes `+` or `-` and returns true for `-`.
    fn sign(&mut self) -> bool {
        match self.peek().and_then(char::from_u32) {
            Some('-') => {
                self.index += 1;
                true
            }
            Some('+') => {
                self.index += 1;
                false
            }
            _ => false,
        }
    }

    /// Takes digits with single underscores between them, passing each digit to `each`, and
    /// returns true when it took at least one.
    fn digits(&mut self, mut each: impl FnMut(u32)) -> bool {
        let Some(first) = self.digit_at(self.index) else {
            return false;
        };
        each(first);
        self.index += 1;
        loop {
            if let Some(digit) = self.digit_at(self.index) {
                each(digit);
                self.index += 1;
            } else if self.peek() == Some(u32::from(b'_'))
                && let Some(digit) = self.digit_at(self.index + 1)
            {
                each(digit);
                self.index += 2;
            } else {
                return true;
            }
        }
    }

    /// Returns true when the rest of the text is `word`, ignoring ASCII case.
    fn rest_is(&self, word: &str) -> bool {
        self.end - self.index == word.len()
            && word.bytes().enumerate().all(|(offset, byte)| {
                char::from_u32(self.points.get(self.index + offset))
                    .is_some_and(|character| character.eq_ignore_ascii_case(&char::from(byte)))
            })
    }
}

/// The digits of a decimal number, `mantissa * 10^exponent`.
struct Number {
    mantissa: Limbs,
    significant: i64,
    exponent: i64,
}

impl Number {
    fn push(&mut self, digit: u32) {
        if self.mantissa.is_zero() && digit == 0 {
            return;
        }
        self.significant += 1;
        self.mantissa = multiply_add_small(self.mantissa.as_slice(), 10, u64::from(digit));
    }

    /// Returns the value rounded once to the nearest double; too large a value is infinite.
    fn value(&self) -> f64 {
        let magnitude = self.significant.saturating_add(self.exponent);
        if self.mantissa.is_zero() || magnitude < ZERO_BELOW {
            return 0.0;
        }
        if magnitude > INFINITE_ABOVE {
            return f64::INFINITY;
        }
        let mut power = Limbs::from_slice(&[1]);
        for _ in 0..self.exponent.unsigned_abs() {
            power = multiply_add_small(power.as_slice(), 10, 0);
        }
        let result = if self.exponent >= 0 {
            let scaled = crate::int::natural::multiply(self.mantissa.as_slice(), power.as_slice());
            ratio(scaled.as_slice(), &[1])
        } else {
            ratio(self.mantissa.as_slice(), power.as_slice())
        };
        result.unwrap_or(f64::INFINITY)
    }
}

/// Reads a decimal number: digits, a fraction, or both, then an optional exponent.
fn number(cursor: &mut Cursor<'_>) -> Option<f64> {
    let mut number = Number {
        mantissa: Limbs::new(),
        significant: 0,
        exponent: 0,
    };
    let whole = cursor.digits(|digit| number.push(digit));
    let mut fraction = false;
    if cursor.peek() == Some(u32::from(b'.')) {
        cursor.index += 1;
        fraction = cursor.digits(|digit| {
            number.push(digit);
            number.exponent -= 1;
        });
    }
    if !whole && !fraction {
        return None;
    }
    if matches!(cursor.peek(), Some(0x45 | 0x65)) {
        cursor.index += 1;
        let negative = cursor.sign();
        let mut exponent = 0_i64;
        let found = cursor.digits(|digit| {
            exponent = (exponent * 10 + i64::from(digit)).min(EXPONENT_LIMIT);
        });
        if !found {
            return None;
        }
        let exponent = if negative { -exponent } else { exponent };
        number.exponent = number.exponent.saturating_add(exponent);
    }
    (cursor.index == cursor.end).then(|| number.value())
}

fn invalid(text: *const StrObject) -> Raising<f64> {
    let mut message = MessageBuffer::new();
    message.push_str("could not convert string to float: ");
    // SAFETY: the caller of `typy_rt_float_from_str` passes a live string.
    unsafe { message.push_repr(text) };
    Raising::error(0.0, message.raise(ExceptionKind::ValueError))
}

/// Returns `float(text)`: surrounding whitespace, a sign, then `inf`, `infinity`, `nan` in any
/// case, or a decimal number with single underscores between digits.
///
/// # Safety
///
/// `text` points to a live `str` object.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn typy_rt_float_from_str(text: *const StrObject) -> Raising<f64> {
    // SAFETY: the caller passes a live string.
    let points = unsafe { Points::of(text) };
    let mut start = 0;
    let mut end = points.len();
    while start < end && is_space(points.get(start)) {
        start += 1;
    }
    while end > start && is_space(points.get(end - 1)) {
        end -= 1;
    }
    let mut cursor = Cursor {
        points: &points,
        index: start,
        end,
    };
    let negative = cursor.sign();
    let magnitude = if cursor.rest_is("inf") || cursor.rest_is("infinity") {
        Some(f64::INFINITY)
    } else if cursor.rest_is("nan") {
        Some(f64::NAN)
    } else {
        number(&mut cursor)
    };
    match magnitude {
        Some(value) => Raising::ok(if negative { -value } else { value }),
        None => invalid(text),
    }
}

#[cfg(test)]
mod tests;
