//! Tests of `span`.

use super::{FileId, Span, TextOffset};

#[test]
fn span_whose_end_is_before_its_start_is_refused() {
    assert_eq!(Span::new(FileId(0), TextOffset(5), TextOffset(4)), None);
}

#[test]
fn empty_constructor_gives_a_zero_length_span() {
    let span = Span::empty(FileId(1), TextOffset(7));
    assert!(span.is_empty());
    assert_eq!(span.start(), TextOffset(7));
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
