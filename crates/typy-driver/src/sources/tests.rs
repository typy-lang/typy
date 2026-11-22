//! Tests of `sources`.

use super::Sources;
use typy_diagnostics::{FileId, Position, SourceLookup, TextOffset};

#[test]
fn no_sources_answer_with_empty_text() {
    let sources = Sources::None;
    assert_eq!(sources.path(FileId(0)), "");
    assert_eq!(sources.line_text(FileId(0), 1), "");
    assert_eq!(
        sources.position(FileId(0), TextOffset(9)),
        Position { line: 1, column: 1 }
    );
}
