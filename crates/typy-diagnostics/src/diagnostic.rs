//! One diagnostic: its code, location, message and optional details.

use crate::code::{DiagnosticCode, Severity};
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

#[cfg(test)]
mod tests {
    use super::Diagnostic;
    use crate::code::DiagnosticCode;
    use crate::span::{FileId, Span, TextOffset};

    fn at(file: u32, start: u32) -> Span {
        Span::new(FileId(file), TextOffset(start), TextOffset(start + 1)).unwrap()
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
