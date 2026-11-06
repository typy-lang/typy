//! One diagnostic, and the collection of diagnostics a command reports.

use crate::code::{DiagnosticCode, Severity};
use crate::render::SourceLookup;
use crate::rule::RuleId;
use crate::span::Span;

/// A secondary location with a short text, shown under the source line.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Label {
    /// Where the label points.
    pub span: Span,
    /// What the label says, such as "this is a str".
    pub text: String,
}

/// A certain fix: the text that replaces one span (DIA-02).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Fix {
    /// The text to replace.
    pub span: Span,
    /// The text that replaces it.
    pub replacement: String,
}

/// One error or warning, with its code, location, message and optional details.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Diagnostic {
    code: DiagnosticCode,
    span: Span,
    message: String,
    labels: Vec<Label>,
    help: Option<String>,
    fix: Option<Fix>,
    rule: Option<RuleId>,
}

impl Diagnostic {
    /// Returns a diagnostic with a code, a primary span and a one-sentence message (DIA-01).
    #[must_use]
    pub fn new(code: DiagnosticCode, span: Span, message: impl Into<String>) -> Self {
        let message = message.into();
        debug_assert!(
            !message.ends_with('.'),
            "a diagnostic message has no final period"
        );
        Self {
            code,
            span,
            message,
            labels: Vec::new(),
            help: None,
            fix: None,
            rule: None,
        }
    }

    /// Adds a label at another span.
    #[must_use]
    pub fn with_label(mut self, span: Span, text: impl Into<String>) -> Self {
        self.labels.push(Label {
            span,
            text: text.into(),
        });
        self
    }

    /// Adds a hint shown after `help:` when the fix is not certain.
    #[must_use]
    pub fn with_help(mut self, help: impl Into<String>) -> Self {
        self.help = Some(help.into());
        self
    }

    /// Adds a certain fix, shown as the corrected line (DIA-02).
    #[must_use]
    pub fn with_fix(mut self, span: Span, replacement: impl Into<String>) -> Self {
        self.fix = Some(Fix {
            span,
            replacement: replacement.into(),
        });
        self
    }

    /// Names the rule a program falls outside (DIA-03).
    #[must_use]
    pub fn with_rule(mut self, rule: RuleId) -> Self {
        self.rule = Some(rule);
        self
    }

    /// Returns the code.
    #[must_use]
    pub fn code(&self) -> DiagnosticCode {
        self.code
    }

    /// Returns the severity the code gives.
    #[must_use]
    pub fn severity(&self) -> Severity {
        self.code.severity()
    }

    /// Returns the primary span.
    #[must_use]
    pub fn span(&self) -> Span {
        self.span
    }

    /// Returns the message.
    #[must_use]
    pub fn message(&self) -> &str {
        &self.message
    }

    /// Returns the secondary labels.
    #[must_use]
    pub fn labels(&self) -> &[Label] {
        &self.labels
    }

    /// Returns the hint, if any.
    #[must_use]
    pub fn help(&self) -> Option<&str> {
        self.help.as_deref()
    }

    /// Returns the certain fix, if any.
    #[must_use]
    pub fn fix(&self) -> Option<&Fix> {
        self.fix.as_ref()
    }

    /// Returns the rule, if any.
    #[must_use]
    pub fn rule(&self) -> Option<RuleId> {
        self.rule
    }
}

/// The diagnostics of one command, without duplicates.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Diagnostics {
    items: Vec<Diagnostic>,
}

impl Diagnostics {
    /// Returns an empty collection.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Adds a diagnostic, unless one with the same code and span is already present.
    pub fn push(&mut self, diagnostic: Diagnostic) {
        let duplicate = self
            .items
            .iter()
            .any(|item| item.code == diagnostic.code && item.span == diagnostic.span);
        if !duplicate {
            self.items.push(diagnostic);
        }
    }

    /// Adds every diagnostic of another collection.
    pub fn extend(&mut self, other: Self) {
        for diagnostic in other.items {
            self.push(diagnostic);
        }
    }

    /// Returns true when any diagnostic is an error.
    #[must_use]
    pub fn has_errors(&self) -> bool {
        self.items
            .iter()
            .any(|item| item.severity() == Severity::Error)
    }

    /// Returns the number of diagnostics.
    #[must_use]
    pub fn len(&self) -> usize {
        self.items.len()
    }

    /// Returns true when there are none.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    /// Returns the diagnostics sorted by file path, line, column and code.
    #[must_use]
    pub fn into_sorted(mut self, lookup: &dyn SourceLookup) -> Vec<Diagnostic> {
        self.items.sort_by_cached_key(|item| {
            let span = item.span;
            let position = lookup.position(span.file(), span.start());
            (lookup.path(span.file()).to_owned(), position, item.code)
        });
        self.items
    }
}

impl From<Diagnostic> for Diagnostics {
    fn from(diagnostic: Diagnostic) -> Self {
        Self {
            items: vec![diagnostic],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{Diagnostic, Diagnostics};
    use crate::code::DiagnosticCode;
    use crate::render::SourceLookup;
    use crate::span::{FileId, Position, Span, TextOffset};

    struct Files;

    #[allow(
        clippy::unnecessary_literal_bound,
        reason = "the trait ties the returned text to self"
    )]
    impl SourceLookup for Files {
        fn path(&self, file: FileId) -> &str {
            if file.0 == 0 { "b.py" } else { "a.py" }
        }
        fn position(&self, _file: FileId, offset: TextOffset) -> Position {
            Position {
                line: offset.0 / 10 + 1,
                column: offset.0 % 10 + 1,
            }
        }
        fn line_text(&self, _file: FileId, _line: u32) -> &str {
            ""
        }
    }

    fn at(file: u32, start: u32) -> Span {
        Span::new(FileId(file), TextOffset(start), TextOffset(start + 1)).unwrap()
    }

    #[test]
    fn same_code_at_the_same_span_is_kept_once() {
        let mut diagnostics = Diagnostics::new();
        diagnostics.push(Diagnostic::new(
            DiagnosticCode::error(402),
            at(0, 3),
            "first",
        ));
        diagnostics.push(Diagnostic::new(
            DiagnosticCode::error(402),
            at(0, 3),
            "second",
        ));
        diagnostics.push(Diagnostic::new(
            DiagnosticCode::error(401),
            at(0, 3),
            "other code",
        ));
        assert_eq!(diagnostics.len(), 2);
    }

    #[test]
    fn warnings_alone_are_not_errors() {
        let mut diagnostics = Diagnostics::new();
        assert!(!diagnostics.has_errors());
        assert!(diagnostics.is_empty());
        diagnostics.push(Diagnostic::new(
            DiagnosticCode::warning(501),
            at(0, 0),
            "unused",
        ));
        assert!(!diagnostics.has_errors());
        diagnostics.push(Diagnostic::new(
            DiagnosticCode::error(401),
            at(0, 0),
            "bad operands",
        ));
        assert!(diagnostics.has_errors());
    }

    #[test]
    fn sorted_order_is_path_then_line_then_column_then_code() {
        let mut diagnostics = Diagnostics::new();
        diagnostics.push(Diagnostic::new(
            DiagnosticCode::error(402),
            at(0, 21),
            "b line 3",
        ));
        diagnostics.push(Diagnostic::new(
            DiagnosticCode::error(402),
            at(0, 5),
            "b line 1",
        ));
        diagnostics.push(Diagnostic::new(
            DiagnosticCode::error(401),
            at(0, 5),
            "b line 1 lower code",
        ));
        diagnostics.push(Diagnostic::new(
            DiagnosticCode::error(999),
            at(1, 30),
            "a line 4",
        ));
        let messages: Vec<String> = diagnostics
            .into_sorted(&Files)
            .iter()
            .map(|item| item.message().to_owned())
            .collect();
        assert_eq!(
            messages,
            ["a line 4", "b line 1 lower code", "b line 1", "b line 3"]
        );
    }

    #[test]
    fn extend_keeps_one_copy_of_each_diagnostic() {
        let mut first =
            Diagnostics::from(Diagnostic::new(DiagnosticCode::error(401), at(0, 0), "x"));
        let second = Diagnostics::from(Diagnostic::new(DiagnosticCode::error(401), at(0, 0), "x"));
        first.extend(second);
        assert_eq!(first.len(), 1);
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
    }
}
