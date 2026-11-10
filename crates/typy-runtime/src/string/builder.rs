//! Builds a new string of a length not known in advance, one code point or piece at a time.

use super::{StrKind, StrObject, code_point, length, new_string, set_code_point};
use crate::allocator::{allocate, array_size, free, reallocate};

/// A growing buffer of code points that becomes a `str` as narrow as its widest code point.
pub(crate) struct StrBuilder {
    points: *mut u32,
    length: usize,
    capacity: usize,
    kind: StrKind,
}

impl StrBuilder {
    /// Returns an empty builder with room for `capacity` code points.
    #[allow(
        clippy::cast_ptr_alignment,
        reason = "malloc returns memory aligned for every type"
    )]
    pub(crate) fn with_capacity(capacity: usize) -> Self {
        let capacity = capacity.max(8);
        Self {
            points: allocate(array_size(capacity, 4, 0)).cast::<u32>(),
            length: 0,
            capacity,
            kind: StrKind::Latin1,
        }
    }

    /// Returns the code points appended so far.
    pub(crate) fn points(&self) -> &[u32] {
        // SAFETY: the buffer holds `length` code points that were written.
        unsafe { core::slice::from_raw_parts(self.points, self.length) }
    }

    /// Appends one code point.
    #[allow(
        clippy::cast_ptr_alignment,
        reason = "malloc returns memory aligned for every type"
    )]
    pub(crate) fn push(&mut self, point: u32) {
        if self.length == self.capacity {
            self.capacity = self.capacity * 2 + 8;
            self.points =
                reallocate(self.points.cast(), array_size(self.capacity, 4, 0)).cast::<u32>();
        }
        // SAFETY: the buffer has room for `capacity` code points, and `length` is below it.
        unsafe { *self.points.add(self.length) = point };
        self.length += 1;
        self.kind = self.kind.max(StrKind::of(point));
    }

    /// Appends the code points `start..end` of `text`.
    ///
    /// # Safety
    ///
    /// `text` points to a live string, and `start..end` lies inside it.
    pub(crate) unsafe fn push_range(&mut self, text: *const StrObject, start: usize, end: usize) {
        for index in start..end {
            // SAFETY: the caller keeps the range inside the string.
            self.push(unsafe { code_point(text, index) });
        }
    }

    /// Appends every code point of `text`.
    ///
    /// # Safety
    ///
    /// `text` points to a live string.
    pub(crate) unsafe fn push_str(&mut self, text: *const StrObject) {
        // SAFETY: the caller passes a live string.
        unsafe { self.push_range(text, 0, length(text)) };
    }

    /// Frees the buffer without making a string, when the result is not wanted.
    pub(crate) fn discard(self) {
        free(self.points.cast());
    }

    /// Returns the new string, of the type of `source`, and frees the buffer.
    ///
    /// # Safety
    ///
    /// `source` points to a live string.
    pub(crate) unsafe fn finish_like(self, source: *const StrObject) -> *mut StrObject {
        let text = self.finish();
        // SAFETY: the caller passes a live string; the new one is not shared yet.
        unsafe { (*text).header.type_id = (*source).header.type_id };
        text
    }

    /// Returns the new string, and frees the buffer.
    pub(crate) fn finish(self) -> *mut StrObject {
        let text = new_string(self.length, self.kind);
        for index in 0..self.length {
            // SAFETY: the string has room for `length` code points of this kind, and the buffer
            // holds that many.
            unsafe { set_code_point(text, index, *self.points.add(index)) };
        }
        free(self.points.cast());
        text
    }
}

/// Returns a new string of the code points `start..end` of `text`, of the type of `text`: a
/// part of a `bytes` is a `bytes`.
///
/// # Safety
///
/// `text` points to a live string, and `start..end` lies inside it.
pub(crate) unsafe fn substring(text: *const StrObject, start: usize, end: usize) -> *mut StrObject {
    let mut builder = StrBuilder::with_capacity(end - start);
    // SAFETY: the caller's guarantee.
    unsafe {
        builder.push_range(text, start, end);
        builder.finish_like(text)
    }
}

/// Returns whether `text` is a `bytes`, whose methods treat only ASCII as whitespace, letters
/// and line breaks.
///
/// # Safety
///
/// `text` points to a live string.
pub(crate) unsafe fn is_bytes(text: *const StrObject) -> bool {
    // SAFETY: the caller's guarantee.
    unsafe { (*text).header.type_id == crate::header::BYTES_TYPE_ID }
}

/// Returns whether a code point is whitespace for `text`: `str.isspace` for a `str`, and for a
/// `bytes` only space, `\t`, `\n`, `\v`, `\f` and `\r`.
///
/// # Safety
///
/// `text` points to a live string.
pub(crate) unsafe fn space_for(text: *const StrObject) -> fn(u32) -> bool {
    // SAFETY: the caller's guarantee.
    if unsafe { is_bytes(text) } {
        |point| matches!(point, 0x20 | 0x09..=0x0d)
    } else {
        super::is_space
    }
}
