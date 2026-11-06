//! Locations in source text: files, byte offsets, spans and positions.

/// Index of a source file within one project.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct FileId(pub u32);

/// Byte offset into a source file; LEX-09 keeps every offset below 2^24.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TextOffset(pub u32);

/// A range of bytes in one source file, from `start` up to `end`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Span {
    file: FileId,
    start: TextOffset,
    end: TextOffset,
}

impl Span {
    /// Returns the span, or `None` when `end` is before `start`.
    #[must_use]
    pub const fn new(file: FileId, start: TextOffset, end: TextOffset) -> Option<Self> {
        if end.0 < start.0 {
            return None;
        }
        Some(Self { file, start, end })
    }

    /// Returns the file the span is in.
    #[must_use]
    pub const fn file(&self) -> FileId {
        self.file
    }

    /// Returns the offset of the first byte.
    #[must_use]
    pub const fn start(&self) -> TextOffset {
        self.start
    }

    /// Returns the offset just past the last byte.
    #[must_use]
    pub const fn end(&self) -> TextOffset {
        self.end
    }

    /// Returns the number of bytes the span covers.
    #[must_use]
    pub const fn len(&self) -> u32 {
        self.end.0 - self.start.0
    }

    /// Returns true when the span covers no bytes.
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

/// A line and a column, both counted from 1, the column in code points (DIA-01).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Position {
    /// The line, from 1.
    pub line: u32,
    /// The column, from 1, counted in code points.
    pub column: u32,
}

#[cfg(test)]
mod tests {
    use super::{FileId, Span, TextOffset};

    #[test]
    fn span_whose_end_is_before_its_start_is_refused() {
        assert_eq!(Span::new(FileId(0), TextOffset(5), TextOffset(4)), None);
    }

    #[test]
    fn empty_span_is_allowed_and_reports_zero_length() {
        let span = Span::new(FileId(0), TextOffset(3), TextOffset(3)).unwrap();
        assert!(span.is_empty());
        assert_eq!(span.len(), 0);
    }

    #[test]
    fn span_reports_its_parts_and_length() {
        let span = Span::new(FileId(2), TextOffset(10), TextOffset(14)).unwrap();
        assert_eq!(span.file(), FileId(2));
        assert_eq!(span.start(), TextOffset(10));
        assert_eq!(span.end(), TextOffset(14));
        assert_eq!(span.len(), 4);
        assert!(!span.is_empty());
    }
}
