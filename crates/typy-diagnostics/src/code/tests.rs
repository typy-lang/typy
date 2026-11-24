//! Tests of `code`.

use super::{CodeError, DiagnosticCode, Severity};
use proptest::prelude::*;

#[test]
fn codes_are_written_with_their_letter_and_four_digits() {
    assert_eq!(DiagnosticCode::error(401).to_string(), "E0401");
    assert_eq!(DiagnosticCode::warning(7).to_string(), "W0007");
    assert_eq!(DiagnosticCode::error(9999).to_string(), "E9999");
}

#[test]
fn well_formed_codes_parse_with_their_severity() {
    let code = DiagnosticCode::parse("W0501").unwrap();
    assert_eq!(code.severity(), Severity::Warning);
    assert_eq!(code.number(), 501);
    assert_eq!(
        DiagnosticCode::parse("E0000").unwrap(),
        DiagnosticCode::error(0)
    );
}

#[test]
fn malformed_codes_are_refused_with_the_text() {
    for text in [
        "",
        "E",
        "E401",
        "E04011",
        "e0401",
        "X0401",
        "E04a1",
        "E 401",
        "E-401",
        "\u{e9}0401",
    ] {
        assert_eq!(
            DiagnosticCode::parse(text),
            Err(CodeError::Malformed {
                text: text.to_owned()
            }),
            "{text:?}"
        );
    }
}

#[test]
fn code_error_message_names_the_text() {
    let error = DiagnosticCode::parse("Q1").unwrap_err();
    assert!(error.to_string().contains("`Q1`"));
}

#[test]
fn severity_labels_are_lower_case_words() {
    assert_eq!(Severity::Error.label(), "error");
    assert_eq!(Severity::Warning.label(), "warning");
}

proptest! {
    #[test]
    fn display_and_parse_round_trip(number in 0_u16..=9999, warning in any::<bool>()) {
        let code = if warning {
            DiagnosticCode::warning(number)
        } else {
            DiagnosticCode::error(number)
        };
        prop_assert_eq!(DiagnosticCode::parse(&code.to_string()), Ok(code));
    }

    #[test]
    fn parse_never_panics(text in "\\PC{0,8}") {
        let _ = DiagnosticCode::parse(&text);
    }
}
