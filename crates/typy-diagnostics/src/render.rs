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
mod tests;
