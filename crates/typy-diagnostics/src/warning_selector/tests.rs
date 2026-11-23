//! Tests of `warning_selector`.

use super::{SelectorError, WarningSelector};
use crate::code::DiagnosticCode;

#[test]
fn warnings_names_every_warning_and_no_error() {
    let all = WarningSelector::parse("warnings").unwrap();
    assert_eq!(all, WarningSelector::All);
    assert!(all.names(DiagnosticCode::warning(101)));
    assert!(all.names(DiagnosticCode::warning(401)));
    assert!(!all.names(DiagnosticCode::error(101)));
}

#[test]
fn a_reported_warning_code_names_only_itself() {
    let one = WarningSelector::parse("W0301").unwrap();
    assert_eq!(one, WarningSelector::Code(DiagnosticCode::warning(301)));
    assert!(one.names(DiagnosticCode::warning(301)));
    assert!(!one.names(DiagnosticCode::warning(101)));
}

#[test]
fn a_text_that_names_no_reported_warning_is_refused() {
    for text in [
        "", "warning", "Warnings", "W101", "w0101", "E0101", "W0999", "W01010",
    ] {
        assert_eq!(
            WarningSelector::parse(text),
            Err(SelectorError {
                text: text.to_owned()
            }),
            "{text}"
        );
    }
}

#[test]
fn the_refusal_lists_the_warnings_that_can_be_named() {
    let error = WarningSelector::parse("E0101").unwrap_err();
    assert_eq!(
        error.to_string(),
        "`E0101` names no warning; give `warnings` or one of W0101, W0301, W0401, W0402"
    );
}
