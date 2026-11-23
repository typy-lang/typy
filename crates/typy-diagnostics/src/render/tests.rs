//! Tests of `render`.

use super::{SourceLookup, render_coloured, render_text};
use crate::code::DiagnosticCode;
use crate::diagnostic::Diagnostic;
use crate::rule::RuleId;
use crate::span::{FileId, Position, Span, TextOffset};

/// One file whose lines are split on `\n`; columns count code points.
struct OneFile(&'static str);

#[allow(
    clippy::unnecessary_literal_bound,
    reason = "the trait ties the returned text to self"
)]
impl SourceLookup for OneFile {
    fn path(&self, _file: FileId) -> &str {
        "app.py"
    }
    fn position(&self, _file: FileId, offset: TextOffset) -> Position {
        let before = &self.0[..offset.0 as usize];
        let line = before.matches('\n').count() + 1;
        let column = before.rsplit('\n').next().unwrap().chars().count() + 1;
        Position {
            line: u32::try_from(line).unwrap(),
            column: u32::try_from(column).unwrap(),
        }
    }
    fn line_text(&self, _file: FileId, line: u32) -> &str {
        self.0.split('\n').nth(line as usize - 1).unwrap_or("")
    }
}

const SOURCE: &str = "def f() -> None:\n    total: int = 0\n    total = \"done\"\n";

fn span(start: u32, end: u32) -> Span {
    Span::new(FileId(0), TextOffset(start), TextOffset(end)).unwrap()
}

#[test]
fn error_is_rendered_with_location_marker_and_rule() {
    let start = u32::try_from(SOURCE.find("\"done\"").unwrap()).unwrap();
    let diagnostic = Diagnostic::new(
        DiagnosticCode::error(402),
        span(start, start + 6),
        "`total` was declared as int and is assigned a str",
    )
    .with_rule(RuleId::TYP_03)
    .with_help("assign an int, or use a new variable");
    let expected = "\
error[E0402]: `total` was declared as int and is assigned a str
 --> app.py:3:13
  |
3 |     total = \"done\"
  |             ^^^^^^
  = rule: TYP-03
help: assign an int, or use a new variable
";
    assert_eq!(render_text(&diagnostic, &OneFile(SOURCE)), expected);
}

#[test]
fn the_margin_is_as_wide_as_the_line_number() {
    let source: &'static str = Box::leak(format!("{}x = 1\n", "\n".repeat(11)).into_boxed_str());
    let start = u32::try_from(source.find('x').unwrap()).unwrap();
    let diagnostic = Diagnostic::new(DiagnosticCode::error(402), span(start, start + 1), "wrong");
    let expected = "\
error[E0402]: wrong
  --> app.py:12:1
   |
12 | x = 1
   | ^
";
    assert_eq!(render_text(&diagnostic, &OneFile(source)), expected);
}

#[test]
fn certain_fix_is_rendered_as_the_corrected_line() {
    let start = u32::try_from(SOURCE.find("\"done\"").unwrap()).unwrap();
    let diagnostic = Diagnostic::new(
        DiagnosticCode::error(402),
        span(start, start + 6),
        "wrong type",
    )
    .with_fix(span(start, start + 6), "1");
    let text = render_text(&diagnostic, &OneFile(SOURCE));
    assert!(
        text.ends_with("help: write it as\n3 |     total = 1\n"),
        "{text}"
    );
}

#[test]
fn a_diagnostic_without_location_shows_its_header_rule_and_help() {
    let diagnostic = Diagnostic::without_location(DiagnosticCode::error(701), "`cc` not found")
        .with_rule(RuleId::A_01)
        .with_help("install a C toolchain");
    assert_eq!(
        render_text(&diagnostic, &OneFile(SOURCE)),
        "error[E0701]: `cc` not found\n = rule: A-01\nhelp: install a C toolchain\n"
    );
}

#[test]
fn a_fix_over_several_lines_is_not_shown_as_a_corrected_line() {
    let diagnostic = Diagnostic::new(DiagnosticCode::error(402), span(4, 22), "wrong")
        .with_fix(span(4, 22), "x");
    let text = render_text(&diagnostic, &OneFile(SOURCE));
    assert!(!text.contains("help: write it as"), "{text}");
}

#[test]
fn a_diagnostic_without_location_rule_or_help_is_its_header() {
    let diagnostic = Diagnostic::without_location(DiagnosticCode::error(702), "link failed");
    assert_eq!(
        render_text(&diagnostic, &OneFile(SOURCE)),
        "error[E0702]: link failed\n"
    );
}

#[test]
fn empty_span_gets_one_caret() {
    let diagnostic = Diagnostic::new(DiagnosticCode::error(103), span(0, 0), "unexpected end");
    let text = render_text(&diagnostic, &OneFile(SOURCE));
    assert!(text.contains("\n  | ^\n"), "{text}");
}

#[test]
fn span_over_several_lines_is_marked_to_the_end_of_its_first_line() {
    let diagnostic = Diagnostic::new(DiagnosticCode::error(103), span(4, 22), "runs on");
    let text = render_text(&diagnostic, &OneFile(SOURCE));
    assert!(text.contains("\n  |     ^^^^^^^^^^^^\n"), "{text}");
}

#[test]
fn warning_and_labels_are_rendered() {
    let diagnostic = Diagnostic::new(DiagnosticCode::warning(501), span(4, 5), "unused value")
        .with_label(span(21, 26), "assigned here");
    let text = render_text(&diagnostic, &OneFile(SOURCE));
    assert!(text.starts_with("warning[W0501]: unused value\n"), "{text}");
    assert!(text.contains("  = 2:5: assigned here\n"), "{text}");
}

#[test]
fn columns_count_code_points_not_bytes() {
    let source = "x = \"\u{e9}\u{e9}\" + 1\n";
    let start = u32::try_from(source.find('+').unwrap()).unwrap();
    let diagnostic = Diagnostic::new(
        DiagnosticCode::error(401),
        span(start, start + 1),
        "bad operands",
    );
    let text = render_text(&diagnostic, &OneFile(source));
    assert!(text.contains("app.py:1:10\n"), "{text}");
    assert!(text.contains("\n  |          ^\n"), "{text}");
}

/// Returns the text without its colour escape sequences.
fn without_colour(text: &str) -> String {
    let mut plain = String::new();
    let mut rest = text;
    while let Some(start) = rest.find('\x1b') {
        plain.push_str(&rest[..start]);
        let end = rest[start..].find('m').unwrap();
        rest = &rest[start + end + 1..];
    }
    plain.push_str(rest);
    plain
}

fn every_detail() -> Diagnostic {
    let start = u32::try_from(SOURCE.find("\"done\"").unwrap()).unwrap();
    Diagnostic::new(
        DiagnosticCode::error(402),
        span(start, start + 6),
        "`total` was declared as int and is assigned a str",
    )
    .with_label(span(21, 26), "declared here")
    .with_rule(RuleId::TYP_03)
    .with_help("assign an int")
    .with_fix(span(start, start + 6), "0")
}

#[test]
fn an_error_is_coloured_as_rustc_colours_one() {
    let expected = "\
\x1b[1;31merror[E0402]\x1b[0m\x1b[1m: `total` was declared as int and is assigned a str\x1b[0m
 \x1b[1;34m-->\x1b[0m app.py:3:13
  \x1b[1;34m|\x1b[0m
\x1b[1;34m3 |\x1b[0m     total = \"done\"
  \x1b[1;34m|\x1b[0m             \x1b[1;31m^^^^^^\x1b[0m
  \x1b[1;34m=\x1b[0m 2:5: declared here
  \x1b[1;34m=\x1b[0m rule: TYP-03
\x1b[1mhelp\x1b[0m: assign an int
\x1b[1mhelp\x1b[0m: write it as
\x1b[1;34m3 |\x1b[0m     total = 0
";
    assert_eq!(render_coloured(&every_detail(), &OneFile(SOURCE)), expected);
}

#[test]
fn coloured_text_is_the_plain_text_with_escape_sequences_added() {
    let diagnostics = [
        every_detail(),
        Diagnostic::new(
            DiagnosticCode::warning(101),
            span(4, 5),
            "an invalid escape",
        ),
        Diagnostic::new(
            DiagnosticCode::warning(101),
            span(4, 5),
            "an invalid escape",
        )
        .denied(),
        Diagnostic::without_location(DiagnosticCode::error(701), "`cc` not found")
            .with_rule(RuleId::A_01)
            .with_help("install a C toolchain"),
    ];
    for diagnostic in diagnostics {
        let coloured = render_coloured(&diagnostic, &OneFile(SOURCE));
        assert_eq!(
            without_colour(&coloured),
            render_text(&diagnostic, &OneFile(SOURCE))
        );
        assert!(coloured.contains('\x1b'), "{coloured}");
    }
}

#[test]
fn a_warning_is_yellow_and_a_denied_warning_is_a_red_error() {
    let warning = Diagnostic::new(
        DiagnosticCode::warning(101),
        span(4, 5),
        "an invalid escape",
    );
    let coloured = render_coloured(&warning, &OneFile(SOURCE));
    assert!(
        coloured.starts_with("\x1b[1;33mwarning[W0101]\x1b[0m"),
        "{coloured}"
    );
    let denied = render_coloured(&warning.denied(), &OneFile(SOURCE));
    assert!(
        denied.starts_with("\x1b[1;31merror[W0101]\x1b[0m"),
        "{denied}"
    );
}

#[test]
fn a_diagnostic_without_location_is_coloured_too() {
    let diagnostic = Diagnostic::without_location(DiagnosticCode::error(701), "`cc` not found")
        .with_rule(RuleId::A_01)
        .with_help("install a C toolchain");
    assert_eq!(
        render_coloured(&diagnostic, &OneFile(SOURCE)),
        "\x1b[1;31merror[E0701]\x1b[0m\x1b[1m: `cc` not found\x1b[0m\n \x1b[1;34m=\x1b[0m rule: \
         A-01\n\x1b[1mhelp\x1b[0m: install a C toolchain\n"
    );
}
