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
    render(diagnostic, lookup, &Palette::PLAIN)
}

/// Returns the diagnostic as `render_text` writes it, coloured for a terminal as `rustc`
/// colours its own: the severity in red or yellow, the message in bold, the location and the
/// gutter in blue (CLI-26).
#[must_use]
pub fn render_coloured(diagnostic: &Diagnostic, lookup: &dyn SourceLookup) -> String {
    let palette = match diagnostic.severity() {
        crate::code::Severity::Error => Palette::ERROR,
        crate::code::Severity::Warning => Palette::WARNING,
    };
    render(diagnostic, lookup, &palette)
}

/// The escape sequences that colour each part of a diagnostic; all empty for plain text.
struct Palette {
    /// The severity, the code and the marker.
    severity: &'static str,
    /// The message and `help:`.
    bold: &'static str,
    /// The arrow, the line numbers and the gutter.
    gutter: &'static str,
    /// What ends each coloured part.
    reset: &'static str,
}

impl Palette {
    const PLAIN: Self = Self {
        severity: "",
        bold: "",
        gutter: "",
        reset: "",
    };
    const ERROR: Self = Self {
        severity: "\x1b[1;31m",
        bold: "\x1b[1m",
        gutter: "\x1b[1;34m",
        reset: "\x1b[0m",
    };
    const WARNING: Self = Self {
        severity: "\x1b[1;33m",
        ..Self::ERROR
    };
}

fn render(diagnostic: &Diagnostic, lookup: &dyn SourceLookup, palette: &Palette) -> String {
    let Palette {
        severity,
        bold,
        gutter: blue,
        reset,
    } = palette;
    let Some(span) = diagnostic.span() else {
        return render_without_location(diagnostic, palette);
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
        "{severity}{}[{}]{reset}{bold}: {}{reset}",
        diagnostic.severity().label(),
        diagnostic.code(),
        diagnostic.message()
    );
    let _ = writeln!(
        text,
        "{blank}{blue}-->{reset} {}:{}:{}",
        lookup.path(file),
        start.line,
        start.column
    );
    let _ = writeln!(text, "{blank} {blue}|{reset}");
    let _ = writeln!(text, "{blue}{:>gutter$} |{reset} {line}", start.line);
    let marker = marker(line, start, end);
    let carets = marker.trim_start();
    let indent = &marker[..marker.len() - carets.len()];
    let _ = writeln!(
        text,
        "{blank} {blue}|{reset} {indent}{severity}{carets}{reset}"
    );
    for label in diagnostic.labels() {
        let label_start = lookup.position(file, label.span.start());
        let _ = writeln!(
            text,
            "{blank} {blue}={reset} {}:{}: {}",
            label_start.line, label_start.column, label.text
        );
    }
    if let Some(rule) = diagnostic.rule() {
        let _ = writeln!(text, "{blank} {blue}={reset} rule: {rule}");
    }
    if let Some(help) = diagnostic.help() {
        let _ = writeln!(text, "{bold}help{reset}: {help}");
    }
    if let Some(fix) = diagnostic.fix() {
        let fix_start = lookup.position(file, fix.span.start());
        let fix_end = lookup.position(file, fix.span.end());
        if fix_start.line == fix_end.line {
            let source = lookup.line_text(file, fix_start.line);
            let corrected =
                replace_columns(source, fix_start.column, fix_end.column, &fix.replacement);
            let _ = writeln!(text, "{bold}help{reset}: write it as");
            let _ = writeln!(
                text,
                "{blue}{:>gutter$} |{reset} {corrected}",
                fix_start.line
            );
        }
    }
    text
}

/// Returns a diagnostic about no place in the source: the header, then any rule and help.
fn render_without_location(diagnostic: &Diagnostic, palette: &Palette) -> String {
    let Palette {
        severity,
        bold,
        gutter: blue,
        reset,
    } = palette;
    let mut text = String::new();
    let _ = writeln!(
        text,
        "{severity}{}[{}]{reset}{bold}: {}{reset}",
        diagnostic.severity().label(),
        diagnostic.code(),
        diagnostic.message()
    );
    if let Some(rule) = diagnostic.rule() {
        let _ = writeln!(text, " {blue}={reset} rule: {rule}");
    }
    if let Some(help) = diagnostic.help() {
        let _ = writeln!(text, "{bold}help{reset}: {help}");
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
