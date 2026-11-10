//! Writes diagnostics as text for a terminal (detailed design section 4.2).

use std::fmt::Write as _;

use crate::diagnostic::Diagnostic;
use crate::span::{FileId, Position, TextOffset};

/// What the renderer needs to know about the source files.
pub trait SourceLookup {
    /// Returns the file's path relative to the project root.
    fn path(&self, file: FileId) -> &str;
    /// Returns the line and column of a byte offset.
    fn position(&self, file: FileId, offset: TextOffset) -> Position;
    /// Returns the text of one line, from 1, without its line ending.
    fn line_text(&self, file: FileId, line: u32) -> &str;
}

/// Returns the diagnostic as text: the header, the location, the source line with a marker, and any rule, help and fix.
#[must_use]
pub fn render_text(diagnostic: &Diagnostic, lookup: &dyn SourceLookup) -> String {
    let Some(span) = diagnostic.span() else {
        return render_without_location(diagnostic);
    };
    let file = span.file();
    let start = lookup.position(file, span.start());
    let end = lookup.position(file, span.end());
    let gutter = digits(start.line);
    let blank = " ".repeat(gutter);
    let line = lookup.line_text(file, start.line);

    let mut text = String::new();
    let _ = writeln!(
        text,
        "{}[{}]: {}",
        diagnostic.severity().label(),
        diagnostic.code(),
        diagnostic.message()
    );
    let _ = writeln!(
        text,
        "{blank}--> {}:{}:{}",
        lookup.path(file),
        start.line,
        start.column
    );
    let _ = writeln!(text, "{blank} |");
    let _ = writeln!(text, "{:>gutter$} | {line}", start.line);
    let _ = writeln!(text, "{blank} | {}", marker(line, start, end));
    for label in diagnostic.labels() {
        let label_start = lookup.position(file, label.span.start());
        let _ = writeln!(
            text,
            "{blank} = {}:{}: {}",
            label_start.line, label_start.column, label.text
        );
    }
    if let Some(rule) = diagnostic.rule() {
        let _ = writeln!(text, "{blank} = rule: {rule}");
    }
    if let Some(help) = diagnostic.help() {
        let _ = writeln!(text, "help: {help}");
    }
    if let Some(fix) = diagnostic.fix() {
        let fix_start = lookup.position(file, fix.span.start());
        let fix_end = lookup.position(file, fix.span.end());
        if fix_start.line == fix_end.line {
            let source = lookup.line_text(file, fix_start.line);
            let corrected =
                replace_columns(source, fix_start.column, fix_end.column, &fix.replacement);
            let _ = writeln!(text, "help: write it as");
            let _ = writeln!(text, "{:>gutter$} | {corrected}", fix_start.line);
        }
    }
    text
}

/// Returns a diagnostic about no place in the source: the header, then any rule and help.
fn render_without_location(diagnostic: &Diagnostic) -> String {
    let mut text = String::new();
    let _ = writeln!(
        text,
        "{}[{}]: {}",
        diagnostic.severity().label(),
        diagnostic.code(),
        diagnostic.message()
    );
    if let Some(rule) = diagnostic.rule() {
        let _ = writeln!(text, " = rule: {rule}");
    }
    if let Some(help) = diagnostic.help() {
        let _ = writeln!(text, "help: {help}");
    }
    text
}

fn digits(number: u32) -> usize {
    number.to_string().len()
}

/// Returns spaces up to the start column and carets to the end column, or to the end of the line when the span runs on.
fn marker(line: &str, start: Position, end: Position) -> String {
    let line_length = u32::try_from(line.chars().count()).unwrap_or(u32::MAX);
    let last = if end.line == start.line {
        end.column
    } else {
        line_length + 1
    };
    let width = last.saturating_sub(start.column).max(1);
    let indent = start.column.saturating_sub(1);
    format!(
        "{}{}",
        " ".repeat(indent as usize),
        "^".repeat(width as usize)
    )
}

/// Returns the line with the code points from `start` up to `end`, both counted from 1, replaced.
fn replace_columns(line: &str, start: u32, end: u32, replacement: &str) -> String {
    let before: String = line
        .chars()
        .take(start.saturating_sub(1) as usize)
        .collect();
    let after: String = line.chars().skip(end.saturating_sub(1) as usize).collect();
    format!("{before}{replacement}{after}")
}

#[cfg(test)]
mod tests {
    use super::{SourceLookup, render_text};
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
        let source: &'static str =
            Box::leak(format!("{}x = 1\n", "\n".repeat(11)).into_boxed_str());
        let start = u32::try_from(source.find('x').unwrap()).unwrap();
        let diagnostic =
            Diagnostic::new(DiagnosticCode::error(402), span(start, start + 1), "wrong");
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
}
