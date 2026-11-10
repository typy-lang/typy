//! Reads a format specification, `[[fill]align][sign][z][#][0][width][grouping][.precision][type]`,
//! with the checks and messages of CPython's `parse_internal_render_format_spec`.

use crate::exception::{ExceptionKind, ExceptionObject};
use crate::message::MessageBuffer;

/// A format specification after reading.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct Spec {
    /// The character that pads the value to its width.
    pub(super) fill: u32,
    /// `<`, `>`, `^` or `=`, when one is given.
    pub(super) align: Option<u8>,
    /// `+`, `-` or a space, when one is given.
    pub(super) sign: Option<u8>,
    /// `z`: a negative zero is written without its sign.
    pub(super) no_negative_zero: bool,
    /// `#`: the alternate form.
    pub(super) alternate: bool,
    /// The least number of characters.
    pub(super) width: usize,
    /// `,` or `_`, which separate groups of digits.
    pub(super) grouping: Option<u8>,
    /// The digits after the point, or the characters kept of a string.
    pub(super) precision: Option<usize>,
    /// The presentation type, or the type's default; 0 for a `float` without one.
    pub(super) kind: u32,
}

/// Raises `ValueError` with `message`.
pub(super) fn value_error(message: &str) -> *mut ExceptionObject {
    crate::exception::raise(ExceptionKind::ValueError, message)
}

fn is_alignment(point: u32) -> bool {
    matches!(point, 0x3c | 0x3e | 0x5e | 0x3d)
}

/// Returns the code point as a byte when it is ASCII.
fn ascii(point: u32) -> Option<u8> {
    u8::try_from(point).ok().filter(u8::is_ascii)
}

/// Reads the decimal digits at `position`, if any; a number too large for a width is an error.
fn integer(points: &[u32], position: &mut usize) -> Result<Option<usize>, *mut ExceptionObject> {
    let start = *position;
    let mut value: usize = 0;
    while let Some(digit) = points
        .get(*position)
        .and_then(|point| char::from_u32(*point))
        .and_then(|character| character.to_digit(10))
    {
        value = value
            .checked_mul(10)
            .and_then(|value| value.checked_add(digit as usize))
            .filter(|value| i64::try_from(*value).is_ok())
            .ok_or_else(|| value_error("Too many decimal digits in format string"))?;
        *position += 1;
    }
    Ok((*position > start).then_some(value))
}

impl Spec {
    /// Reads `points` for an object of the type `type_name`, whose presentation type defaults
    /// to `default_kind` and whose alignment defaults to `default_align`.
    pub(super) fn parse(
        points: &[u32],
        type_name: &str,
        default_kind: u32,
        default_align: u8,
    ) -> Result<Self, *mut ExceptionObject> {
        let mut spec = Self {
            fill: u32::from(b' '),
            align: None,
            sign: None,
            no_negative_zero: false,
            alternate: false,
            width: 0,
            grouping: None,
            precision: None,
            kind: default_kind,
        };
        let mut position = 0;
        let mut fill_given = false;
        if points.len() >= 2 && is_alignment(points[1]) {
            spec.fill = points[0];
            spec.align = ascii(points[1]);
            fill_given = true;
            position = 2;
        } else if points.first().is_some_and(|point| is_alignment(*point)) {
            spec.align = ascii(points[0]);
            position = 1;
        }
        let at = |position: usize| points.get(position).copied().and_then(ascii);
        if let Some(sign @ (b'+' | b'-' | b' ')) = at(position) {
            spec.sign = Some(sign);
            position += 1;
        }
        if at(position) == Some(b'z') {
            spec.no_negative_zero = true;
            position += 1;
        }
        if at(position) == Some(b'#') {
            spec.alternate = true;
            position += 1;
        }
        // A `0` before the width pads with zeros after the sign, unless a fill is given.
        if !fill_given && at(position) == Some(b'0') {
            spec.fill = u32::from(b'0');
            if spec.align.is_none() && default_align == b'>' {
                spec.align = Some(b'=');
            }
            position += 1;
        }
        spec.width = integer(points, &mut position)?.unwrap_or(0);
        if at(position) == Some(b',') {
            spec.grouping = Some(b',');
            position += 1;
        }
        if at(position) == Some(b'_') {
            if spec.grouping.is_some() {
                return Err(value_error("Cannot specify both ',' and '_'."));
            }
            spec.grouping = Some(b'_');
            position += 1;
        }
        if at(position) == Some(b',') {
            return Err(value_error("Cannot specify both ',' and '_'."));
        }
        if at(position) == Some(b'.') {
            position += 1;
            spec.precision = Some(
                integer(points, &mut position)?
                    .ok_or_else(|| value_error("Format specifier missing precision"))?,
            );
        }
        if points.len() - position > 1 {
            let mut message = MessageBuffer::new();
            message.push_str("Invalid format specifier '");
            for point in points {
                message.push(*point);
            }
            message.push_str("' for object of type '");
            message.push_str(type_name);
            message.push_str("'");
            return Err(message.raise(ExceptionKind::ValueError));
        }
        if let Some(kind) = points.get(position) {
            spec.kind = *kind;
        }
        if let Some(separator) = spec.grouping {
            let allowed = match ascii(spec.kind) {
                Some(b'd' | b'e' | b'f' | b'g' | b'E' | b'G' | b'%' | b'F') => true,
                Some(b'b' | b'o' | b'x' | b'X') => separator == b'_',
                _ => spec.kind == 0,
            };
            if !allowed {
                let mut message = MessageBuffer::new();
                message.push_str("Cannot specify '");
                message.push(u32::from(separator));
                message.push_str("' with '");
                message.push(spec.kind);
                message.push_str("'.");
                return Err(message.raise(ExceptionKind::ValueError));
            }
        }
        Ok(spec)
    }
}
