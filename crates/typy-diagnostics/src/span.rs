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

    /// Returns the empty span at `at`, which is always valid.
    #[must_use]
    pub const fn empty(file: FileId, at: TextOffset) -> Self {
        Self {
            file,
            start: at,
            end: at,
        }
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
mod tests;
