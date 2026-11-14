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
    pub(super) type_parameters: &'tree [typy_source::syntax::TypeParameter],
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
        // A generic class, or one of its copies, as a base is planned (GEN-08).
        let generic_base = class.bases.iter().any(|base| {
            let named = match base {
                Expr::Subscript { value, .. } => value.as_ref(),
                other => other,
            };
            matches!(named, Expr::Name { id, .. } if self.generic_classes.contains(id))
        });
        if generic_base {
            self.report(
                class.span,
                planned(
                    "classes derived from a generic class",
                    "0.5",
                    RuleId::GEN_08,
                ),
            );
            return;
        }
        if !class.type_parameters.is_empty() {
            self.type_parameters(class.type_parameters);
            if !class.bases.is_empty() || !class.decorators.is_empty() {
                self.report(
                    class.span,
                    planned(
                        "generic classes with a base or a decorator",
                        "0.5",
                        RuleId::GEN_08,
                    ),
                );
            }
        }
        // One base is resolved with the other classes, which reports one that is not
        // `object`, an exception class or a class of the program (CLS-03, CLS-04).
        match class.bases {
            [] | [_] => {}
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
            Stmt::FunctionDef { .. } | Stmt::TypeAlias { .. } => {
                self.statement(statement, Place::Class);
            }
            // A name declared with or without a value, and a value given to one name, which
            // the checker reads as a class variable or the default of an attribute (CLS-11).
            Stmt::AnnAssign {
                target: Expr::Name { .. },
                annotation,
                value,
                ..
            } => {
                self.annotation(annotation);
                if let Some(value) = value {
                    self.expression(value);
                }
            }
            Stmt::Assign { targets, value, .. }
                if matches!(targets.as_slice(), [Expr::Name { .. }]) =>
            {
                self.expression(value);
            }
            Stmt::AnnAssign { span, .. } | Stmt::Assign { span, .. } => {
                self.report(
                    *span,
                    Feature::Excluded {
                        what: "assignments in a class body to anything but one name",
                    },
                );
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
