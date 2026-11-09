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
mod tests {
    use super::{Points, is_space};
    use crate::string::from_str;

    #[test]
    fn points_are_read_by_index() {
        // SAFETY: the string is live for the test.
        let points = unsafe { Points::of(from_str("a\u{e9}")) };
        assert_eq!(
            (points.len(), points.get(0), points.get(1)),
            (2, 0x61, 0xe9)
        );
    }

    #[test]
    fn whitespace_matches_str_isspace() {
        for point in [
            0x20, 0x09, 0x0a, 0x0b, 0x0c, 0x0d, 0x1c, 0x1f, 0x85, 0xa0, 0x3000,
        ] {
            assert!(is_space(point), "{point:#x}");
        }
        for point in [0x41, 0x5f, 0x200b, 0xfeff] {
            assert!(!is_space(point), "{point:#x}");
        }
    }
}
