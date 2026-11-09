//! `str` objects: a sequence of code points stored at 1, 2 or 4 bytes each, the narrowest that holds them all (PEP 393, DAT-05).

mod builder;
mod case;
mod character;
mod methods;
mod operations;
mod points;
mod repr;
mod split;
mod unicode;
mod utf8;

use crate::allocator::{allocate, array_size};
use crate::header::{ObjectHeader, STR_TYPE_ID};

pub(crate) use operations::typy_rt_str_compare;
pub(crate) use points::{Points, is_space};
pub(crate) use repr::{repr_into, typy_rt_str_repr};
pub(crate) use unicode::decimal_value;
pub(crate) use utf8::write_utf8;

/// The width of each code point of a string.
#[repr(u32)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum StrKind {
    /// One byte: code points up to U+00FF.
    Latin1 = 1,
    /// Two bytes: code points up to U+FFFF.
    Ucs2 = 2,
    /// Four bytes.
    Ucs4 = 4,
}

impl StrKind {
    /// Returns the narrowest kind that holds `point`.
    #[must_use]
    pub const fn of(point: u32) -> Self {
        if point <= 0xFF {
            Self::Latin1
        } else if point <= 0xFFFF {
            Self::Ucs2
        } else {
            Self::Ucs4
        }
    }
}

/// The fixed part of a `str` object; the code points follow it (detailed design section 10.2).
#[repr(C)]
#[derive(Debug)]
pub struct StrObject {
    /// The object header.
    pub header: ObjectHeader,
    /// The number of code points.
    pub length: u64,
    /// The width of each code point.
    pub kind: StrKind,
    padding: u32,
}

const DATA_OFFSET: usize = core::mem::size_of::<StrObject>();

/// Returns a new string of `length` code points of `kind`, with its data not yet written.
pub(crate) fn new_string(length: usize, kind: StrKind) -> *mut StrObject {
    let size = array_size(length, kind as usize, DATA_OFFSET);
    #[allow(
        clippy::cast_ptr_alignment,
        reason = "malloc returns memory aligned for every type"
    )]
    let object = allocate(size).cast::<StrObject>();
    // SAFETY: `object` points to `size` writable bytes, which hold the fixed part.
    unsafe {
        object.write(StrObject {
            header: ObjectHeader::new(STR_TYPE_ID),
            length: length as u64,
            kind,
            padding: 0,
        });
    }
    object
}

/// Returns the number of code points of a string.
///
/// # Safety
///
/// `string` points to a live `str` object.
pub(crate) unsafe fn length(string: *const StrObject) -> usize {
    // SAFETY: the caller passes a live string.
    usize::try_from(unsafe { (*string).length }).unwrap_or(usize::MAX)
}

/// Returns the code point at `index`.
///
/// # Safety
///
/// `string` points to a live `str` object and `index` is below its length.
#[allow(
    clippy::cast_ptr_alignment,
    reason = "the data is read with read_unaligned"
)]
pub(crate) unsafe fn code_point(string: *const StrObject, index: usize) -> u32 {
    // SAFETY: the data follows the fixed part, and `index` is in bounds by the caller's guarantee.
    unsafe {
        let data = string.cast::<u8>().add(DATA_OFFSET);
        match (*string).kind {
            StrKind::Latin1 => u32::from(*data.add(index)),
            StrKind::Ucs2 => u32::from(data.cast::<u16>().add(index).read_unaligned()),
            StrKind::Ucs4 => data.cast::<u32>().add(index).read_unaligned(),
        }
    }
}

/// Writes the code point at `index`, which must fit the string's kind.
///
/// # Safety
///
/// `string` points to a new `str` object, `index` is below its length, and `point` fits its kind.
#[allow(
    clippy::cast_ptr_alignment,
    reason = "the data is written with write_unaligned"
)]
pub(crate) unsafe fn set_code_point(string: *mut StrObject, index: usize, point: u32) {
    // SAFETY: the data follows the fixed part, and the caller keeps `index` in bounds and `point` within the kind.
    unsafe {
        let data = string.cast::<u8>().add(DATA_OFFSET);
        match (*string).kind {
            StrKind::Latin1 => *data.add(index) = u8::try_from(point).unwrap_or(0),
            StrKind::Ucs2 => data
                .cast::<u16>()
                .add(index)
                .write_unaligned(u16::try_from(point).unwrap_or(0)),
            StrKind::Ucs4 => data.cast::<u32>().add(index).write_unaligned(point),
        }
    }
}

/// Returns a new string of the code points of `text`.
pub(crate) fn from_str(text: &str) -> *mut StrObject {
    let kind = text
        .chars()
        .map(|point| StrKind::of(u32::from(point)))
        .max()
        .unwrap_or(StrKind::Latin1);
    let string = new_string(text.chars().count(), kind);
    for (index, point) in text.chars().enumerate() {
        // SAFETY: the string is new, the index is in bounds, and the kind holds every code point.
        unsafe { set_code_point(string, index, u32::from(point)) };
    }
    string
}

#[cfg(test)]
mod tests {
    use super::{StrKind, code_point, from_str, length};

    fn points(string: *const super::StrObject) -> Vec<u32> {
        // SAFETY: the string is live and the indexes are in bounds.
        (0..unsafe { length(string) })
            .map(|index| unsafe { code_point(string, index) })
            .collect()
    }

    #[test]
    fn strings_hold_their_code_points_in_the_narrowest_kind() {
        for (text, kind) in [
            ("abc", StrKind::Latin1),
            ("caf\u{e9}", StrKind::Latin1),
            ("\u{3b1}x", StrKind::Ucs2),
            ("a\u{1f600}", StrKind::Ucs4),
            ("", StrKind::Latin1),
        ] {
            let string = from_str(text);
            // SAFETY: the string was just created.
            assert_eq!(unsafe { (*string).kind }, kind);
            assert_eq!(
                points(string),
                text.chars().map(u32::from).collect::<Vec<_>>()
            );
        }
    }

    #[test]
    fn the_fixed_part_takes_twenty_four_bytes() {
        assert_eq!(super::DATA_OFFSET, 24);
    }
}
