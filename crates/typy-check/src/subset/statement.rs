//! Checks statements against the feature table.

use typy_diagnostics::{Diagnostic, DiagnosticCode, RuleId};
use typy_source::syntax::{Expr, ParameterKind, Stmt, TypeParameter, TypeParameterKind};

use super::feature::{Feature, feature, planned};
use super::{Place, Walker};

impl Walker {
    #[allow(clippy::too_many_lines, reason = "one arm per statement kind")]
    pub(super) fn statement(&mut self, statement: &Stmt, place: Place) {
        match statement {
            Stmt::FunctionDef {
                parameters,
                returns,
                body,
                decorators,
                type_parameters,
                span,
                ..
            } => {
                match place {
                    Place::Function => {
                        self.report(*span, planned("nested functions", "1.0", RuleId::FUN_04));
                    }
                    Place::ModuleBlock => self.report(
                        *span,
                        Feature::Excluded {
                            what: "functions defined inside `if`, `for` or `while` blocks",
                        },
                    ),
                    Place::ModuleTop | Place::Class => {}
                }
                // A method may carry one of the decorators of CLS-06; the checker reads which.
                let method_decorator = place == Place::Class
                    && matches!(decorators.as_slice(), [one] if is_method_decorator(one));
                if !method_decorator && let Some(decorator) = decorators.first() {
                    self.report(
                        decorator.span(),
                        planned("decorators", "1.0", RuleId::FUN_06),
                    );
                }
                self.type_parameters(type_parameters);
                for parameter in parameters {
                    match parameter.kind {
                        ParameterKind::Normal => {}
                        ParameterKind::PositionalOnly | ParameterKind::KeywordOnly => self.report(
                            parameter.span,
                            planned(
                                "positional-only and keyword-only parameters",
                                "1.0",
                                RuleId::FUN_03,
                            ),
                        ),
                        ParameterKind::VarPositional | ParameterKind::VarKeyword => self.report(
                            parameter.span,
                            planned("`*args` and `**kwargs` parameters", "1.0", RuleId::EXP_17),
                        ),
                    }
                    if let Some(annotation) = &parameter.annotation {
                        self.annotation(annotation);
                    }
                    match &parameter.default {
                        None | Some(Expr::Constant { .. }) => {}
                        Some(default) => self.report(
                            default.span(),
                            planned("default values other than constants", "1.0", RuleId::EXP_16),
                        ),
                    }
                }
                if let Some(returns) = returns {
                    self.annotation(returns);
                }
                let outer_loops = std::mem::take(&mut self.loop_depth);
                let names = parameters
                    .iter()
                    .map(|parameter| parameter.name.name.as_str());
                let outer_scope = self
                    .scope
                    .replace(super::global_scope::GlobalScope::of_parameters(names));
                self.statements(body, Place::Function);
                self.scope = outer_scope;
                self.loop_depth = outer_loops;
            }
            Stmt::ClassDef {
                bases,
                keyword_count,
                body,
                decorators,
                type_parameters,
                span,
                ..
            } => self.class(
                &super::class::ClassParts {
                    bases,
                    keyword_count: *keyword_count,
                    body,
                    decorators,
                    type_parameters,
                    span: *span,
                },
                place,
            ),
            Stmt::Return { value, span } => {
                if place != Place::Function {
                    self.syntax_error(*span, "`return` outside a function");
                }
                if let Some(value) = value {
                    self.expression(value);
                }
            }
            Stmt::With { items, body, .. } => {
                for item in items {
                    self.expression(&item.context);
                    if let Some(target) = &item.target {
                        self.name_target(target);
                    }
                }
                self.block(body, place);
            }
            Stmt::Delete { targets, .. } => {
                for target in targets {
                    self.expression(target);
                }
            }
            Stmt::Assert { test, message, .. } => {
                // CPython warns, since a tuple with items is always true (DIA-08).
                if let Expr::Tuple { elements, span } = test
                    && !elements.is_empty()
                {
                    self.diagnostics.push(
                        Diagnostic::new(
                            DiagnosticCode::warning(301),
                            *span,
                            "this assertion is always true, since a tuple with items is true",
                        )
                        .with_rule(RuleId::DIA_08)
                        .with_help(
                            "remove the parentheses to give the test and its message: \
                             `assert test, message`",
                        ),
                    );
                }
                for value in std::iter::once(test).chain(message) {
                    self.expression(value);
                }
            }
            Stmt::Raise {
                exception, cause, ..
            } => {
                for value in exception.iter().chain(cause) {
                    self.expression(value);
                }
            }
            Stmt::Assign { targets, value, .. } => {
                for target in targets {
                    self.name_target(target);
                }
                self.expression(value);
            }
            Stmt::AugAssign { target, value, .. } => {
                self.name_target(target);
                self.expression(value);
            }
            Stmt::AnnAssign {
                target,
                annotation,
                value,
                ..
            } => {
                if let Expr::Name { id, span } = target {
                    self.annotated_name(id, *span);
                }
                self.name_target(target);
                self.annotation(annotation);
                if let Some(value) = value {
                    self.expression(value);
                }
            }
            Stmt::For {
                target,
                iter,
                body,
                orelse,
                ..
            } => {
                self.name_target(target);
                // The checker decides by type what a `for` may loop over (CST-03).
                self.expression(iter);
                self.loop_body(body, place);
                self.block(orelse, place);
            }
            Stmt::While {
                test, body, orelse, ..
            } => {
                self.expression(test);
                self.loop_body(body, place);
                self.block(orelse, place);
            }
            Stmt::If {
                test, body, orelse, ..
            } => {
                self.expression(test);
                self.block(body, place);
                self.block(orelse, place);
            }
            Stmt::Try { handlers, .. } => {
                for class in handlers.iter().filter_map(|handler| handler.class.as_ref()) {
                    self.expression(class);
                }
                self.try_depth += 1;
                for block in statement.try_blocks() {
                    self.block(block, place);
                }
                self.try_depth -= 1;
            }
            Stmt::Import {
                is_lazy: true,
                span,
                ..
            }
            | Stmt::ImportFrom {
                is_lazy: true,
                span,
                ..
            } => self.lazy_import(statement, *span, place),
            Stmt::ImportFrom { names, span, .. }
                if names.iter().any(|alias| alias.name.name == "*") =>
            {
                self.report(
                    *span,
                    Feature::AfterOne {
                        what: "imports of every name with `from module import *`",
                    },
                );
            }
            Stmt::Expr { value, .. } => self.expression(value),
            Stmt::Break { span } if self.loop_depth == 0 => {
                self.syntax_error(*span, "`break` outside a loop");
            }
            Stmt::Continue { span } if self.loop_depth == 0 => {
                self.syntax_error(*span, "`continue` outside a loop");
            }
            Stmt::Global { names, .. } => self.global_statement(names),
            Stmt::TypeAlias {
                type_parameters,
                value,
                span,
                ..
            } => {
                // The checker expands the aliases of the module's top level only.
                if place != Place::ModuleTop {
                    self.report(
                        *span,
                        planned(
                            "`type` statements inside functions, classes and blocks",
                            "0.5",
                            RuleId::SMT_12,
                        ),
                    );
                }
                if !type_parameters.is_empty() {
                    self.report(
                        *span,
                        planned("type parameters of aliases", "0.4", RuleId::GEN_01),
                    );
                }
                self.annotation(value);
            }
            // The checker resolves each import against the modules of the program.
            Stmt::Import { .. }
            | Stmt::ImportFrom { .. }
            | Stmt::Pass { .. }
            | Stmt::Break { .. }
            | Stmt::Continue { .. } => {}
            Stmt::Other { kind, span } => self.report(*span, feature(*kind)),
        }
    }

    /// Checks a list of type parameters: a bound or the constraints are annotations, and
    /// `*Ts`, `**P` and defaults are planned (GEN-01, GEN-06).
    pub(super) fn type_parameters(&mut self, parameters: &[TypeParameter]) {
        for parameter in parameters {
            if parameter.kind != TypeParameterKind::Type {
                self.report(
                    parameter.span,
                    planned("`*Ts` and `**P` type parameters", "1.0", RuleId::GEN_06),
                );
            }
            if parameter.default.is_some() {
                self.report(
                    parameter.span,
                    planned("defaults of type parameters", "1.0", RuleId::GEN_06),
                );
            }
            match &parameter.bound {
                Some(Expr::Tuple { elements, .. }) => {
                    for constraint in elements {
                        self.annotation(constraint);
                    }
                }
                Some(bound) => self.annotation(bound),
                None => {}
            }
        }
    }

    pub(super) fn name_target(&mut self, target: &Expr) {
        match target {
            Expr::Name { id, .. } => self.assigned_name(id),
            // An item of a list; the checker decides what may be indexed (DAT-07).
            Expr::Subscript { value, index, .. } => {
                self.expression(value);
                self.expression(index);
            }
            // An unpacking target: each element is itself a target (SMT-02).
            Expr::Tuple { elements, .. } | Expr::List { elements, .. } => {
                for element in elements {
                    self.name_target(element);
                }
            }
            // An attribute of an instance; the checker decides which may be assigned (CLS-02).
            Expr::Attribute { value, .. } => self.expression(value),
            other => self.expression(other),
        }
    }
}

/// Returns true for `@staticmethod`, `@classmethod`, `@property` and `@name.setter`, the
/// decorators of a method (CLS-06).
fn is_method_decorator(decorator: &Expr) -> bool {
    match decorator {
        Expr::Name { id, .. } => matches!(id.as_str(), "staticmethod" | "classmethod" | "property"),
        Expr::Attribute {
            value, attribute, ..
        } => attribute.name == "setter" && matches!(value.as_ref(), Expr::Name { .. }),
        _ => false,
    }
}
