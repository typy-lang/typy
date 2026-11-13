//! `format(value, spec)` as f-strings call it (LEX-07): the format specification
//! mini-language of CPython's `formatter_unicode.c` for `int`, `bool`, `float` and `str`, and
//! `object.__format__` for every other type, with CPython's results and messages.

mod float;
mod layout;
mod percent;
mod spec;
#[cfg(test)]
mod tests;

use crate::Raising;
use crate::exception::{ExceptionKind, ExceptionObject, raise};
use crate::int::value::Integer;
use crate::message::MessageBuffer;
use crate::string::{StrBuilder, StrObject, code_point, from_str, length};
use crate::word::{inline_value, is_inline};
use layout::Number;
use spec::{Spec, value_error};

/// Returns the code points of a live string.
///
/// # Safety
///
/// `text` points to a live string.
unsafe fn points(text: *const StrObject) -> StrBuilder {
    // SAFETY: the caller's guarantee; every index is in bounds.
    unsafe {
        let count = length(text);
        let mut points = StrBuilder::with_capacity(count);
        for index in 0..count {
            points.push(code_point(text, index));
        }
        points
    }
}

/// Returns `Unknown format code 'c' for object of type 'name'`, as `ValueError`.
fn unknown_code(kind: u32, type_name: &str) -> *mut ExceptionObject {
    let mut message = MessageBuffer::new();
    message.push_str("Unknown format code '");
    message.push(kind);
    message.push_str("' for object of type '");
    message.push_str(type_name);
    message.push_str("'");
    message.raise(ExceptionKind::ValueError)
}

/// Returns a result or the exception that stopped it, as a runtime function returns it.
fn finish(result: Result<*mut StrObject, *mut ExceptionObject>) -> Raising<*mut StrObject> {
    match result {
        Ok(text) => Raising::ok(text),
        Err(error) => Raising::error(core::ptr::null_mut(), error),
    }
}

/// Returns `format(value, spec)` for an `int` word.
///
/// # Safety
///
/// `value` is an `int` word whose large integer is live, and `spec` points to a live string.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn typy_rt_format_int(
    value: i64,
    spec: *const StrObject,
) -> Raising<*mut StrObject> {
    // SAFETY: the caller's guarantee.
    finish(unsafe { integer(value, spec, "int") })
}

/// Returns `format(value, spec)` for a `bool`, given as the `int` word of 0 or 1, which
/// formats as that `int` except with an empty specification, which gives `True` or `False`.
///
/// # Safety
///
/// `spec` points to a live string.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn typy_rt_format_bool(
    word: i64,
    spec: *const StrObject,
) -> Raising<*mut StrObject> {
    // SAFETY: the caller passes a live string.
    if unsafe { length(spec) } == 0 {
        let value = inline_value(word) != 0;
        return Raising::ok(from_str(if value { "True" } else { "False" }));
    }
    // SAFETY: an inline word has no large integer, and the caller passes a live string.
    finish(unsafe { integer(word, spec, "bool") })
}

/// Returns `format(value, spec)` for a `float`.
///
/// # Safety
///
/// `spec` points to a live string.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn typy_rt_format_float(
    value: f64,
    spec: *const StrObject,
) -> Raising<*mut StrObject> {
    // SAFETY: the caller passes a live string.
    let points = unsafe { points(spec) };
    let result = Spec::parse(points.points(), "float", 0, b'>')
        .and_then(|parsed| float_value(value, &parsed, "float"));
    points.discard();
    finish(result)
}

/// Returns `format(text, spec)` for a `str`.
///
/// # Safety
///
/// Both point to live strings.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn typy_rt_format_str(
    text: *const StrObject,
    spec: *const StrObject,
) -> Raising<*mut StrObject> {
    // SAFETY: the caller passes live strings.
    let (text, spec_points) = unsafe { (points(text), points(spec)) };
    let result = Spec::parse(spec_points.points(), "str", u32::from(b's'), b'<')
        .and_then(|parsed| string(text.points(), &parsed));
    text.discard();
    spec_points.discard();
    finish(result)
}

/// Returns `format(value, spec)` for a value of any other type, given as `str(value)` and
/// its type's name: `object.__format__` gives `str(value)` for an empty specification and
/// refuses any other.
///
/// # Safety
///
/// All three point to live strings.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn typy_rt_format_other(
    text: *mut StrObject,
    spec: *const StrObject,
    type_name: *const StrObject,
) -> Raising<*mut StrObject> {
    // SAFETY: the caller passes live strings.
    unsafe {
        if length(spec) == 0 {
            crate::refcount::typy_rt_increment(text.cast());
            return Raising::ok(text);
        }
        let mut message = MessageBuffer::new();
        message.push_str("unsupported format string passed to ");
        for index in 0..length(type_name) {
            message.push(code_point(type_name, index));
        }
        message.push_str(".__format__");
        Raising::error(
            core::ptr::null_mut(),
            message.raise(ExceptionKind::TypeError),
        )
    }
}

/// Returns `text` with each code point outside ASCII escaped as `\xhh`, `\uhhhh` or
/// `\Uhhhhhhhh`, which makes `ascii(value)` of `repr(value)` (LEX-07).
///
/// # Safety
///
/// `text` points to a live string.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn typy_rt_ascii(text: *const StrObject) -> *mut StrObject {
    // SAFETY: the caller passes a live string.
    let source = unsafe { points(text) };
    let mut escaped = StrBuilder::with_capacity(source.points().len());
    for point in source.points() {
        if *point < 0x80 {
            escaped.push(*point);
        } else {
            crate::codec::escape(*point, &mut |part| escaped.push(part));
        }
    }
    source.discard();
    escaped.finish()
}

/// Formats text: only the type `s`, without a sign, `z`, `#` or `=` alignment.
fn string(text: &[u32], spec: &Spec) -> Result<*mut StrObject, *mut ExceptionObject> {
    if spec.kind != u32::from(b's') {
        return Err(unknown_code(spec.kind, "str"));
    }
    match spec.sign {
        Some(b' ') => return Err(value_error("Space not allowed in string format specifier")),
        Some(_) => return Err(value_error("Sign not allowed in string format specifier")),
        None => {}
    }
    if spec.no_negative_zero {
        return Err(value_error(
            "Negative zero coercion (z) not allowed in string format specifier",
        ));
    }
    if spec.alternate {
        return Err(value_error(
            "Alternate form (#) not allowed in string format specifier",
        ));
    }
    if spec.align == Some(b'=') {
        return Err(value_error(
            "'=' alignment not allowed in string format specifier",
        ));
    }
    let kept = spec
        .precision
        .map_or(text.len(), |precision| precision.min(text.len()));
    Ok(layout::text(&text[..kept], spec))
}

/// Formats an `int` word for the type named `type_name`, `int` or `bool`.
///
/// # Safety
///
/// `value` is an `int` word whose large integer is live, and `spec` points to a live string.
unsafe fn integer(
    value: i64,
    spec: *const StrObject,
    type_name: &str,
) -> Result<*mut StrObject, *mut ExceptionObject> {
    // SAFETY: the caller passes a live string.
    let points = unsafe { points(spec) };
    let parsed = Spec::parse(points.points(), type_name, u32::from(b'd'), b'>');
    points.discard();
    let spec = parsed?;
    let kind = u8::try_from(spec.kind).unwrap_or(0);
    match kind {
        b'b' | b'c' | b'd' | b'o' | b'x' | b'X' | b'n' => {}
        b'e' | b'E' | b'f' | b'F' | b'g' | b'G' | b'%' => {
            // SAFETY: the caller's guarantee.
            let converted = unsafe { crate::float::typy_rt_int_to_float(value) };
            if !converted.exception.is_null() {
                return Err(converted.exception);
            }
            return float_value(converted.value, &spec, type_name);
        }
        _ => return Err(unknown_code(spec.kind, type_name)),
    }
    if spec.precision.is_some() {
        return Err(value_error(
            "Precision not allowed in integer format specifier",
        ));
    }
    if spec.no_negative_zero {
        return Err(value_error(
            "Negative zero coercion (z) not allowed in integer format specifier",
        ));
    }
    // SAFETY: the caller's guarantee.
    let integer = unsafe { whole(value) };
    if kind == b'c' {
        return character(&integer, &spec);
    }
    let (base, prefix, separation) = match kind {
        b'b' => (2, "0b", 4),
        b'o' => (8, "0o", 4),
        b'x' => (16, "0x", 4),
        b'X' => (16, "0X", 4),
        _ => (10, "", 3),
    };
    let digits = in_base(&integer, base, kind == b'X')?;
    let separator = match (spec.grouping, kind) {
        (_, b'n') | (None, _) => None,
        (Some(separator), _) => Some((u32::from(separator), separation)),
    };
    let number = Number {
        sign: sign(integer.negative, &spec),
        prefix: if spec.alternate { prefix } else { "" },
        digits: digits.points(),
        rest: &[],
    };
    let text = layout::number(&number, &spec, separator);
    digits.discard();
    Ok(text)
}

/// Returns the integer of an `int` word.
///
/// # Safety
///
/// `value` is an `int` word whose large integer is live.
unsafe fn whole(value: i64) -> Integer {
    if is_inline(value) {
        Integer::from_i128(i128::from(inline_value(value)))
    } else {
        // SAFETY: the caller's guarantee.
        unsafe { Integer::from_word(value) }
    }
}

/// Returns the sign a number shows: `-` when it is negative, otherwise the one `spec` asks
/// for, if any.
fn sign(negative: bool, spec: &Spec) -> Option<u8> {
    if negative {
        return Some(b'-');
    }
    match spec.sign {
        Some(b'+') => Some(b'+'),
        Some(b' ') => Some(b' '),
        _ => None,
    }
}

/// Formats an integer with the type `c`: the character of that code point.
fn character(integer: &Integer, spec: &Spec) -> Result<*mut StrObject, *mut ExceptionObject> {
    if spec.sign.is_some() {
        return Err(value_error(
            "Sign not allowed with integer format specifier 'c'",
        ));
    }
    if spec.alternate {
        return Err(value_error(
            "Alternate form (#) not allowed with integer format specifier 'c'",
        ));
    }
    let magnitude = integer.magnitude.as_slice();
    if magnitude.len() > 1 || magnitude.first().is_some_and(|low| *low > i64::MAX as u64) {
        return Err(raise(
            ExceptionKind::OverflowError,
            "Python int too large to convert to C long",
        ));
    }
    let point = magnitude.first().copied().unwrap_or(0);
    if integer.negative && point != 0 || point >= 0x11_0000 {
        return Err(raise(
            ExceptionKind::OverflowError,
            "%c arg not in range(0x110000)",
        ));
    }
    let digits = [u32::try_from(point).unwrap_or(0)];
    let number = Number {
        sign: None,
        prefix: "",
        digits: &[],
        rest: &digits,
    };
    Ok(layout::number(&number, spec, None))
}

/// Returns the digits of an integer's magnitude in `base`, as ASCII code points.
fn in_base(integer: &Integer, base: u64, upper: bool) -> Result<StrBuilder, *mut ExceptionObject> {
    let magnitude = integer.magnitude.as_slice();
    let mut digits = StrBuilder::with_capacity(magnitude.len() * 64 + 1);
    if base == 10 {
        let text = decimal_text(integer)?;
        for point in text.points() {
            digits.push(*point);
        }
        text.discard();
        return Ok(digits);
    }
    let bits = match base {
        2 => 1,
        8 => 3,
        _ => 4,
    };
    let total = crate::int::natural::bit_length(magnitude).max(1);
    let count = total.div_ceil(bits);
    let alphabet: &[u8; 16] = if upper {
        b"0123456789ABCDEF"
    } else {
        b"0123456789abcdef"
    };
    for index in (0..count).rev() {
        let mut value = 0;
        for bit in 0..bits {
            let position = index * bits + bit;
            let limb = usize::try_from(position / 64).unwrap_or(usize::MAX);
            let set = magnitude
                .get(limb)
                .is_some_and(|word| (word >> (position % 64)) & 1 == 1);
            value |= u64::from(set) << bit;
        }
        digits.push(u32::from(alphabet[usize::try_from(value).unwrap_or(0)]));
    }
    Ok(digits)
}

/// Returns the decimal digits of an integer's magnitude, with CPython's limit on the number
/// of digits.
fn decimal_text(integer: &Integer) -> Result<StrBuilder, *mut ExceptionObject> {
    let magnitude = Integer {
        negative: false,
        magnitude: crate::int::limbs::Limbs::from_slice(integer.magnitude.as_slice()),
    };
    let word = magnitude.into_word();
    // SAFETY: the word was just made, and its large integer, if any, is live until released.
    let text = unsafe { crate::int::typy_rt_int_to_str(word) };
    // SAFETY: the word holds the only reference to its large integer, if any.
    unsafe { crate::refcount::typy_rt_int_decrement(word) };
    if !text.exception.is_null() {
        return Err(text.exception);
    }
    // SAFETY: the runtime returned a live string, released once it is copied.
    unsafe {
        let copied = points(text.value);
        crate::refcount::typy_rt_decrement(text.value.cast());
        Ok(copied)
    }
}

/// Formats a `float`, or an `int` given a float type.
fn float_value(
    value: f64,
    spec: &Spec,
    type_name: &str,
) -> Result<*mut StrObject, *mut ExceptionObject> {
    let kind = u8::try_from(spec.kind).unwrap_or(1);
    let mut options = float::Options {
        code: kind.to_ascii_lowercase(),
        precision: spec.precision.unwrap_or(6),
        alternate: spec.alternate,
        add_dot_zero: false,
        no_negative_zero: spec.no_negative_zero,
        upper: matches!(kind, b'E' | b'F' | b'G'),
    };
    let mut value = value;
    let mut percent = false;
    match kind {
        0 => {
            options.add_dot_zero = true;
            if spec.precision.is_some() {
                options.code = b'g';
            } else {
                options.code = b'r';
            }
        }
        b'n' => options.code = b'g',
        b'%' => {
            options.code = b'f';
            value *= 100.0;
            percent = true;
        }
        b'e' | b'E' | b'f' | b'F' | b'g' | b'G' => {}
        _ => return Err(unknown_code(spec.kind, type_name)),
    }
    let (negative, mut text) = float::write(value, &options);
    if percent {
        text.push(u32::from(b'%'));
    }
    let points = text.points();
    let split = points
        .iter()
        .position(|point| !(u32::from(b'0')..=u32::from(b'9')).contains(point))
        .unwrap_or(points.len());
    let number = Number {
        sign: sign(negative, spec),
        prefix: "",
        digits: &points[..split],
        rest: &points[split..],
    };
    let separator = match (spec.grouping, kind) {
        (_, b'n') | (None, _) => None,
        (Some(separator), _) => Some((u32::from(separator), 3)),
    };
    let formatted = layout::number(&number, spec, separator);
    text.discard();
    Ok(formatted)
}
