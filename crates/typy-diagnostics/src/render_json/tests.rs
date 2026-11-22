//! Tests of `render_json`.

use super::{render_json, string};
use crate::code::DiagnosticCode;
use crate::diagnostic::Diagnostic;
use crate::render::SourceLookup;
use crate::rule::RuleId;
use crate::span::{FileId, Position, Span, TextOffset};
use proptest::prelude::*;

/// One file, `app.py`, whose offsets are columns of line 1.
struct OneLine;

impl SourceLookup for OneLine {
    fn path(&self, _: FileId) -> &'static str {
        "app.py"
    }
    fn position(&self, _: FileId, offset: TextOffset) -> Position {
        Position {
            line: 1,
            column: offset.0 + 1,
        }
    }
    fn line_text(&self, _: FileId, _: u32) -> &'static str {
        "x = \"a\""
    }
}

fn span(start: u32, end: u32) -> Span {
    Span::new(FileId(0), TextOffset(start), TextOffset(end)).unwrap()
}

/// Reads back a JSON string `string` wrote, as a JSON parser would.
fn unescape(text: &str) -> String {
    let inner = &text[1..text.len() - 1];
    let mut value = String::new();
    let mut characters = inner.chars();
    while let Some(character) = characters.next() {
        if character != '\\' {
            value.push(character);
            continue;
        }
        match characters.next() {
            Some('n') => value.push('\n'),
            Some('r') => value.push('\r'),
            Some('t') => value.push('\t'),
            Some('u') => {
                let digits: String = characters.by_ref().take(4).collect();
                let code = u32::from_str_radix(&digits, 16).unwrap();
                value.push(char::from_u32(code).unwrap());
            }
            Some(other) => value.push(other),
            None => panic!("a backslash ends {text}"),
        }
    }
    value
}

#[test]
fn a_located_diagnostic_gives_every_field_on_one_line() {
    let diagnostic = Diagnostic::new(
        DiagnosticCode::error(402),
        span(4, 7),
        "`x` was declared as int and is assigned \"a\"",
    )
    .with_label(span(0, 1), "the variable")
    .with_rule(RuleId::TYP_03)
    .with_help("assign an int")
    .with_fix(span(4, 7), "1");
    assert_eq!(
        render_json(&diagnostic, &OneLine),
        concat!(
            "{\"code\":\"E0402\",\"severity\":\"error\",",
            "\"message\":\"`x` was declared as int and is assigned \\\"a\\\"\",",
            "\"file\":\"app.py\",\"line\":1,\"column\":5,\"end_line\":1,\"end_column\":8,",
            "\"rule\":\"TYP-03\",\"help\":\"assign an int\",",
            "\"labels\":[{\"line\":1,\"column\":1,\"text\":\"the variable\"}],",
            "\"fix\":{\"line\":1,\"column\":5,\"end_line\":1,\"end_column\":8,",
            "\"replacement\":\"1\"}}\n"
        )
    );
}

#[test]
fn a_diagnostic_without_a_place_gives_null_for_it() {
    let diagnostic = Diagnostic::without_location(DiagnosticCode::error(701), "no `cc`");
    assert_eq!(
        render_json(&diagnostic, &OneLine),
        concat!(
            "{\"code\":\"E0701\",\"severity\":\"error\",\"message\":\"no `cc`\",",
            "\"file\":null,\"line\":null,\"column\":null,\"end_line\":null,",
            "\"end_column\":null,\"rule\":null,\"help\":null,\"labels\":[],\"fix\":null}\n"
        )
    );
}

#[test]
fn control_characters_are_escaped_as_code_points() {
    assert_eq!(
        string("tab\there\u{1}\u{7f}\r\u{e9}"),
        "\"tab\\there\\u0001\\u007f\\r\u{e9}\""
    );
}

proptest! {
    #[test]
    fn every_string_reads_back_as_itself(value in any::<String>()) {
        let text = string(&value);
        prop_assert!(!text.chars().any(|character| u32::from(character) < 0x20));
        prop_assert_eq!(unescape(&text), value);
    }
}
