//! Checks statements against the feature table.

use typy_diagnostics::RuleId;
use typy_source::syntax::{Expr, ParameterKind, Stmt};

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
                type_parameter_count,
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
                if let Some(decorator) = decorators.first() {
                    self.report(
                        decorator.span(),
                        planned("decorators", "1.0", RuleId::FUN_06),
                    );
                }
                if *type_parameter_count > 0 {
                    self.report(*span, planned("type parameters", "0.4", RuleId::GEN_01));
                }
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
                type_parameter_count,
                span,
                ..
            } => self.class(
                &super::class::ClassParts {
                    bases,
                    keyword_count: *keyword_count,
                    body,
                    decorators,
                    type_parameter_count: *type_parameter_count,
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
                for block in statement.try_blocks() {
                    self.block(block, place);
                }
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
            } => {
                self.report(*span, planned("lazy imports", "0.3", RuleId::MOD_06));
            }
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
            // The checker resolves each import against the modules of the program.
            Stmt::Import { .. }
            | Stmt::ImportFrom { .. }
            | Stmt::Pass { .. }
            | Stmt::Break { .. }
            | Stmt::Continue { .. } => {}
            Stmt::Other { kind, span } => self.report(*span, feature(*kind)),
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
