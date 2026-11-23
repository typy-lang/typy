//! Tests of `diagnostic`.

use super::Diagnostic;
use crate::code::DiagnosticCode;
use crate::span::{FileId, Span, TextOffset};

fn at(file: u32, start: u32) -> Span {
    Span::new(FileId(file), TextOffset(start), TextOffset(start + 1)).unwrap()
}

#[test]
fn builder_methods_set_every_detail() {
    let diagnostic = Diagnostic::new(DiagnosticCode::error(402), at(0, 0), "message")
        .with_label(at(0, 4), "label")
        .with_help("hint")
        .with_fix(at(0, 4), "new")
        .with_rule(crate::rule::RuleId::TYP_03);
    assert_eq!(diagnostic.labels().len(), 1);
    assert_eq!(diagnostic.help(), Some("hint"));
    assert_eq!(
        diagnostic.fix().map(|fix| fix.replacement.as_str()),
        Some("new")
    );
    assert_eq!(diagnostic.rule(), Some(crate::rule::RuleId::TYP_03));
    assert_eq!(diagnostic.span(), Some(at(0, 0)));
}

#[test]
fn a_diagnostic_may_have_no_location() {
    let diagnostic = Diagnostic::without_location(DiagnosticCode::error(702), "link failed");
    assert_eq!(
        (diagnostic.span(), diagnostic.message()),
        (None, "link failed")
    );
}

#[test]
fn a_denied_warning_is_an_error_that_keeps_its_code() {
    let warning = Diagnostic::without_location(DiagnosticCode::warning(101), "an escape");
    assert_eq!(warning.severity(), crate::code::Severity::Warning);
    let denied = warning.denied();
    assert_eq!(
        (denied.severity(), denied.code()),
        (crate::code::Severity::Error, DiagnosticCode::warning(101))
    );
}
