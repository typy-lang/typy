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

    /// Reads `root/path`; on failure the file is returned empty, with the diagnostic for it.
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
                let at = TextOffset(offset(error.utf8_error().valid_up_to()));
                let span = Span::empty(id, at);
                let diagnostic = Diagnostic::new(
                    DiagnosticCode::error(102),
                    span,
                    format!("`{path}` is not valid UTF-8 at byte {}", at.0),
                )
                .with_rule(RuleId::LEX_01);
                (empty, Some(diagnostic))
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
mod tests {
    use super::{MAX_SOURCE_BYTES, SourceFile};
    use camino::Utf8Path;

    use typy_diagnostics::{FileId, TextOffset};

    #[test]
    fn byte_order_mark_is_removed() {
        let file = SourceFile::from_text(FileId(0), "a.py".into(), "\u{feff}x = 1\n");
        assert_eq!(file.text(), "x = 1\n");
    }

    fn read(contents: &[u8]) -> (SourceFile, Option<typy_diagnostics::Diagnostic>) {
        let directory = tempfile::tempdir().unwrap();
        std::fs::write(directory.path().join("app.py"), contents).unwrap();
        let root = Utf8Path::from_path(directory.path()).unwrap();
        SourceFile::read(FileId(3), root, Utf8Path::new("app.py"))
    }

    #[test]
    fn readable_file_is_read_with_its_id_and_relative_path() {
        let (file, diagnostic) = read(b"print(1)\n");
        assert!(diagnostic.is_none());
        assert_eq!(file.id(), FileId(3));
        assert_eq!(file.path(), "app.py");
        assert_eq!(file.text(), "print(1)\n");
        assert_eq!(file.lines().line_count(), 2);
    }

    #[test]
    fn empty_file_is_read() {
        let (file, diagnostic) = read(b"");
        assert!(diagnostic.is_none());
        assert_eq!(file.text(), "");
    }

    #[test]
    fn missing_file_gives_e0105_and_an_empty_file() {
        let directory = tempfile::tempdir().unwrap();
        let root = Utf8Path::from_path(directory.path()).unwrap();
        let (file, diagnostic) = SourceFile::read(FileId(0), root, Utf8Path::new("absent.py"));
        let diagnostic = diagnostic.unwrap();
        assert_eq!(diagnostic.code().to_string(), "E0105");
        assert!(
            diagnostic
                .message()
                .starts_with("`absent.py` cannot be read: ")
        );
        assert_eq!(file.text(), "");
    }

    #[test]
    fn invalid_utf8_gives_e0102_at_the_first_invalid_byte() {
        let (_, diagnostic) = read(b"x = 1\n\xff\n");
        let diagnostic = diagnostic.unwrap();
        assert_eq!(diagnostic.code().to_string(), "E0102");
        assert_eq!(diagnostic.span().unwrap().start(), TextOffset(6));
        assert_eq!(
            diagnostic.rule().map(typy_diagnostics::RuleId::as_str),
            Some("LEX-01")
        );
    }

    #[test]
    fn file_of_exactly_16_mib_is_read() {
        let contents = vec![b'#'; usize::try_from(MAX_SOURCE_BYTES).unwrap()];
        let (file, diagnostic) = read(&contents);
        assert!(diagnostic.is_none());
        assert_eq!(file.text().len() as u64, MAX_SOURCE_BYTES);
    }

    #[test]
    fn file_one_byte_over_16_mib_gives_e0101() {
        let contents = vec![b'#'; usize::try_from(MAX_SOURCE_BYTES).unwrap() + 1];
        let (file, diagnostic) = read(&contents);
        let diagnostic = diagnostic.unwrap();
        assert_eq!(diagnostic.code().to_string(), "E0101");
        assert_eq!(
            diagnostic.rule().map(typy_diagnostics::RuleId::as_str),
            Some("LEX-09")
        );
        assert_eq!(file.text(), "");
    }
}
