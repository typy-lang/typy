//! Finds constructs that the current release does not compile (detailed design section 7.2, step 1).

mod class;
mod expression;
mod feature;
mod statement;

use typy_diagnostics::{Diagnostic, DiagnosticCode, Diagnostics, RuleId, Span};
use typy_source::syntax::{Stmt, SyntaxTree};

use feature::Feature;

/// Returns a diagnostic for every construct of `tree` that the 0.1 feature table does not accept.
#[must_use]
pub(crate) fn check(tree: &SyntaxTree) -> Diagnostics {
    let mut walker = Walker {
        diagnostics: Diagnostics::new(),
        loop_depth: 0,
    };
    walker.statements(&tree.body, Place::ModuleTop);
    walker.diagnostics
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Place {
    ModuleTop,
    ModuleBlock,
    Function,
    /// The body of a class, which holds its methods.
    Class,
}

struct Walker {
    diagnostics: Diagnostics,
    loop_depth: usize,
}

impl Walker {
    pub(super) fn report(&mut self, span: Span, feature: Feature) {
        let diagnostic = match feature {
            Feature::Planned {
                what,
                release,
                rule,
            } => Diagnostic::new(
                DiagnosticCode::error(201),
                span,
                format!("{what} are planned for TyPy {release} ({rule})"),
            )
            .with_rule(rule),
            Feature::AfterOne { what } => Diagnostic::new(
                DiagnosticCode::error(201),
                span,
                format!("{what} are planned for a release after TyPy 1.0"),
            )
            .with_rule(RuleId::STD_04),
            Feature::Excluded { what } => Diagnostic::new(
                DiagnosticCode::error(202),
                span,
                format!("{what} cannot be compiled by TyPy with CPython's meaning"),
            )
            .with_rule(RuleId::STD_04),
        };
        self.diagnostics.push(diagnostic);
    }

    /// Reports what CPython's compiler rejects as a `SyntaxError` but the parser accepts (STD-01).
    pub(super) fn syntax_error(&mut self, span: Span, message: &str) {
        self.diagnostics.push(
            Diagnostic::new(DiagnosticCode::error(103), span, message).with_rule(RuleId::STD_01),
        );
    }

    pub(super) fn statements(&mut self, statements: &[Stmt], place: Place) {
        for statement in statements {
            self.statement(statement, place);
        }
    }

    pub(super) fn block(&mut self, statements: &[Stmt], place: Place) {
        let inner = if place == Place::ModuleTop {
            Place::ModuleBlock
        } else {
            place
        };
        self.statements(statements, inner);
    }

    pub(super) fn loop_body(&mut self, body: &[Stmt], place: Place) {
        self.loop_depth += 1;
        self.block(body, place);
        self.loop_depth -= 1;
    }
}

#[cfg(test)]
mod tests;
