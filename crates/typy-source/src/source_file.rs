//! Reading one source file, with the checks of LEX-01 and LEX-09.

use std::io::Read as _;
use std::sync::Arc;

use camino::{Utf8Path, Utf8PathBuf};
use typy_diagnostics::{Diagnostic, DiagnosticCode, FileId, Position, RuleId, Span, TextOffset};

use crate::line_index::{LineIndex, offset};

/// The largest source file TyPy reads: 16 MiB (LEX-09).
pub const MAX_SOURCE_BYTES: u64 = 16 * 1024 * 1024;

const BYTE_ORDER_MARK: &str = "\u{feff}";

/// One `.py` file: its path relative to the project root and its UTF-8 text.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SourceFile {
    id: FileId,
    path: Utf8PathBuf,
    text: Arc<str>,
    lines: LineIndex,
}

impl SourceFile {
    /// Returns a file made from text already in memory.
    #[must_use]
    pub fn from_text(id: FileId, path: Utf8PathBuf, text: &str) -> Self {
        let text = text.strip_prefix(BYTE_ORDER_MARK).unwrap_or(text);
        Self {
            id,
            path,
            lines: LineIndex::new(text),
            text: Arc::from(text),
        }
    }

    /// Reads `root/path`; on failure the file is returned empty, or holding the valid text before
    /// the first byte that is not UTF-8, with the diagnostic for it.
    #[must_use]
    pub fn read(id: FileId, root: &Utf8Path, path: &Utf8Path) -> (Self, Option<Diagnostic>) {
        let empty = Self::from_text(id, path.to_owned(), "");
        let start = Span::empty(id, TextOffset(0));
        let full_path = root.join(path);
        let unreadable = |reason: std::io::Error| {
            Diagnostic::new(
                DiagnosticCode::error(105),
                start,
                format!("`{path}` cannot be read: {reason}"),
            )
        };
        let file = match std::fs::File::open(&full_path) {
            Ok(file) => file,
            Err(reason) => return (empty, Some(unreadable(reason))),
        };
        // Reading one byte past the limit detects a file that is too large without trusting
        // its metadata, which can change between the check and the read.
        let mut bytes = Vec::new();
        if let Err(reason) = file.take(MAX_SOURCE_BYTES + 1).read_to_end(&mut bytes) {
            return (empty, Some(unreadable(reason)));
        }
        if bytes.len() as u64 > MAX_SOURCE_BYTES {
            let diagnostic = Diagnostic::new(
                DiagnosticCode::error(101),
                start,
                format!("`{path}` is larger than 16 MiB"),
            )
            .with_rule(RuleId::LEX_09);
            return (empty, Some(diagnostic));
        }
        match String::from_utf8(bytes) {
            Ok(text) => (Self::from_text(id, path.to_owned(), &text), None),
            Err(error) => {
                let valid = error.utf8_error().valid_up_to();
                let at = TextOffset(offset(valid));
                let span = Span::empty(id, at);
                let diagnostic = Diagnostic::new(
                    DiagnosticCode::error(102),
                    span,
                    format!("`{path}` is not valid UTF-8 at byte {}", at.0),
                )
                .with_rule(RuleId::LEX_01);
                // The bytes before the first invalid one are valid UTF-8; keeping them lets the
                // diagnostic show its line and column.
                let bytes = error.into_bytes();
                let prefix = String::from_utf8_lossy(&bytes[..valid]);
                (
                    Self::from_text(id, path.to_owned(), &prefix),
                    Some(diagnostic),
                )
            }
        }
    }

    /// Returns the file's ID.
    #[must_use]
    pub fn id(&self) -> FileId {
        self.id
    }

    /// Returns the path relative to the project root.
    #[must_use]
    pub fn path(&self) -> &Utf8Path {
        &self.path
    }

    /// Returns the text, without a byte order mark.
    #[must_use]
    pub fn text(&self) -> &str {
        &self.text
    }

    /// Returns the line index.
    #[must_use]
    pub fn lines(&self) -> &LineIndex {
        &self.lines
    }

    /// Returns the position of a byte offset.
    #[must_use]
    pub fn position(&self, at: TextOffset) -> Position {
        self.lines.position(&self.text, at)
    }
}

#[cfg(test)]
mod tests;
