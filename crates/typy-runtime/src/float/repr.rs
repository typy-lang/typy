//! `repr(float)` and `str(float)` (DAT-02): the shortest digits that read back as the same double,
//! found exactly with large integers by the free-format method of Steele and White as Burger and
//! Dybvig state it, then laid out as CPython's `float_repr` does.

use core::cmp::Ordering;

use crate::int::limbs::Limbs;
use crate::int::natural::{add, compare, divide, multiply_add_small, shift_left};
use crate::libm::is_sign_negative;
use crate::string::{StrObject, from_str};

/// Seventeen significant digits identify every double.
const MAX_DIGITS: usize = 17;

/// A decimal value `0.d1 d2 ... dn * 10^point`.
pub(crate) struct Decimal {
    digits: [u8; MAX_DIGITS],
    count: usize,
    point: i64,
}

impl Decimal {
    fn push(&mut self, digit: u64) {
        if self.count < MAX_DIGITS {
            self.digits[self.count] = u8::try_from(digit).unwrap_or(9);
            self.count += 1;
        }
    }

    /// Returns the digits, each from 0 to 9, without trailing zeros.
    pub(crate) fn digits(&self) -> &[u8] {
        &self.digits[..self.count]
    }

    /// Returns where the decimal point is: the value is `0.digits * 10^point`.
    pub(crate) const fn point(&self) -> i64 {
        self.point
    }
}

fn times_ten(value: &Limbs) -> Limbs {
    multiply_add_small(value.as_slice(), 10, 0)
}

fn power_of_two(bits: u64) -> Limbs {
    shift_left(&[1], bits)
}

/// Returns true when `ordering` passes a rounding bound; the bound itself counts when the
/// mantissa is even, because a reader rounds a tie to the even mantissa.
fn within(ordering: Ordering, beyond: Ordering, even: bool) -> bool {
    ordering == beyond || (even && ordering == Ordering::Equal)
}

/// The state of digit generation: the value is `remainder / scale * 10^point`, and `plus` and
/// `minus` are half the gaps to the neighbouring doubles, on the same scale.
struct Generator {
    remainder: Limbs,
    scale: Limbs,
    plus: Limbs,
    minus: Limbs,
    point: i64,
    even: bool,
}

impl Generator {
    /// Returns the generator of a positive finite value, scaled so its first digit comes next.
    fn new(value: f64) -> Self {
        let bits = value.to_bits();
        let biased = (bits >> 52) & 0x7ff;
        let fraction = bits & ((1 << 52) - 1);
        let (mantissa, exponent) = if biased == 0 {
            (fraction, -1074_i64)
        } else {
            (
                fraction | (1 << 52),
                i64::try_from(biased).unwrap_or(0) - 1075,
            )
        };
        // Above a power of two the next double is twice as far as below it, except at the
        // smallest normal, whose neighbour below is a subnormal at the same distance.
        let unequal = fraction == 0 && biased > 1;
        let distance = exponent.unsigned_abs();
        let one = || Limbs::from_slice(&[1]);
        let (remainder, scale, plus, minus) = match (exponent >= 0, unequal) {
            (true, false) => (
                shift_left(&[mantissa], distance + 1),
                Limbs::from_slice(&[2]),
                power_of_two(distance),
                power_of_two(distance),
            ),
            (true, true) => (
                shift_left(&[mantissa], distance + 2),
                Limbs::from_slice(&[4]),
                power_of_two(distance + 1),
                power_of_two(distance),
            ),
            (false, false) => (
                shift_left(&[mantissa], 1),
                power_of_two(distance + 1),
                one(),
                one(),
            ),
            (false, true) => (
                shift_left(&[mantissa], 2),
                power_of_two(distance + 2),
                Limbs::from_slice(&[2]),
                one(),
            ),
        };
        // floor(log2(value) * log10(2)) less one never exceeds the decimal exponent.
        let log2 = exponent + i64::from(64 - mantissa.leading_zeros()) - 1;
        let mut generator = Self {
            remainder,
            scale,
            plus,
            minus,
            point: ((log2 * 78_913) >> 18) - 1,
            even: mantissa % 2 == 0,
        };
        generator.scale_to_point();
        generator
    }

    /// Multiplies by `10^-point`, then raises `point` until the upper bound is below one.
    fn scale_to_point(&mut self) {
        for _ in 0..self.point.unsigned_abs() {
            if self.point >= 0 {
                self.scale = times_ten(&self.scale);
            } else {
                self.remainder = times_ten(&self.remainder);
                self.plus = times_ten(&self.plus);
                self.minus = times_ten(&self.minus);
            }
        }
        while self.high() {
            self.scale = times_ten(&self.scale);
            self.point += 1;
        }
    }

    /// Returns true when the value plus its upper half gap reaches the next unit.
    fn high(&self) -> bool {
        let upper = add(self.remainder.as_slice(), self.plus.as_slice());
        within(
            compare(upper.as_slice(), self.scale.as_slice()),
            Ordering::Greater,
            self.even,
        )
    }

    /// Returns true when the value less its lower half gap falls below the current digit.
    fn low(&self) -> bool {
        within(
            compare(self.remainder.as_slice(), self.minus.as_slice()),
            Ordering::Less,
            self.even,
        )
    }

    /// Returns the next digit, leaving the rest in `remainder`.
    fn next_digit(&mut self) -> u64 {
        self.remainder = times_ten(&self.remainder);
        self.plus = times_ten(&self.plus);
        self.minus = times_ten(&self.minus);
        let (quotient, rest) = divide(self.remainder.as_slice(), self.scale.as_slice());
        self.remainder = rest;
        quotient.as_slice().first().copied().unwrap_or(0)
    }

    /// Returns true when the last digit rounds up: the rest is above half a unit, or exactly half
    /// with an odd digit, as CPython rounds a tie.
    fn rounds_up(&self, digit: u64) -> bool {
        let twice = shift_left(self.remainder.as_slice(), 1);
        match compare(twice.as_slice(), self.scale.as_slice()) {
            Ordering::Less => false,
            Ordering::Greater => true,
            Ordering::Equal => digit % 2 == 1,
        }
    }
}

/// Returns the shortest decimal that reads back as `value`, which is positive and finite.
pub(crate) fn shortest(value: f64) -> Decimal {
    let mut generator = Generator::new(value);
    let mut decimal = Decimal {
        digits: [0; MAX_DIGITS],
        count: 0,
        point: generator.point,
    };
    loop {
        let digit = generator.next_digit();
        let last = match (generator.low(), generator.high()) {
            (false, false) => {
                decimal.push(digit);
                continue;
            }
            (true, false) => digit,
            (false, true) => digit + 1,
            (true, true) => digit + u64::from(generator.rounds_up(digit)),
        };
        decimal.push(last);
        return decimal;
    }
}

/// Up to 32 bytes of ASCII text, enough for any `repr` of a double.
struct Ascii {
    bytes: [u8; 32],
    used: usize,
}

impl Ascii {
    fn push(&mut self, byte: u8) {
        if self.used < self.bytes.len() {
            self.bytes[self.used] = byte;
            self.used += 1;
        }
    }

    fn push_all(&mut self, bytes: &[u8]) {
        for byte in bytes {
            self.push(*byte);
        }
    }

    fn push_digits(&mut self, digits: &[u8]) {
        for digit in digits {
            self.push(b'0' + digit);
        }
    }

    fn push_zeros(&mut self, count: i64) {
        (0..count).for_each(|_| self.push(b'0'));
    }

    fn as_str(&self) -> &str {
        core::str::from_utf8(&self.bytes[..self.used]).unwrap_or("")
    }
}

/// Lays out `decimal` as CPython's `repr` does: positional when the decimal exponent is from -4
/// to 15, otherwise scientific with a signed exponent of at least two digits.
fn layout(negative: bool, decimal: &Decimal, text: &mut Ascii) {
    if negative {
        text.push(b'-');
    }
    let digits = decimal.digits();
    let count = i64::try_from(digits.len()).unwrap_or(0);
    let point = decimal.point;
    if point > -4 && point <= 16 {
        if point <= 0 {
            text.push_all(b"0.");
            text.push_zeros(-point);
            text.push_digits(digits);
        } else if point >= count {
            text.push_digits(digits);
            text.push_zeros(point - count);
            text.push_all(b".0");
        } else {
            let split = usize::try_from(point).unwrap_or(0);
            text.push_digits(&digits[..split]);
            text.push(b'.');
            text.push_digits(&digits[split..]);
        }
        return;
    }
    text.push_digits(&digits[..1]);
    if digits.len() > 1 {
        text.push(b'.');
        text.push_digits(&digits[1..]);
    }
    let exponent = point - 1;
    text.push(b'e');
    text.push(if exponent < 0 { b'-' } else { b'+' });
    let size = exponent.unsigned_abs();
    if size < 10 {
        text.push(b'0');
    }
    let mut digits = [0_u8; 3];
    let mut used = 0;
    let mut rest = size;
    while used == 0 || rest > 0 {
        digits[used] = u8::try_from(rest % 10).unwrap_or(0);
        used += 1;
        rest /= 10;
    }
    digits[..used].reverse();
    text.push_digits(&digits[..used]);
}

/// Returns `repr(value)`, a new string owned by the caller.
#[unsafe(no_mangle)]
pub extern "C" fn typy_rt_float_to_str(value: f64) -> *mut StrObject {
    if value.is_nan() {
        return from_str("nan");
    }
    let negative = is_sign_negative(value);
    if value.is_infinite() {
        return from_str(if negative { "-inf" } else { "inf" });
    }
    if value == 0.0 {
        return from_str(if negative { "-0.0" } else { "0.0" });
    }
    let mut text = Ascii {
        bytes: [0; 32],
        used: 0,
    };
    layout(negative, &shortest(crate::libm::absolute(value)), &mut text);
    from_str(text.as_str())
}

#[cfg(test)]
mod tests;
