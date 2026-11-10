//! Lays out a formatted value: groups the digits of a number, pads it to its width with its
//! fill character, and aligns it, as CPython's `fill_number` and `fill_padding` do.

use super::spec::Spec;
use crate::string::{StrBuilder, StrObject};

/// The parts of a formatted number: its sign, its prefix such as `0x`, the digits before any
/// point, which are grouped, and the rest, such as the point, the fraction and the exponent.
pub(super) struct Number<'parts> {
    pub(super) sign: Option<u8>,
    pub(super) prefix: &'parts str,
    pub(super) digits: &'parts [u32],
    pub(super) rest: &'parts [u32],
}

/// Returns the digits with a separator between groups of `size`, counted from the right, and
/// with zeros added on the left, also grouped, until there are at least `min_width`
/// characters, as CPython's `_PyUnicode_InsertThousandsGrouping` adds them.
fn grouped(digits: &[u32], separator: Option<(u32, usize)>, min_width: usize) -> StrBuilder {
    let zero = u32::from(b'0');
    let mut reversed = StrBuilder::with_capacity(digits.len() + min_width + 8);
    let Some((separator, size)) = separator else {
        for _ in digits.len()..min_width {
            reversed.push(zero);
        }
        let mut forward = StrBuilder::with_capacity(reversed.points().len() + digits.len());
        for point in reversed.points().iter().chain(digits) {
            forward.push(*point);
        }
        reversed.discard();
        return forward;
    };
    // `inf` and `nan` have no digits, and so no groups.
    if digits.is_empty() {
        reversed.discard();
        return StrBuilder::with_capacity(0);
    }
    let size = isize::try_from(size).unwrap_or(isize::MAX);
    let mut remaining = isize::try_from(digits.len()).unwrap_or(isize::MAX);
    let mut wanted = isize::try_from(min_width).unwrap_or(isize::MAX);
    let mut taken = 0;
    let mut separate = false;
    loop {
        let length = size.min(remaining.max(wanted).max(1));
        let zeros = (length - remaining).max(0);
        let characters = remaining.min(length).max(0);
        if separate {
            reversed.push(separator);
        }
        for _ in 0..characters {
            taken += 1;
            reversed.push(digits[digits.len() - taken]);
        }
        for _ in 0..zeros {
            reversed.push(zero);
        }
        separate = true;
        remaining -= characters;
        wanted -= length;
        if remaining <= 0 && wanted <= 0 {
            break;
        }
        wanted -= 1;
    }
    let mut forward = StrBuilder::with_capacity(reversed.points().len());
    for point in reversed.points().iter().rev() {
        forward.push(*point);
    }
    reversed.discard();
    forward
}

/// Appends `count` copies of the fill character.
fn fill(text: &mut StrBuilder, point: u32, count: usize) {
    for _ in 0..count {
        text.push(point);
    }
}

/// Returns the padding before and after a value of `length` characters.
fn padding(spec: &Spec, length: usize, default_align: u8) -> (usize, usize) {
    let total = spec.width.saturating_sub(length);
    match spec.align.unwrap_or(default_align) {
        b'<' => (0, total),
        b'^' => (total / 2, total - total / 2),
        _ => (total, 0),
    }
}

/// Returns a number laid out by `spec`; numbers align right unless `spec` says otherwise.
pub(super) fn number(
    number: &Number<'_>,
    spec: &Spec,
    separator: Option<(u32, usize)>,
) -> *mut StrObject {
    let leading = usize::from(number.sign.is_some()) + number.prefix.len();
    // Zeros that fill a number after its sign are digits, grouped like the others.
    let zero_padded = spec.fill == u32::from(b'0') && spec.align == Some(b'=');
    let min_width = if zero_padded && !number.digits.is_empty() {
        spec.width.saturating_sub(leading + number.rest.len())
    } else {
        0
    };
    let digits = grouped(number.digits, separator, min_width);
    let length = leading + digits.points().len() + number.rest.len();
    let mut text = StrBuilder::with_capacity(spec.width.max(length));
    let (before, after) = padding(spec, length, b'>');
    let inside = spec.align == Some(b'=');
    if !inside {
        fill(&mut text, spec.fill, before);
    }
    if let Some(sign) = number.sign {
        text.push(u32::from(sign));
    }
    for byte in number.prefix.bytes() {
        text.push(u32::from(byte));
    }
    if inside {
        fill(&mut text, spec.fill, before);
    }
    for point in digits.points().iter().chain(number.rest) {
        text.push(*point);
    }
    digits.discard();
    fill(&mut text, spec.fill, after);
    text.finish()
}

/// Returns `points` padded to the width of `spec`; text aligns left unless `spec` says
/// otherwise.
pub(super) fn text(points: &[u32], spec: &Spec) -> *mut StrObject {
    let mut text = StrBuilder::with_capacity(spec.width.max(points.len()));
    let (before, after) = padding(spec, points.len(), b'<');
    fill(&mut text, spec.fill, before);
    for point in points {
        text.push(*point);
    }
    fill(&mut text, spec.fill, after);
    text.finish()
}
