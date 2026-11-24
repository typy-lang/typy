//! Tests of `source_file`.

use super::{MAX_SOURCE_BYTES, SourceFile};
use camino::Utf8Path;

use typy_diagnostics::{FileId, TextOffset};

#[test]
fn a_main_file_that_is_a_directory_is_e0105() {
    let directory = tempfile::tempdir().unwrap();
    let root = camino::Utf8PathBuf::from_path_buf(directory.path().to_owned()).unwrap();
    std::fs::create_dir(root.join("app.py")).unwrap();
    let (file, diagnostic) = SourceFile::read(FileId(0), &root, Utf8Path::new("app.py"));
    assert_eq!(diagnostic.unwrap().code().to_string(), "E0105");
    assert_eq!(file.text(), "");
}

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
    let (file, diagnostic) = read(b"x = 1\ny = \"\xff\"\n");
    let diagnostic = diagnostic.unwrap();
    assert_eq!(diagnostic.code().to_string(), "E0102");
    let at = diagnostic.span().unwrap().start();
    assert_eq!(at, TextOffset(11));
    // The valid text before the byte is kept, so the diagnostic shows its line and column.
    assert_eq!(file.text(), "x = 1\ny = \"");
    let position = file.position(at);
    assert_eq!((position.line, position.column), (2, 6));
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
