//! Finds lines and columns in source text.

use typy_diagnostics::{Position, TextOffset};

/// The byte offset where each line starts, so positions are found by binary search.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LineIndex {
    starts: Vec<u32>,
}

impl LineIndex {
    /// Builds the index; LF, CRLF and a lone CR each end a line (LR 2.1.2).
    #[must_use]
    pub fn new(text: &str) -> Self {
        let bytes = text.as_bytes();
        let mut starts = vec![0];
        let mut index = 0;
        while index < bytes.len() {
            match bytes[index] {
                b'\n' => starts.push(offset(index + 1)),
                b'\r' if bytes.get(index + 1) != Some(&b'\n') => starts.push(offset(index + 1)),
                _ => {}
            }
            index += 1;
        }
        Self { starts }
    }

    /// Returns the number of lines, counting a final line without an ending.
    #[must_use]
    pub fn line_count(&self) -> u32 {
        offset(self.starts.len())
    }

    /// Returns the line and column of `at`, with the column in code points (DIA-01).
    #[must_use]
    pub fn position(&self, text: &str, at: TextOffset) -> Position {
        let at = at.0.min(offset(text.len()));
        let line = self.starts.partition_point(|start| *start <= at);
        let start = self.starts[line - 1] as usize;
        let column = text[start..at as usize].chars().count() + 1;
        Position {
            line: offset(line),
            column: offset(column),
        }
    }

    /// Returns the text of `line`, from 1, without its line ending; an absent line is empty.
    #[must_use]
    pub fn line_text<'text>(&self, text: &'text str, line: u32) -> &'text str {
        let Some(&start) = self.starts.get((line as usize).wrapping_sub(1)) else {
            return "";
        };
        let end = self
            .starts
            .get(line as usize)
            .map_or(text.len(), |next| *next as usize);
        text[start as usize..end].trim_end_matches(['\n', '\r'])
    }
}

/// Converts a length that LEX-09 keeps below 2^24 to a `u32`.
pub(crate) fn offset(value: usize) -> u32 {
    u32::try_from(value).unwrap_or(u32::MAX)
}

#[cfg(test)]
mod tests;
