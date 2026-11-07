//! Parses a source file with `ruff_python_parser` and converts the result to TyPy's syntax tree.
//!
//! No type of `ruff_python_parser` or `ruff_python_ast` leaves this module (ADR-0005).

mod expression;
mod operator;
mod statement;

use ruff_python_ast::{self as ast, PythonVersion};
use ruff_python_parser::{Mode, ParseOptions, parse_unchecked};
use ruff_text_size::{Ranged, TextRange};
use typy_diagnostics::{Diagnostic, DiagnosticCode, FileId, RuleId, Span, TextOffset};

use crate::settings::PythonFloor;
use crate::source_file::SourceFile;
use crate::syntax::{Identifier, SyntaxTree};

/// Parses `file` with `floor` as the target version; any syntax error, or syntax newer than the floor, is an E0103 (STD-01).
pub fn parse(file: &SourceFile, floor: PythonFloor) -> Result<SyntaxTree, Vec<Diagnostic>> {
    let version = PythonVersion {
        major: 3,
        minor: floor.minor(),
    };
    let options = ParseOptions::from(Mode::Module).with_target_version(version);
    let parsed = parse_unchecked(file.text(), options);
    let converter = Converter {
        file: file.id(),
        text: file.text(),
    };

    let mut errors: Vec<Diagnostic> = parsed
        .errors()
        .iter()
        .map(|error| converter.syntax_error(error.location, &error.error.to_string()))
        .collect();
    errors.extend(
        parsed
            .unsupported_syntax_errors()
            .iter()
            .map(|error| converter.syntax_error(error.range, &error.to_string())),
    );
    if !errors.is_empty() {
        return Err(errors);
    }
    let ast::Mod::Module(module) = parsed.into_syntax() else {
        unreachable!("Mode::Module always produces a module");
    };
    Ok(SyntaxTree {
        body: converter.statements(&module.body),
    })
}

/// Rewrites a parser message as one sentence that starts in lower case and has no final period (DIA-01).
fn sentence(message: &str) -> String {
    let trimmed = message.trim().trim_end_matches('.');
    let mut characters = trimmed.chars();
    match characters.next() {
        Some(first) => first.to_lowercase().chain(characters).collect(),
        None => "invalid syntax".to_owned(),
    }
}

struct Converter<'text> {
    file: FileId,
    text: &'text str,
}

impl Converter<'_> {
    pub(super) fn span(&self, range: TextRange) -> Span {
        let start = TextOffset(range.start().to_u32());
        let end = TextOffset(range.end().to_u32());
        Span::new(self.file, start, end).expect("the parser produces ordered ranges")
    }

    pub(super) fn syntax_error(&self, range: TextRange, message: &str) -> Diagnostic {
        Diagnostic::new(
            DiagnosticCode::error(103),
            self.span(range),
            sentence(message),
        )
        .with_rule(RuleId::STD_01)
    }

    pub(super) fn identifier(&self, identifier: &ast::Identifier) -> Identifier {
        Identifier {
            name: identifier.as_str().to_owned(),
            span: self.span(identifier.range()),
        }
    }
}

#[cfg(test)]
mod tests;
