//! Tests of `warning_levels`.

use super::WarningLevels;
use crate::code::{DiagnosticCode, Severity};
use crate::diagnostic::Diagnostic;
use crate::diagnostics::Diagnostics;
use crate::warning_level::WarningLevel;
use crate::warning_level::WarningLevel::{Allow, Deny, Warn};
use crate::warning_selector::WarningSelector::{self, All, Code};

const ESCAPE: DiagnosticCode = DiagnosticCode::warning(101);
const ASSERTION: DiagnosticCode = DiagnosticCode::warning(301);
const MATCH: DiagnosticCode = DiagnosticCode::warning(401);

/// The options of one command line, in order.
type Options = [(WarningSelector, WarningLevel)];

fn levels(options: &Options) -> WarningLevels {
    let mut levels = WarningLevels::default();
    for (selector, level) in options {
        levels.set(*selector, *level);
    }
    levels
}

#[test]
fn the_last_option_that_names_a_code_decides_its_level() {
    let cases: [(&Options, [WarningLevel; 3], &str); 7] = [
        (&[], [Warn, Warn, Warn], "warn"),
        (&[(All, Deny)], [Deny, Deny, Deny], "deny"),
        (
            &[(All, Deny), (Code(ESCAPE), Allow)],
            [Allow, Deny, Deny],
            "deny W0101=allow",
        ),
        (
            &[(Code(ESCAPE), Allow), (All, Deny)],
            [Deny, Deny, Deny],
            "deny",
        ),
        (
            &[
                (Code(MATCH), Deny),
                (Code(ESCAPE), Allow),
                (Code(MATCH), Warn),
            ],
            [Allow, Warn, Warn],
            "warn W0101=allow",
        ),
        (
            &[
                (Code(MATCH), Deny),
                (Code(ASSERTION), Allow),
                (Code(ESCAPE), Deny),
            ],
            [Deny, Allow, Deny],
            "warn W0101=deny W0301=allow W0401=deny",
        ),
        (
            &[(All, Allow), (Code(ESCAPE), Allow)],
            [Allow, Allow, Allow],
            "allow",
        ),
    ];
    for (options, expected, written) in cases {
        let levels = levels(options);
        assert_eq!(
            [ESCAPE, ASSERTION, MATCH].map(|code| levels.level(code)),
            expected,
            "{options:?}"
        );
        assert_eq!(levels.to_string(), written, "{options:?}");
    }
}

#[test]
fn options_that_give_each_code_the_same_level_give_equal_levels() {
    let given = levels(&[(Code(ESCAPE), Deny), (All, Warn), (Code(ASSERTION), Warn)]);
    assert_eq!(given, WarningLevels::default());
}

#[test]
fn applying_the_levels_drops_allowed_warnings_and_reports_denied_ones_as_errors() {
    let mut diagnostics = Diagnostics::new();
    diagnostics.push(Diagnostic::without_location(ESCAPE, "an escape"));
    diagnostics.push(Diagnostic::without_location(ASSERTION, "an assertion"));
    diagnostics.push(Diagnostic::without_location(MATCH, "a match"));
    diagnostics.push(Diagnostic::without_location(
        DiagnosticCode::error(402),
        "an error",
    ));
    let applied = levels(&[(Code(ESCAPE), Allow), (Code(MATCH), Deny)]).apply(&diagnostics);
    let kept: Vec<(DiagnosticCode, Severity)> = applied
        .iter()
        .map(|diagnostic| (diagnostic.code(), diagnostic.severity()))
        .collect();
    assert_eq!(
        kept,
        [
            (ASSERTION, Severity::Warning),
            (MATCH, Severity::Error),
            (DiagnosticCode::error(402), Severity::Error),
        ]
    );
    assert!(applied.has_errors());
}

#[test]
fn allowing_every_warning_leaves_only_the_errors() {
    let mut diagnostics = Diagnostics::new();
    diagnostics.push(Diagnostic::without_location(ESCAPE, "an escape"));
    assert!(levels(&[(All, Allow)]).apply(&diagnostics).is_empty());
}
