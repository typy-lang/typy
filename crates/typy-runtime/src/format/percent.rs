//! `str % values`, printf-style formatting (LIB-10): each conversion is given its flags, width
//! and precision as text, such as `-08.3`, and is laid out by the same code as `format()`,
//! with the rules of CPython's `unicode_format_arg_output` and `formatlong`.

use crate::Raising;
use crate::exception::ExceptionKind;
use crate::message::MessageBuffer;
use crate::string::{StrBuilder, StrObject, length};
use crate::word::inline_value;

use super::layout::{self, Number};
use super::spec::Spec;
use super::{finish, float_value, in_base, points, sign, string, whole};

/// The flags, width and precision of one conversion.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[allow(
    clippy::struct_excessive_bools,
    reason = "the five flags of printf are independent of each other"
)]
struct Printf {
    /// `-`: the value goes on the left of its width.
    left: bool,
    /// `+`: a sign also before a positive number.
    plus: bool,
    /// ` `: a space before a positive number.
    space: bool,
    /// `#`: the alternate form.
    alternate: bool,
    /// `0`: zeros after the sign fill the width of a number.
    zero: bool,
    width: usize,
    /// A negative precision from `*` counts as 0, as CPython takes it.
    precision: Option<usize>,
}

impl Printf {
    /// Reads `[flags][width][.precision]`. A width from `*` may be negative, which is the `-`
    /// flag with its magnitude, so a `-` may also come after the flags.
    fn parse(points: &[u32]) -> Self {
        let mut printf = Self::default();
        let mut position = 0;
        while let Some(flag) = points
            .get(position)
            .and_then(|point| u8::try_from(*point).ok())
        {
            match flag {
                b'-' => printf.left = true,
                b'+' => printf.plus = true,
                b' ' => printf.space = true,
                b'#' => printf.alternate = true,
                b'0' => printf.zero = true,
                _ => break,
            }
            position += 1;
        }
        printf.width = number(points, &mut position);
        if points.get(position) == Some(&u32::from(b'.')) {
            position += 1;
            let negative = points.get(position) == Some(&u32::from(b'-'));
            position += usize::from(negative);
            let precision = number(points, &mut position);
            printf.precision = Some(if negative { 0 } else { precision });
        }
        printf
    }

    /// Returns the format specification that lays out a number with these flags.
    fn number_spec(self, kind: u8) -> Spec {
        let zero = self.zero && !self.left;
        Spec {
            fill: u32::from(if zero { b'0' } else { b' ' }),
            align: Some(if self.left {
                b'<'
            } else if zero {
                b'='
            } else {
                b'>'
            }),
            sign: if self.plus {
                Some(b'+')
            } else if self.space {
                Some(b' ')
            } else {
                None
            },
            no_negative_zero: false,
            alternate: self.alternate,
            width: self.width,
            grouping: None,
            precision: self.precision,
            kind: u32::from(kind),
        }
    }

    /// Returns the format specification that lays out text: flags other than `-` do nothing.
    fn text_spec(self) -> Spec {
        Spec {
            sign: None,
            alternate: false,
            fill: u32::from(b' '),
            align: Some(if self.left { b'<' } else { b'>' }),
            ..self.number_spec(b's')
        }
    }
}

/// Reads decimal digits at `position`; a width too large for memory is held as the largest
/// one, which fails when the text is made.
fn number(points: &[u32], position: &mut usize) -> usize {
    let mut value: usize = 0;
    while let Some(digit) = points
        .get(*position)
        .and_then(|point| char::from_u32(*point))
        .and_then(|character| character.to_digit(10))
    {
        value = value.saturating_mul(10).saturating_add(digit as usize);
        *position += 1;
    }
    value
}

/// Returns the flags of a live string.
///
/// # Safety
///
/// `spec` points to a live string.
unsafe fn read(spec: *const StrObject) -> Printf {
    // SAFETY: the caller's guarantee.
    let text = unsafe { points(spec) };
    let printf = Printf::parse(text.points());
    text.discard();
    printf
}

/// Returns `%d`, `%i`, `%u`, `%x`, `%X` or `%o` of an `int` word: its digits, at least as many
/// as the precision, after its sign and, with `#`, `0x`, `0X` or `0o`. The conversion is the
/// `int` word of its character.
///
/// # Safety
///
/// `value` is an `int` word whose large integer is live, and `spec` points to a live string.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn typy_rt_percent_int(
    value: i64,
    spec: *const StrObject,
    conversion: i64,
) -> Raising<*mut StrObject> {
    // SAFETY: the caller's guarantee.
    let (printf, integer) = unsafe { (read(spec), whole(value)) };
    let kind = u8::try_from(inline_value(conversion)).unwrap_or(b'd');
    let (base, prefix) = match kind {
        b'x' => (16, "0x"),
        b'X' => (16, "0X"),
        b'o' => (8, "0o"),
        _ => (10, ""),
    };
    let digits = match in_base(&integer, base, kind == b'X') {
        Ok(digits) => digits,
        Err(error) => return finish(Err(error)),
    };
    let precision = printf.precision.unwrap_or(0);
    let mut padded = StrBuilder::with_capacity(precision.max(digits.points().len()));
    for _ in digits.points().len()..precision {
        padded.push(u32::from(b'0'));
    }
    for point in digits.points() {
        padded.push(*point);
    }
    digits.discard();
    let layout = Spec {
        precision: None,
        ..printf.number_spec(kind)
    };
    let number = Number {
        sign: sign(integer.negative, &layout),
        prefix: if printf.alternate { prefix } else { "" },
        digits: padded.points(),
        rest: &[],
    };
    let text = layout::number(&number, &layout, None);
    padded.discard();
    Raising::ok(text)
}

/// Returns `%e`, `%E`, `%f`, `%F`, `%g` or `%G` of a `float`, with 6 digits unless a
/// precision is given, as `format()` gives them. The conversion is the `int` word of its
/// character.
///
/// # Safety
///
/// `spec` points to a live string.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn typy_rt_percent_float(
    value: f64,
    spec: *const StrObject,
    conversion: i64,
) -> Raising<*mut StrObject> {
    // SAFETY: the caller's guarantee.
    let printf = unsafe { read(spec) };
    let kind = u8::try_from(inline_value(conversion)).unwrap_or(b'f');
    let layout = Spec {
        precision: Some(printf.precision.unwrap_or(6)),
        ..printf.number_spec(kind)
    };
    finish(float_value(value, &layout, "float"))
}

/// Returns `%s`, `%r` or `%a` of text already converted: at most the precision's code
/// points, padded to the width.
///
/// # Safety
///
/// Both point to live strings.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn typy_rt_percent_str(
    text: *const StrObject,
    spec: *const StrObject,
) -> Raising<*mut StrObject> {
    // SAFETY: the caller's guarantee.
    let (printf, text) = unsafe { (read(spec), points(text)) };
    let result = string(text.points(), &printf.text_spec());
    text.discard();
    finish(result)
}

/// Returns `%c` of an `int` word: the character of that code point. `label` names the value
/// in an error, such as `format argument 2`.
///
/// # Safety
///
/// `value` is an `int` word whose large integer is live, and `spec` and `label` point to
/// live strings.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn typy_rt_percent_char(
    value: i64,
    spec: *const StrObject,
    label: *const StrObject,
) -> Raising<*mut StrObject> {
    // SAFETY: the caller's guarantee.
    let (printf, integer) = unsafe { (read(spec), whole(value)) };
    let magnitude = integer.magnitude.as_slice();
    let point = match magnitude {
        [] => Some(0),
        [low] if !integer.negative => u32::try_from(*low).ok().filter(|point| *point < 0x11_0000),
        _ => None,
    };
    let Some(point) = point else {
        let mut message = MessageBuffer::new();
        // SAFETY: the caller passes a live string.
        unsafe { message.push_text(label) };
        message.push_str(": %c argument not in range(0x110000)");
        return finish(Err(message.raise(ExceptionKind::OverflowError)));
    };
    Raising::ok(layout::text(&[point], &printf.text_spec()))
}

/// Returns `%c` of a `str`, which must be one character; `label` names the value in an
/// error.
///
/// # Safety
///
/// All three point to live strings.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn typy_rt_percent_char_str(
    text: *const StrObject,
    spec: *const StrObject,
    label: *const StrObject,
) -> Raising<*mut StrObject> {
    // SAFETY: the caller's guarantee.
    let count = unsafe { length(text) };
    if count != 1 {
        let mut message = MessageBuffer::new();
        // SAFETY: the caller passes a live string.
        unsafe { message.push_text(label) };
        message
            .push_str(": %c requires an integer or a unicode character, not a string of length ");
        message.push_number(i64::try_from(count).unwrap_or(i64::MAX));
        return finish(Err(message.raise(ExceptionKind::TypeError)));
    }
    // SAFETY: the caller's guarantee; a precision does nothing to one character.
    let (printf, text) = unsafe { (read(spec), points(text)) };
    let result = layout::text(text.points(), &printf.text_spec());
    text.discard();
    Raising::ok(result)
}

#[cfg(test)]
mod tests;
