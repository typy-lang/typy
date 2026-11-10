//! Writes a `float` as CPython's `PyOS_double_to_string` does for the `e`, `f`, `g` and `r`
//! codes: the digits come from the exact value of the double, rounded half to even, as
//! `_Py_dg_dtoa` gives them, and are laid out as `format_float_short` lays them out.

use crate::float::{power_of_ten, round_quotient, shortest};
use crate::int::limbs::Limbs;
use crate::int::natural::{compare, divide_small, shift_left};
use crate::string::StrBuilder;

/// Decimal digits, most significant first, without trailing zeros, and where the point is:
/// the value is `0.digits * 10^point`.
struct Digits {
    digits: StrBuilder,
    point: i64,
}

/// What to write besides the digits.
#[derive(Clone, Copy, Debug)]
#[allow(
    clippy::struct_excessive_bools,
    reason = "each is one of the independent flags of CPython's PyOS_double_to_string"
)]
pub(super) struct Options {
    /// `e`, `f`, `g` or `r`, the last for `repr`'s shortest digits.
    pub(super) code: u8,
    /// Digits after the point for `e` and `f`, significant digits for `g`.
    pub(super) precision: usize,
    /// `#`: keep the point and, for `g`, the trailing zeros.
    pub(super) alternate: bool,
    /// Add `.0` to a value written as an integer, as `repr` does.
    pub(super) add_dot_zero: bool,
    /// Drop the sign of a value that rounds to zero.
    pub(super) no_negative_zero: bool,
    /// Write `E`, `INF` and `NAN` in capitals.
    pub(super) upper: bool,
}

/// Returns the exact value of a finite, positive double as `mantissa * 2^exponent`.
fn parts(value: f64) -> (u64, i64) {
    let bits = value.to_bits();
    let exponent = i64::try_from((bits >> 52) & 0x7ff).unwrap_or(0);
    let fraction = bits & ((1 << 52) - 1);
    if exponent == 0 {
        (fraction, -1074)
    } else {
        (fraction | (1 << 52), exponent - 1075)
    }
}

/// Returns `value * 10^power` rounded to an integer, a half to the even one.
fn scaled(value: f64, power: i64) -> Limbs {
    let (mantissa, exponent) = parts(value);
    let mut numerator = Limbs::from_slice(&[mantissa]);
    let mut denominator = Limbs::from_slice(&[1]);
    if exponent >= 0 {
        numerator = shift_left(numerator.as_slice(), exponent.unsigned_abs());
    } else {
        denominator = shift_left(denominator.as_slice(), exponent.unsigned_abs());
    }
    let ten = power_of_ten(power.unsigned_abs());
    if power >= 0 {
        numerator = crate::int::natural::multiply(numerator.as_slice(), ten.as_slice());
    } else {
        denominator = crate::int::natural::multiply(denominator.as_slice(), ten.as_slice());
    }
    round_quotient(numerator.as_slice(), denominator.as_slice())
}

/// Returns the decimal digits of a natural number, most significant first; zero has none.
fn decimal(value: &[u64]) -> StrBuilder {
    const CHUNK: u64 = 10_000_000_000_000_000_000;
    let mut chunks = Limbs::new();
    let mut rest = Limbs::from_slice(value);
    while !rest.is_zero() {
        let (quotient, remainder) = divide_small(rest.as_slice(), CHUNK);
        chunks.push(remainder);
        rest = quotient;
    }
    let mut digits = StrBuilder::with_capacity(chunks.as_slice().len() * 19);
    for (index, chunk) in chunks.as_slice().iter().rev().enumerate() {
        let mut buffer = [0_u8; 19];
        let mut value = *chunk;
        for slot in buffer.iter_mut().rev() {
            *slot = u8::try_from(value % 10).unwrap_or(0);
            value /= 10;
        }
        let start = if index == 0 {
            buffer.iter().position(|digit| *digit != 0).unwrap_or(19)
        } else {
            0
        };
        for digit in &buffer[start..] {
            digits.push(u32::from(*digit));
        }
    }
    digits
}

/// Returns digits without their trailing zeros.
fn trimmed(digits: StrBuilder) -> StrBuilder {
    let points = digits.points();
    let end = points
        .iter()
        .rposition(|digit| *digit != 0)
        .map_or(0, |last| last + 1);
    let mut kept = StrBuilder::with_capacity(end);
    for digit in &points[..end] {
        kept.push(*digit);
    }
    digits.discard();
    kept
}

/// Returns the shortest digits that read back as `value`, positive and finite.
fn shortest_digits(value: f64) -> Digits {
    let decimal = shortest(value);
    let mut digits = StrBuilder::with_capacity(17);
    for digit in decimal.digits() {
        digits.push(u32::from(*digit));
    }
    Digits {
        digits,
        point: decimal.point(),
    }
}

/// Returns `count` significant digits of `value`, positive and finite, rounded half to even.
fn significant(value: f64, count: usize) -> Digits {
    let count = i64::try_from(count).unwrap_or(i64::MAX);
    let mut point = shortest(value).point();
    loop {
        let rounded = scaled(value, count - point);
        let low = power_of_ten((count - 1).unsigned_abs());
        let high = power_of_ten(count.unsigned_abs());
        if compare(rounded.as_slice(), low.as_slice()).is_lt() {
            point -= 1;
            continue;
        }
        match compare(rounded.as_slice(), high.as_slice()) {
            core::cmp::Ordering::Greater => point += 1,
            core::cmp::Ordering::Equal => {
                let mut digits = StrBuilder::with_capacity(1);
                digits.push(1);
                return Digits {
                    digits,
                    point: point + 1,
                };
            }
            core::cmp::Ordering::Less => {
                return Digits {
                    digits: trimmed(decimal(rounded.as_slice())),
                    point,
                };
            }
        }
    }
}

/// Returns the digits of `value`, positive and finite, rounded half to even at `count` places
/// after the point; a value that rounds to zero has no digits.
fn fixed(value: f64, count: usize) -> Digits {
    let count = i64::try_from(count).unwrap_or(i64::MAX);
    let rounded = scaled(value, count);
    let digits = decimal(rounded.as_slice());
    let length = i64::try_from(digits.points().len()).unwrap_or(0);
    let point = if length == 0 { -count } else { length - count };
    Digits {
        digits: trimmed(digits),
        point,
    }
}

/// Appends `count` zeros.
fn zeros(text: &mut StrBuilder, count: i64) {
    for _ in 0..count.max(0) {
        text.push(u32::from(b'0'));
    }
}

/// Returns the digits `options` asks for of a finite, non-negative value, with the precision
/// as `PyOS_double_to_string` adjusts it.
fn digits_of(magnitude: f64, options: &Options) -> (Digits, usize) {
    let precision = match options.code {
        b'e' => options.precision + 1,
        b'g' => options.precision.max(1),
        _ => options.precision,
    };
    if magnitude == 0.0 {
        let mut zero = StrBuilder::with_capacity(1);
        zero.push(0);
        let digits = Digits {
            digits: zero,
            point: 1,
        };
        return (digits, precision);
    }
    let digits = match options.code {
        b'r' => shortest_digits(magnitude),
        b'f' => fixed(magnitude, precision),
        _ => significant(magnitude, precision),
    };
    (digits, precision)
}

/// Appends `text` in the case `options` asks for.
fn push_ascii(text: &mut StrBuilder, ascii: &str, options: &Options) {
    for byte in ascii.bytes() {
        let byte = if options.upper {
            byte.to_ascii_uppercase()
        } else {
            byte
        };
        text.push(u32::from(byte));
    }
}

/// Appends an exponent as C's `%+.02d` writes it: a sign and at least two digits.
fn push_exponent(text: &mut StrBuilder, exponent: i64) {
    text.push(u32::from(if exponent < 0 { b'-' } else { b'+' }));
    let magnitude = exponent.unsigned_abs();
    if magnitude < 10 {
        text.push(u32::from(b'0'));
    }
    let mut buffer = [0_u8; 20];
    let mut cursor = buffer.len();
    let mut rest = magnitude;
    loop {
        cursor -= 1;
        buffer[cursor] = b'0' + u8::try_from(rest % 10).unwrap_or(0);
        rest /= 10;
        if rest == 0 {
            break;
        }
    }
    for byte in &buffer[cursor..] {
        text.push(u32::from(*byte));
    }
}

/// Where `format_float_short` puts the point: whether an exponent is written, the exponent,
/// the point's position and the position the zeros on the right run to.
fn placement(options: &Options, precision: i64, length: i64, point: i64) -> (bool, i64, i64, i64) {
    let mut use_exponent = false;
    let mut end = length;
    match options.code {
        b'e' => {
            use_exponent = true;
            end = precision;
        }
        b'f' => end = point + precision,
        b'g' => {
            let limit = if options.add_dot_zero {
                precision - 1
            } else {
                precision
            };
            use_exponent = point <= -4 || point > limit;
            if options.alternate {
                end = precision;
            }
        }
        _ => use_exponent = point <= -4 || point > 16,
    }
    let (exponent, point) = if use_exponent {
        (point - 1, 1)
    } else {
        (0, point)
    };
    let end = if !use_exponent && options.add_dot_zero {
        end.max(point + 1)
    } else {
        end.max(point)
    };
    (use_exponent, exponent, point, end)
}

/// Returns the digits laid out with zeros and a point, as `format_float_short` lays them out.
fn laid_out(digits: &[u32], point: i64, end: i64) -> StrBuilder {
    let length = i64::try_from(digits.len()).unwrap_or(0);
    let start = if point <= 0 { point - 1 } else { 0 };
    let mut text = StrBuilder::with_capacity(32);
    let digit = |text: &mut StrBuilder, value: u32| text.push(u32::from(b'0') + value);
    if point <= 0 {
        zeros(&mut text, point - start);
        text.push(u32::from(b'.'));
        zeros(&mut text, -point);
    } else {
        zeros(&mut text, -start);
    }
    if point > 0 && point <= length {
        let split = usize::try_from(point).unwrap_or(0);
        for value in &digits[..split] {
            digit(&mut text, *value);
        }
        text.push(u32::from(b'.'));
        for value in &digits[split..] {
            digit(&mut text, *value);
        }
    } else {
        for value in digits {
            digit(&mut text, *value);
        }
    }
    if length < point {
        zeros(&mut text, point - length);
        text.push(u32::from(b'.'));
        zeros(&mut text, end - point);
    } else {
        zeros(&mut text, end - length);
    }
    text
}

/// Returns `value` written by `options`, and whether it takes a minus sign, which is not
/// written.
pub(super) fn write(value: f64, options: &Options) -> (bool, StrBuilder) {
    // CPython ignores the sign of a NaN.
    if value.is_nan() {
        let mut text = StrBuilder::with_capacity(3);
        push_ascii(&mut text, "nan", options);
        return (false, text);
    }
    let mut negative = value.is_sign_negative();
    if value.is_infinite() {
        let mut text = StrBuilder::with_capacity(3);
        push_ascii(&mut text, "inf", options);
        return (negative, text);
    }
    let (Digits { digits, point }, precision) = digits_of(value.abs(), options);
    let length = i64::try_from(digits.points().len()).unwrap_or(0);
    if options.no_negative_zero && negative && (length == 0 || digits.points() == [0]) {
        negative = false;
    }
    let precision = i64::try_from(precision).unwrap_or(i64::MAX);
    let (use_exponent, exponent, point, end) = placement(options, precision, length, point);
    let mut text = laid_out(digits.points(), point, end);
    digits.discard();
    if text.points().last() == Some(&u32::from(b'.')) && !options.alternate {
        let mut shorter = StrBuilder::with_capacity(text.points().len());
        for point in &text.points()[..text.points().len() - 1] {
            shorter.push(*point);
        }
        text.discard();
        text = shorter;
    }
    if use_exponent {
        push_ascii(&mut text, "e", options);
        push_exponent(&mut text, exponent);
    }
    (negative, text)
}
