//! Checks class definitions against the feature table: a class at the top of the module, with
//! `object` or no base, holds attribute annotations and methods (CLS-01).

use typy_diagnostics::{RuleId, Span};
use typy_source::syntax::{Constant, Expr, Stmt};

use super::feature::{Feature, planned};
use super::{Place, Walker};

/// The parts of a `class` statement the subset checks.
pub(super) struct ClassParts<'tree> {
    pub(super) bases: &'tree [Expr],
    pub(super) keyword_count: usize,
    pub(super) body: &'tree [Stmt],
    pub(super) decorators: &'tree [Expr],
    pub(super) type_parameter_count: usize,
    pub(super) span: Span,
}

impl Walker {
    pub(super) fn class(&mut self, class: &ClassParts<'_>, place: Place) {
        if place != Place::ModuleTop {
            self.report(
                class.span,
                Feature::Excluded {
                    what: "classes defined inside functions, classes or blocks",
                },
            );
        }
        if let Some(decorator) = class.decorators.first() {
            self.report(
                decorator.span(),
                planned("class decorators", "0.4", RuleId::CLS_05),
            );
        }
        if class.type_parameter_count > 0 {
            self.report(
                class.span,
                planned("type parameters", "0.4", RuleId::GEN_01),
            );
        }
        // One base named by its name is resolved with the other classes: `object`, or an
        // exception class (CLS-03).
        match class.bases {
            [] | [Expr::Name { .. }] => {}
            [base] => self.report(
                base.span(),
                planned("base classes other than `object`", "0.4", RuleId::CLS_04),
            ),
            [_, second, ..] => self.report(
                second.span(),
                Feature::AfterOne {
                    what: "classes with several bases",
                },
            ),
        }
        if class.keyword_count > 0 {
            self.report(
                class.span,
                Feature::Excluded {
                    what: "class keywords such as `metaclass`",
                },
            );
        }
        for statement in class.body {
            self.class_statement(statement);
        }
    }

    fn class_statement(&mut self, statement: &Stmt) {
        match statement {
            Stmt::FunctionDef { .. } => self.statement(statement, Place::Class),
            Stmt::AnnAssign {
                target: Expr::Name { .. },
                annotation,
                value: None,
                ..
            } => self.annotation(annotation),
            Stmt::AnnAssign { span, .. } | Stmt::Assign { span, .. } => {
                self.report(*span, planned("class variables", "0.4", RuleId::CLS_11));
            }
            Stmt::Expr {
                value:
                    Expr::Constant {
                        value: Constant::Str(_),
                        ..
                    },
                ..
            }
            | Stmt::Pass { .. } => {}
            other => self.report(
                other.span(),
                Feature::Excluded {
                    what: "statements other than attribute annotations and methods in a class body",
                },
            ),
        }
    }
}
