//! Renders a diagnostic as one line of JSON, for editors and CI (DIA-05, UC-10).

use std::fmt::Write as _;

use crate::diagnostic::Diagnostic;
use crate::render::SourceLookup;
use crate::span::Span;

/// Returns the diagnostic as one JSON object on one line, ended by a newline. Every key is
/// always present, in this order, with `null` where the diagnostic has nothing: `code`,
/// `severity`, `message`, `file`, `line`, `column`, `end_line`, `end_column`, `rule`, `help`,
/// `labels` and `fix`. Lines and columns count from 1, and columns count code points (DIA-01).
#[must_use]
pub fn render_json(diagnostic: &Diagnostic, lookup: &dyn SourceLookup) -> String {
    let mut text = String::from("{");
    field(&mut text, "code", &string(&diagnostic.code().to_string()));
    field(
        &mut text,
        "severity",
        &string(diagnostic.severity().label()),
    );
    field(&mut text, "message", &string(diagnostic.message()));
    let located = diagnostic.span().map(|span| place(span, lookup));
    let file = diagnostic
        .span()
        .map_or_else(null, |span| string(lookup.path(span.file())));
    field(&mut text, "file", &file);
    for (index, key) in ["line", "column", "end_line", "end_column"]
        .iter()
        .enumerate()
    {
        let value = located.map_or_else(null, |numbers| numbers[index].to_string());
        field(&mut text, key, &value);
    }
    let rule = diagnostic
        .rule()
        .map_or_else(null, |rule| string(&rule.to_string()));
    field(&mut text, "rule", &rule);
    field(
        &mut text,
        "help",
        &diagnostic.help().map_or_else(null, string),
    );
    let labels: Vec<String> = diagnostic
        .labels()
        .iter()
        .map(|label| {
            let [line, column, ..] = place(label.span, lookup);
            format!(
                "{{\"line\":{line},\"column\":{column},\"text\":{}}}",
                string(&label.text)
            )
        })
        .collect();
    field(&mut text, "labels", &format!("[{}]", labels.join(",")));
    let fix = diagnostic.fix().map_or_else(null, |fix| {
        let [line, column, end_line, end_column] = place(fix.span, lookup);
        format!(
            "{{\"line\":{line},\"column\":{column},\"end_line\":{end_line},\
             \"end_column\":{end_column},\"replacement\":{}}}",
            string(&fix.replacement)
        )
    });
    field(&mut text, "fix", &fix);
    text.push_str("}\n");
    text
}

/// Appends `"key":value`, after a comma unless it is the first field.
fn field(text: &mut String, key: &str, value: &str) {
    if text.len() > 1 {
        text.push(',');
    }
    let _ = write!(text, "\"{key}\":{value}");
}

fn null() -> String {
    "null".to_owned()
}

/// Returns the start line and column and the end line and column of a span.
fn place(span: Span, lookup: &dyn SourceLookup) -> [u32; 4] {
    let start = lookup.position(span.file(), span.start());
    let end = lookup.position(span.file(), span.end());
    [start.line, start.column, end.line, end.column]
}

/// Returns `value` as a JSON string. A quote, a backslash and each control character are
/// escaped; every other character is written as it is, since the output is UTF-8.
fn string(value: &str) -> String {
    let mut text = String::with_capacity(value.len() + 2);
    text.push('"');
    for character in value.chars() {
        match character {
            '"' => text.push_str("\\\""),
            '\\' => text.push_str("\\\\"),
            '\n' => text.push_str("\\n"),
            '\r' => text.push_str("\\r"),
            '\t' => text.push_str("\\t"),
            control if u32::from(control) < 0x20 || control == '\u{7f}' => {
                let _ = write!(text, "\\u{:04x}", u32::from(control));
            }
            other => text.push(other),
        }
    }
    text.push('"');
    text
}

#[cfg(test)]
mod tests;
