//! Parses a source file with `ruff_python_parser` and converts the result to TyPy's syntax tree.
//!
//! No type of `ruff_python_parser` or `ruff_python_ast` leaves this module (ADR-0005).

mod escape;
mod expression;
mod limits;
mod operator;
mod statement;

use ruff_python_ast::{self as ast, PythonVersion};
use ruff_python_parser::{Mode, ParseOptions, parse_unchecked};
use std::cell::Cell;

use ruff_text_size::{Ranged, TextRange};
use typy_diagnostics::{Diagnostic, DiagnosticCode, FileId, RuleId, Span, TextOffset};

use crate::settings::PythonFloor;
use crate::source_file::SourceFile;
use crate::syntax::{Identifier, SyntaxTree};

/// A parsed file: its syntax tree and the warnings CPython would give while compiling it.
#[derive(Clone, Debug)]
pub struct Parsed {
    /// The syntax tree.
    pub tree: SyntaxTree,
    /// The warnings, such as W0101 for an invalid escape sequence (DIA-08).
    pub warnings: Vec<Diagnostic>,
}

/// Parses `file` with `floor` as the target version; any syntax error, or syntax newer than the floor, is an E0103 (STD-01).
pub fn parse(file: &SourceFile, floor: PythonFloor) -> Result<SyntaxTree, Vec<Diagnostic>> {
    parse_with_warnings(file, floor).map(|parsed| parsed.tree)
}

/// Parses `file` as `parse` does, and also returns its warnings.
pub fn parse_with_warnings(
    file: &SourceFile,
    floor: PythonFloor,
) -> Result<Parsed, Vec<Diagnostic>> {
    let version = PythonVersion {
        major: 3,
        minor: floor.minor(),
    };
    let converter = Converter {
        file: file.id(),
        text: file.text(),
        depth: Cell::new(0),
        too_deep: Cell::new(None),
    };
    if let Some(error) = converter.nesting_error() {
        return Err(vec![error]);
    }
    let options = ParseOptions::from(Mode::Module).with_target_version(version);
    let parsed = parse_unchecked(file.text(), options);

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
    let (warnings, errors) = converter.escape_diagnostics(parsed.tokens());
    if !errors.is_empty() {
        return Err(errors);
    }
    let ast::Mod::Module(module) = parsed.into_syntax() else {
        unreachable!("Mode::Module always produces a module");
    };
    let body = converter.statements(&module.body);
    if let Some(range) = converter.too_deep.get() {
        let message = format!(
            "the program nests more than {MAX_DEPTH} levels deep, past what CPython compiles"
        );
        return Err(vec![
            converter
                .syntax_error(range, &message)
                .with_rule(RuleId::LEX_11),
        ]);
    }
    Ok(Parsed {
        tree: SyntaxTree { body },
        warnings,
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
    /// How many expressions and statements enclose the one being converted.
    depth: Cell<usize>,
    /// Where the nesting first went past `MAX_DEPTH`.
    too_deep: Cell<Option<TextRange>>,
}

/// The deepest nesting of expressions and statements TyPy compiles: past the deepest program
/// CPython 3.12 to 3.15 compile, 22685 levels of `+` for 3.15, so every pass's recursion has
/// a bound that its stack holds (NFR-09).
pub const MAX_DEPTH: usize = 25_000;

impl Converter<'_> {
    /// Runs `convert` one level deeper, or, past `MAX_DEPTH`, records where and returns `None`
    /// without descending further.
    fn deeper<T>(&self, range: TextRange, convert: impl FnOnce() -> T) -> Option<T> {
        let depth = self.depth.get() + 1;
        if depth > MAX_DEPTH {
            if self.too_deep.get().is_none() {
                self.too_deep.set(Some(range));
            }
            return None;
        }
        self.depth.set(depth);
        let converted = convert();
        self.depth.set(depth - 1);
        Some(converted)
    }

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
