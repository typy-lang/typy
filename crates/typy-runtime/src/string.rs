//! `str` objects: a sequence of code points stored at 1, 2 or 4 bytes each, the narrowest that holds them all (PEP 393, DAT-05).

mod builder;
mod case;
mod character;
mod methods;
pub(crate) mod narrow;
mod operations;
#[cfg(test)]
pub(crate) use operations::typy_rt_str_index;
mod points;
mod repr;
mod split;
mod unicode;
mod utf8;

use core::sync::atomic::AtomicI64;

use crate::allocator::{allocate, array_size};
use crate::header::{ObjectHeader, STR_TYPE_ID};

pub(crate) use builder::StrBuilder;
pub(crate) use character::typy_rt_chr;
pub(crate) use methods::{
    typy_rt_str_expandtabs, typy_rt_str_isascii, typy_rt_str_match, typy_rt_str_match_any,
    typy_rt_str_pad, typy_rt_str_remove_affix, typy_rt_str_replace, typy_rt_str_search,
    typy_rt_str_strip, typy_rt_str_zfill,
};
pub(crate) use operations::{
    typy_rt_str_compare, typy_rt_str_concat, typy_rt_str_contains, typy_rt_str_length,
    typy_rt_str_repeat, typy_rt_str_slice,
};
pub(crate) use points::{Points, is_space};
pub(crate) use repr::{repr_into, typy_rt_str_repr};
pub(crate) use split::{
    typy_rt_str_join, typy_rt_str_partition, typy_rt_str_split, typy_rt_str_splitlines,
};
pub(crate) use unicode::decimal_value;
pub(crate) use utf8::{Surrogates, unescapable, write_utf8};

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
    /// The hash of the string, or `NOT_HASHED` until it is first hashed (ADR-0101).
    pub(crate) hash: AtomicI64,
}

/// What `hash` holds until the string is first hashed: -1, which no hash is.
pub(crate) const NOT_HASHED: i64 = -1;

const DATA_OFFSET: usize = core::mem::size_of::<StrObject>();

/// An immortal `str` of at most one code point of one byte, with the zero unit after it.
#[repr(C)]
struct Shared {
    object: StrObject,
    data: [u8; 8],
}

/// Returns the immortal `str` of the code point `point`, or the empty one for `None`.
const fn shared(point: Option<u8>) -> Shared {
    let (length, byte) = match point {
        Some(byte) => (1, byte),
        None => (0, 0),
    };
    Shared {
        object: StrObject {
            header: ObjectHeader {
                reference_count: crate::header::IMMORTAL,
                type_id: STR_TYPE_ID,
            },
            length,
            kind: StrKind::Latin1,
            padding: 0,
            hash: AtomicI64::new(NOT_HASHED),
        },
        data: [byte, 0, 0, 0, 0, 0, 0, 0],
    }
}

/// The `str` of each code point up to U+00FF, which every string of one such code point is,
/// as CPython shares them: text of one character is common, and needs no allocation.
static CHARACTERS: [Shared; 256] = {
    let mut all = [const { shared(None) }; 256];
    let mut point: u8 = 0;
    loop {
        all[point as usize] = shared(Some(point));
        if point == u8::MAX {
            break all;
        }
        point += 1;
    }
};

/// The empty `str`, which every empty string a method makes is.
static EMPTY: Shared = shared(None);

/// Returns the shared `str` of `point`, for a code point up to U+00FF.
pub(crate) fn character(point: u32) -> Option<*mut StrObject> {
    let index = usize::try_from(point).ok().filter(|index| *index < 256)?;
    Some(core::ptr::from_ref(&CHARACTERS[index].object).cast_mut())
}

/// Returns the shared empty `str`.
pub(crate) fn empty() -> *mut StrObject {
    core::ptr::from_ref(&EMPTY.object).cast_mut()
}

/// Returns a new string of `length` code points of `kind`, with its data not yet written and
/// a zero unit after it. A string never changes its length, so the zero stays there, and a
/// `bytes` gives C its own buffer as a string ended by a zero byte (FFI-03, ADR-0047).
pub(crate) fn new_string(length: usize, kind: StrKind) -> *mut StrObject {
    let size = array_size(length.saturating_add(1), kind as usize, DATA_OFFSET);
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
            hash: AtomicI64::new(NOT_HASHED),
        });
        core::ptr::write_bytes(
            object
                .cast::<u8>()
                .add(DATA_OFFSET + length * kind as usize),
            0,
            kind as usize,
        );
    }
    object
}

/// Returns the address of a `bytes` object's data, which a zero byte follows, for a
/// `c_char_p` or `c_void_p` argument, or null for null, `None` of an optional `bytes`
/// (FFI-03, ADR-0047).
///
/// # Safety
///
/// `bytes` is null or points to a live `bytes` object.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn typy_rt_bytes_data(bytes: *const StrObject) -> *const u8 {
    if bytes.is_null() {
        return core::ptr::null();
    }
    // SAFETY: the data follows the fixed part of the object.
    unsafe { bytes.cast::<u8>().add(DATA_OFFSET) }
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

/// Returns the bytes of a string of ASCII characters only, or `None` for any other string,
/// which a method then reads code point by code point (NFR-21).
///
/// # Safety
///
/// `string` points to a live `str` object, which outlives the slice.
pub(crate) unsafe fn ascii_bytes<'a>(string: *const StrObject) -> Option<&'a [u8]> {
    // SAFETY: the caller passes a live string; a `Latin1` string's data is one byte per code
    // point, `length` of them.
    unsafe {
        if (*string).kind != StrKind::Latin1 {
            return None;
        }
        let bytes =
            core::slice::from_raw_parts(string.cast::<u8>().add(DATA_OFFSET), length(string));
        bytes.is_ascii().then_some(bytes)
    }
}

/// Returns a new string of `length` ASCII characters, which `fill` writes.
pub(crate) fn new_ascii(length: usize, fill: impl FnOnce(&mut [u8])) -> *mut StrObject {
    let string = new_string(length, StrKind::Latin1);
    // SAFETY: the new string has room for `length` one-byte code points, which nothing else
    // refers to yet.
    fill(unsafe { core::slice::from_raw_parts_mut(string.cast::<u8>().add(DATA_OFFSET), length) });
    string
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

/// Returns a new string of the code points of `parts`, one after another.
pub(crate) fn from_parts(parts: &[&str]) -> *mut StrObject {
    let points = || parts.iter().flat_map(|part| part.chars());
    let kind = points()
        .map(|point| StrKind::of(u32::from(point)))
        .max()
        .unwrap_or(StrKind::Latin1);
    let string = new_string(points().count(), kind);
    for (index, point) in points().enumerate() {
        // SAFETY: the string is new, the index is in bounds, and the kind holds every code point.
        unsafe { set_code_point(string, index, u32::from(point)) };
    }
    string
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
    fn the_fixed_part_takes_thirty_two_bytes() {
        assert_eq!(super::DATA_OFFSET, 32);
    }
}
