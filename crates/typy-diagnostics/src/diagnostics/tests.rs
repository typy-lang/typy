//! Tests of `diagnostics`.

use super::Diagnostics;
use crate::code::DiagnosticCode;
use crate::diagnostic::Diagnostic;
use crate::render::SourceLookup;
use crate::span::{FileId, Position, Span, TextOffset};

struct Files;

#[allow(
    clippy::unnecessary_literal_bound,
    reason = "the trait ties the returned text to self"
)]
impl SourceLookup for Files {
    fn path(&self, file: FileId) -> &str {
        if file.0 == 0 { "b.py" } else { "a.py" }
    }
    fn position(&self, _file: FileId, offset: TextOffset) -> Position {
        Position {
            line: offset.0 / 10 + 1,
            column: offset.0 % 10 + 1,
        }
    }
    fn line_text(&self, _file: FileId, _line: u32) -> &str {
        ""
    }
}

fn at(file: u32, start: u32) -> Span {
    Span::new(FileId(file), TextOffset(start), TextOffset(start + 1)).unwrap()
}

#[test]
fn same_code_at_the_same_span_is_kept_once() {
    let mut diagnostics = Diagnostics::new();
    diagnostics.push(Diagnostic::new(
        DiagnosticCode::error(402),
        at(0, 3),
        "first",
    ));
    diagnostics.push(Diagnostic::new(
        DiagnosticCode::error(402),
        at(0, 3),
        "second",
    ));
    diagnostics.push(Diagnostic::new(
        DiagnosticCode::error(401),
        at(0, 3),
        "other code",
    ));
    assert_eq!(diagnostics.len(), 2);
}

#[test]
fn extend_adds_every_new_diagnostic_and_skips_duplicates() {
    let mut diagnostics = Diagnostics::new();
    diagnostics.push(Diagnostic::new(
        DiagnosticCode::error(402),
        at(0, 3),
        "kept",
    ));
    let mut other = Diagnostics::new();
    other.push(Diagnostic::new(
        DiagnosticCode::error(402),
        at(0, 3),
        "duplicate",
    ));
    other.push(Diagnostic::new(DiagnosticCode::error(401), at(0, 5), "new"));
    diagnostics.extend(other);
    let messages: Vec<&str> = diagnostics.iter().map(Diagnostic::message).collect();
    assert_eq!(messages, ["kept", "new"]);
}

#[test]
fn a_collection_is_empty_until_a_diagnostic_is_added() {
    let mut diagnostics = Diagnostics::new();
    assert!(diagnostics.is_empty());
    diagnostics.push(Diagnostic::new(DiagnosticCode::error(401), at(0, 0), "bad"));
    assert!(!diagnostics.is_empty());
    assert_eq!(diagnostics.len(), 1);
}

#[test]
fn warnings_alone_are_not_errors() {
    let mut diagnostics = Diagnostics::new();
    assert!(!diagnostics.has_errors());
    assert!(diagnostics.is_empty());
    diagnostics.push(Diagnostic::new(
        DiagnosticCode::warning(501),
        at(0, 0),
        "unused",
    ));
    assert!(!diagnostics.has_errors());
    diagnostics.push(Diagnostic::new(
        DiagnosticCode::error(401),
        at(0, 0),
        "bad operands",
    ));
    assert!(diagnostics.has_errors());
}

#[test]
fn sorted_order_is_path_then_line_then_column_then_code() {
    let mut diagnostics = Diagnostics::new();
    diagnostics.push(Diagnostic::new(
        DiagnosticCode::error(402),
        at(0, 21),
        "b line 3",
    ));
    diagnostics.push(Diagnostic::new(
        DiagnosticCode::error(402),
        at(0, 5),
        "b line 1",
    ));
    diagnostics.push(Diagnostic::new(
        DiagnosticCode::error(401),
        at(0, 5),
        "b line 1 lower code",
    ));
    diagnostics.push(Diagnostic::new(
        DiagnosticCode::error(999),
        at(1, 30),
        "a line 4",
    ));
    diagnostics.push(Diagnostic::without_location(
        DiagnosticCode::error(702),
        "no location",
    ));
    let messages: Vec<String> = diagnostics
        .into_sorted(&Files)
        .iter()
        .map(|item| item.message().to_owned())
        .collect();
    assert_eq!(
        messages,
        [
            "no location",
            "a line 4",
            "b line 1 lower code",
            "b line 1",
            "b line 3"
        ]
    );
}

#[test]
fn extend_keeps_one_copy_of_each_diagnostic() {
    let mut first = Diagnostics::from(Diagnostic::new(DiagnosticCode::error(401), at(0, 0), "x"));
    let second = Diagnostics::from(Diagnostic::new(DiagnosticCode::error(401), at(0, 0), "x"));
    first.extend(second);
    assert_eq!(first.len(), 1);
}
