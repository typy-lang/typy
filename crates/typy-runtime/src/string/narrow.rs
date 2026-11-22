//! Byte paths for strings of one byte a code point, `Latin1` strings and `bytes`, which most
//! text is (NFR-21): such a string's data is its code points, so the methods search, compare
//! and copy whole runs of bytes where the general path reads and writes one code point at a
//! time. Each path gives what the general path gives, which the tests compare.

use super::{DATA_OFFSET, StrKind, StrObject, length, new_string};

/// Returns the code points of a string of one byte each, or `None` for a wider string.
///
/// # Safety
///
/// `text` points to a live string, which outlives the slice and does not change.
pub(crate) unsafe fn latin1<'a>(text: *const StrObject) -> Option<&'a [u8]> {
    // SAFETY: the caller passes a live string; a `Latin1` string holds `length` bytes after
    // its fixed part.
    unsafe {
        ((*text).kind == StrKind::Latin1)
            .then(|| core::slice::from_raw_parts(text.cast::<u8>().add(DATA_OFFSET), length(text)))
    }
}

/// Returns the data of a string as it is stored, its code points one, two or four bytes each,
/// little-endian as every target of TyPy is.
///
/// # Safety
///
/// `text` points to a live string, which outlives the slice and does not change.
pub(crate) unsafe fn stored<'a>(text: *const StrObject) -> &'a [u8] {
    // SAFETY: the caller passes a live string, which holds `length` code points of its kind
    // after its fixed part.
    unsafe {
        core::slice::from_raw_parts(
            text.cast::<u8>().add(DATA_OFFSET),
            length(text) * (*text).kind as usize,
        )
    }
}

// The stored data of a code point is its little-endian bytes, which hashing reads.
const _: () = assert!(cfg!(target_endian = "little"));

/// Returns whether two strings, or two `bytes`, hold the same code points: a string is held at
/// the narrowest kind that holds its code points, so two equal ones store the same data.
///
/// # Safety
///
/// Both point to live strings.
pub(crate) unsafe fn same_text(left: *const StrObject, right: *const StrObject) -> bool {
    // SAFETY: the caller passes live strings.
    unsafe {
        core::ptr::eq(left, right)
            || (length(left) == length(right)
                && (*left).kind == (*right).kind
                && stored(left) == stored(right))
    }
}

/// Returns a new string of the type of `source`, a `str` or a `bytes`, of `length` code points
/// of one byte each, which `fill` writes.
///
/// # Safety
///
/// `source` points to a live string.
pub(crate) unsafe fn new_like(
    source: *const StrObject,
    length: usize,
    fill: impl FnOnce(&mut [u8]),
) -> *mut StrObject {
    let string = new_string(length, StrKind::Latin1);
    // SAFETY: the new string has room for `length` bytes, which nothing else refers to yet;
    // the caller passes a live source.
    unsafe {
        fill(core::slice::from_raw_parts_mut(
            string.cast::<u8>().add(DATA_OFFSET),
            length,
        ));
        (*string).header.type_id = (*source).header.type_id;
    }
    string
}

/// Returns the first position of `needle` in `haystack`; an empty needle is at 0.
pub(crate) fn find(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    let Some((first, rest)) = needle.split_first() else {
        return Some(0);
    };
    let last = haystack.len().checked_sub(needle.len())?;
    let mut from = 0;
    // Each candidate starts with the needle's first byte, which a scan of bytes finds.
    while let Some(offset) = haystack[from..=last].iter().position(|byte| byte == first) {
        let at = from + offset;
        if haystack[at + 1..at + needle.len()] == *rest {
            return Some(at);
        }
        from = at + 1;
    }
    None
}

/// Returns the last position of `needle` in `haystack`; an empty needle is at the end.
pub(crate) fn rfind(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    let last = haystack.len().checked_sub(needle.len())?;
    (0..=last)
        .rev()
        .find(|at| haystack[*at..at + needle.len()] == *needle)
}

/// Returns how many times `needle`, which is not empty, occurs in `haystack` without
/// overlapping, at most `limit` of them.
pub(crate) fn count(haystack: &[u8], needle: &[u8], limit: u64) -> u64 {
    if let [only] = needle {
        #[allow(
            clippy::naive_bytecount,
            reason = "the runtime takes no dependency for counting bytes, which this loop does"
        )]
        let all = haystack.iter().filter(|byte| *byte == only).count();
        return u64::try_from(all).unwrap_or(u64::MAX).min(limit);
    }
    let (mut at, mut found) = (0, 0);
    while found < limit
        && let Some(position) = find(&haystack[at..], needle)
    {
        found += 1;
        at += position + needle.len();
    }
    found
}

#[cfg(test)]
mod tests;
