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
mod tests {
    use super::LineIndex;
    use proptest::prelude::*;
    use typy_diagnostics::{Position, TextOffset};

    fn position(line: u32, column: u32) -> Position {
        Position { line, column }
    }

    #[test]
    fn lf_crlf_and_lone_cr_each_end_a_line() {
        let text = "a\nbb\r\nccc\rd";
        let lines = LineIndex::new(text);
        assert_eq!(lines.line_count(), 4);
        assert_eq!(lines.line_text(text, 1), "a");
        assert_eq!(lines.line_text(text, 2), "bb");
        assert_eq!(lines.line_text(text, 3), "ccc");
        assert_eq!(lines.line_text(text, 4), "d");
        assert_eq!(lines.position(text, TextOffset(7)), position(3, 2));
        assert_eq!(lines.position(text, TextOffset(11)), position(4, 2));
    }

    #[test]
    fn lines_outside_the_file_are_empty() {
        let text = "x = 1\n";
        let lines = LineIndex::new(text);
        assert_eq!(lines.line_text(text, 0), "");
        assert_eq!(lines.line_text(text, 2), "");
        assert_eq!(lines.line_text(text, 3), "");
    }

    #[test]
    fn empty_text_has_one_line() {
        let lines = LineIndex::new("");
        assert_eq!(lines.line_count(), 1);
        assert_eq!(lines.position("", TextOffset(0)), position(1, 1));
    }

    #[test]
    fn columns_count_code_points() {
        let text = "s = \"\u{e9}\u{1f600}\" + x";
        let at = TextOffset(u32::try_from(text.find('+').unwrap()).unwrap());
        assert_eq!(LineIndex::new(text).position(text, at), position(1, 10));
    }

    #[test]
    fn offset_past_the_end_is_clamped_to_the_end() {
        let text = "ab";
        assert_eq!(
            LineIndex::new(text).position(text, TextOffset(99)),
            position(1, 3)
        );
    }

    proptest! {
        #[test]
        fn position_agrees_with_counting_code_points(text in "[a\u{e9}\u{1f600}\n\r]{0,40}", cut in 0_usize..60) {
            let mut at = cut.min(text.len());
            while !text.is_char_boundary(at) {
                at -= 1;
            }
            // An offset between CR and LF is inside one line ending; step back to its start.
            if at > 0 && text.as_bytes().get(at) == Some(&b'\n') && text.as_bytes()[at - 1] == b'\r' {
                at -= 1;
            }
            let lines = LineIndex::new(&text);
            let found = lines.position(&text, TextOffset(u32::try_from(at).unwrap()));
            let before = &text[..at];
            let normalised = before.replace("\r\n", "\n").replace('\r', "\n");
            let expected_line = normalised.matches('\n').count() + 1;
            let expected_column = normalised.rsplit('\n').next().unwrap().chars().count() + 1;
            prop_assert_eq!(found.line as usize, expected_line);
            prop_assert_eq!(found.column as usize, expected_column);
        }
    }
}
