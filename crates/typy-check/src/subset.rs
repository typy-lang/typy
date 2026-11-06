//! Finds constructs that the current release does not compile (detailed design section 7.2, step 1).

use typy_diagnostics::{Diagnostic, DiagnosticCode, Diagnostics, RuleId, Span};
use typy_source::syntax::{
    BinaryOperator, CompareOperator, Constant, Expr, NodeKind, ParameterKind, Stmt, SyntaxTree,
};

/// Whether a construct comes in a later release or is not compiled at all.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Feature {
    /// Provided by a later release, under a requirement.
    Planned {
        what: &'static str,
        release: &'static str,
        rule: RuleId,
    },
    /// Listed in section 6.2 of the specification, planned for after 1.0.
    AfterOne { what: &'static str },
    /// Not compiled with CPython's meaning (section 6.1 of the specification).
    Excluded { what: &'static str },
}

const fn planned(what: &'static str, release: &'static str, rule: RuleId) -> Feature {
    Feature::Planned {
        what,
        release,
        rule,
    }
}

/// Classifies every node kind the syntax tree does not convert in detail; the match has no wildcard, so a new kind does not compile until it is classified.
fn feature(kind: NodeKind) -> Feature {
    match kind {
        NodeKind::ClassDef => planned("classes", "0.2", RuleId::CLS_01),
        NodeKind::Attribute => planned("attributes and methods", "0.2", RuleId::CLS_01),
        NodeKind::List => planned("lists", "0.2", RuleId::DAT_07),
        NodeKind::Tuple => planned("tuples", "0.2", RuleId::DAT_08),
        NodeKind::Dict => planned("dicts", "0.2", RuleId::DAT_09),
        NodeKind::Set => planned("sets", "0.2", RuleId::DAT_10),
        NodeKind::Slice => planned("slices", "0.2", RuleId::DAT_12),
        NodeKind::ListComp | NodeKind::SetComp | NodeKind::DictComp => {
            planned("comprehensions", "0.2", RuleId::EXP_12)
        }
        NodeKind::JoinedStr => planned("f-strings", "0.2", RuleId::LEX_07),
        NodeKind::BytesLiteral => planned("bytes", "0.2", RuleId::DAT_06),
        NodeKind::Delete => planned("`del` statements", "0.3", RuleId::SMT_05),
        NodeKind::Raise => planned("`raise` statements", "0.3", RuleId::SMT_06),
        NodeKind::Try => planned("`try` statements", "0.3", RuleId::CST_04),
        NodeKind::With => planned("`with` statements", "0.3", RuleId::CST_05),
        NodeKind::Assert => planned("`assert` statements", "0.3", RuleId::SMT_04),
        NodeKind::Global => planned("`global` statements", "0.3", RuleId::SMT_08),
        NodeKind::TypeAlias => planned("`type` statements", "0.4", RuleId::SMT_10),
        NodeKind::Match => planned("`match` statements", "0.5", RuleId::CST_06),
        NodeKind::NamedExpr => planned("assignment expressions", "0.5", RuleId::EXP_13),
        NodeKind::Nonlocal => planned("`nonlocal` statements", "1.0", RuleId::SMT_09),
        NodeKind::Lambda => planned("`lambda` expressions", "1.0", RuleId::EXP_14),
        NodeKind::GeneratorExp => planned("generator expressions", "1.0", RuleId::EXP_15),
        NodeKind::Yield | NodeKind::YieldFrom => planned("generators", "1.0", RuleId::SMT_11),
        NodeKind::Starred => planned("unpacking with `*`", "1.0", RuleId::EXP_17),
        NodeKind::AsyncFunctionDef | NodeKind::AsyncFor | NodeKind::AsyncWith | NodeKind::Await => {
            Feature::AfterOne {
                what: "`async` and `await`",
            }
        }
        NodeKind::TryStar => Feature::AfterOne {
            what: "`except*` clauses",
        },
        NodeKind::TemplateStr => Feature::Excluded {
            what: "template strings",
        },
        NodeKind::IpyEscapeCommand => Feature::Excluded {
            what: "IPython escape commands",
        },
    }
}

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
}

struct Walker {
    diagnostics: Diagnostics,
    loop_depth: usize,
}

impl Walker {
    fn report(&mut self, span: Span, feature: Feature) {
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
    fn syntax_error(&mut self, span: Span, message: &str) {
        self.diagnostics.push(
            Diagnostic::new(DiagnosticCode::error(103), span, message).with_rule(RuleId::STD_01),
        );
    }

    fn statements(&mut self, statements: &[Stmt], place: Place) {
        for statement in statements {
            self.statement(statement, place);
        }
    }

    fn block(&mut self, statements: &[Stmt], place: Place) {
        let inner = if place == Place::ModuleTop {
            Place::ModuleBlock
        } else {
            place
        };
        self.statements(statements, inner);
    }

    fn name_target(&mut self, target: &Expr) {
        match target {
            Expr::Name { .. } => {}
            Expr::Subscript { span, .. } => {
                self.report(
                    *span,
                    planned("assignments to items", "0.2", RuleId::DAT_07),
                );
            }
            Expr::Other {
                kind: NodeKind::Tuple | NodeKind::List,
                span,
            } => {
                self.report(
                    *span,
                    planned("unpacking assignments", "0.2", RuleId::SMT_02),
                );
            }
            Expr::Other {
                kind: NodeKind::Attribute,
                span,
            } => {
                self.report(
                    *span,
                    planned("attributes and methods", "0.2", RuleId::CLS_01),
                );
            }
            other => self.expression(other),
        }
    }

    #[allow(clippy::too_many_lines, reason = "one arm per statement kind")]
    fn statement(&mut self, statement: &Stmt, place: Place) {
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
                    Place::ModuleTop => {}
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
                self.statements(body, Place::Function);
                self.loop_depth = outer_loops;
            }
            Stmt::Return { value, span } => {
                if place != Place::Function {
                    self.syntax_error(*span, "`return` outside a function");
                }
                if let Some(value) = value {
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
                match iter {
                    Expr::Call {
                        function,
                        arguments,
                        keywords,
                        ..
                    } if matches!(function.as_ref(), Expr::Name { id, .. } if id == "range")
                        && keywords.is_empty()
                        && (1..=3).contains(&arguments.len()) =>
                    {
                        for argument in arguments {
                            self.expression(argument);
                        }
                    }
                    other => self.report(
                        other.span(),
                        planned(
                            "`for` loops over values other than `range`",
                            "0.2",
                            RuleId::CST_03,
                        ),
                    ),
                }
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
            Stmt::Import { span, .. } | Stmt::ImportFrom { span, .. } => {
                self.report(*span, planned("imports", "0.3", RuleId::SMT_07));
            }
            Stmt::Expr { value, .. } => self.expression(value),
            Stmt::Break { span } if self.loop_depth == 0 => {
                self.syntax_error(*span, "`break` outside a loop");
            }
            Stmt::Continue { span } if self.loop_depth == 0 => {
                self.syntax_error(*span, "`continue` outside a loop");
            }
            Stmt::Pass { .. } | Stmt::Break { .. } | Stmt::Continue { .. } => {}
            Stmt::Other { kind, span } => self.report(*span, feature(*kind)),
        }
    }

    fn loop_body(&mut self, body: &[Stmt], place: Place) {
        self.loop_depth += 1;
        self.block(body, place);
        self.loop_depth -= 1;
    }

    /// Annotations in 0.1 name `int`, `float`, `bool`, `str` or `None`; the checker reports any other name.
    fn annotation(&mut self, annotation: &Expr) {
        match annotation {
            Expr::Subscript { span, .. } => {
                self.report(
                    *span,
                    planned(
                        "generic annotations such as `list[int]`",
                        "0.2",
                        RuleId::TYP_06,
                    ),
                );
            }
            Expr::BinOp {
                operator: BinaryOperator::BitOr,
                span,
                ..
            } => {
                self.report(
                    *span,
                    planned(
                        "union annotations such as `int | None`",
                        "0.2",
                        RuleId::TYP_06,
                    ),
                );
            }
            Expr::Constant {
                value: Constant::Str(_),
                span,
            } => {
                self.report(
                    *span,
                    Feature::Excluded {
                        what: "annotations written as strings",
                    },
                );
            }
            other => self.expression(other),
        }
    }

    #[allow(clippy::too_many_lines, reason = "one arm per expression kind")]
    fn expression(&mut self, expression: &Expr) {
        match expression {
            Expr::BoolOp { values, .. } => {
                for value in values {
                    self.expression(value);
                }
            }
            Expr::BinOp {
                left,
                operator,
                right,
                span,
            } => {
                if *operator == BinaryOperator::MatMult {
                    self.report(
                        *span,
                        Feature::Excluded {
                            what: "the `@` operator",
                        },
                    );
                }
                self.expression(left);
                self.expression(right);
            }
            Expr::UnaryOp { operand, .. } => self.expression(operand),
            Expr::IfExp {
                test, body, orelse, ..
            } => {
                self.expression(test);
                self.expression(body);
                self.expression(orelse);
            }
            Expr::Compare {
                left,
                operators,
                comparators,
                span,
            } => {
                if operators.iter().any(|operator| {
                    matches!(operator, CompareOperator::In | CompareOperator::NotIn)
                }) {
                    self.report(*span, planned("`in` and `not in`", "0.2", RuleId::EXP_07));
                }
                self.expression(left);
                for comparator in comparators {
                    self.expression(comparator);
                }
            }
            Expr::Call {
                function,
                arguments,
                keywords,
                ..
            } => {
                match function.as_ref() {
                    Expr::Name { .. } => {}
                    Expr::Other {
                        kind: NodeKind::Attribute,
                        span,
                    } => {
                        self.report(
                            *span,
                            planned("attributes and methods", "0.2", RuleId::CLS_01),
                        );
                    }
                    other => self.report(
                        other.span(),
                        planned("calls of values that are not names", "1.0", RuleId::FUN_05),
                    ),
                }
                for argument in arguments {
                    self.expression(argument);
                }
                for keyword in keywords {
                    if keyword.name.is_none() {
                        self.report(
                            keyword.span,
                            planned("unpacking with `**`", "1.0", RuleId::EXP_17),
                        );
                    }
                    self.expression(&keyword.value);
                }
            }
            Expr::Constant { value, span } => match value {
                Constant::Complex { .. } => self.diagnostics.push(
                    Diagnostic::new(
                        DiagnosticCode::error(104),
                        *span,
                        "imaginary literals are not supported",
                    )
                    .with_rule(RuleId::LEX_08),
                ),
                Constant::Bytes(_) => self.report(*span, planned("bytes", "0.2", RuleId::DAT_06)),
                Constant::Ellipsis => self.report(
                    *span,
                    Feature::Excluded {
                        what: "`...` as a value",
                    },
                ),
                Constant::None
                | Constant::Bool(_)
                | Constant::Int(_)
                | Constant::Float(_)
                | Constant::Str(_) => {}
            },
            Expr::Subscript { value, index, .. } => {
                self.expression(value);
                self.expression(index);
            }
            Expr::Name { .. } => {}
            Expr::Other { kind, span } => self.report(*span, feature(*kind)),
        }
    }
}

#[cfg(test)]
mod tests;
