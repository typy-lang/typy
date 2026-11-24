//! Reads the code points of a string by index, and recognises Python's whitespace.

use super::{StrObject, code_point, length};

/// The code points of a string, read on demand.
pub(crate) struct Points {
    text: *const StrObject,
    length: usize,
}

impl Points {
    /// Returns a reader of `text`.
    ///
    /// # Safety
    ///
    /// `text` points to a live `str` object for as long as the value is used.
    pub(crate) unsafe fn of(text: *const StrObject) -> Self {
        Self {
            text,
            // SAFETY: the caller passes a live string.
            length: unsafe { length(text) },
        }
    }

    /// Returns the number of code points.
    pub(crate) const fn len(&self) -> usize {
        self.length
    }

    /// Returns the code point at `index`, which is below `len()`.
    pub(crate) fn get(&self, index: usize) -> u32 {
        // SAFETY: the string is live, and callers keep `index` below its length.
        unsafe { code_point(self.text, index) }
    }
}

/// Returns true for the whitespace `int()`, `float()`, `str.split` and `str.strip` skip:
/// `str.isspace`, from CPython's Unicode data, which adds the information separators U+001C to
/// U+001F to Unicode's white space.
pub(crate) fn is_space(point: u32) -> bool {
    super::unicode::has(super::unicode::Property::Space, point)
}

#[cfg(test)]
mod tests;
