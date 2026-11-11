//! Checks expressions and annotations against the feature table.

use typy_diagnostics::{Diagnostic, DiagnosticCode, RuleId};
use typy_source::syntax::{BinaryOperator, Comprehension, Constant, Expr, NodeKind};

use super::Walker;
use super::feature::{Feature, feature, planned};

impl Walker {
    /// Checks the fields of an f-string and of its format specifications.
    fn fstring(&mut self, parts: &[typy_source::syntax::FStringPart]) {
        for part in parts {
            if let typy_source::syntax::FStringPart::Field(field) = part {
                self.expression(&field.value);
                if let Some(spec) = &field.spec {
                    self.fstring(spec);
                }
            }
        }
    }

    /// Checks each clause of a comprehension: its target, its source and its conditions; an
    /// `async for` clause is refused as `async` is.
    fn generators(&mut self, generators: &[Comprehension]) {
        for generator in generators {
            if generator.is_async {
                self.report(generator.span, feature(NodeKind::AsyncFor));
            }
            self.name_target(&generator.target);
            self.expression(&generator.iter);
            for condition in &generator.ifs {
                self.expression(condition);
            }
        }
    }

    /// Annotations name `int`, `float`, `bool`, `str`, `None`, `tuple[...]`, `list[...]`,
    /// `set[...]` or `dict[...]`; the checker reports any other name.
    pub(super) fn annotation(&mut self, annotation: &Expr) {
        match annotation {
            Expr::Subscript { value, index, .. } if matches!(value.as_ref(), Expr::Name { id, .. } if matches!(id.as_str(), "tuple" | "list" | "set" | "dict" | "type")) =>
            {
                match index.as_ref() {
                    // `tuple[int, ...]` is checked, and reported, by the checker.
                    Expr::Tuple { elements, .. } => {
                        for element in elements {
                            if !matches!(
                                element,
                                Expr::Constant {
                                    value: Constant::Ellipsis,
                                    ..
                                }
                            ) {
                                self.annotation(element);
                            }
                        }
                    }
                    Expr::Constant {
                        value: Constant::Ellipsis,
                        ..
                    } => {}
                    single => self.annotation(single),
                }
            }
            // Only the generic types of TYP-06 and CLS-12 are provided.
            Expr::Subscript { span, .. } => {
                self.report(
                    *span,
                    Feature::Excluded {
                        what: "generic annotations other than `list`, `dict`, `set`, `tuple` and `type`, such as `frozenset[int]`,",
                    },
                );
            }
            Expr::BinOp {
                operator: BinaryOperator::BitOr,
                left,
                right,
                span,
            } => {
                let is_none = |side: &Expr| {
                    matches!(
                        side,
                        Expr::Constant {
                            value: Constant::None,
                            ..
                        }
                    )
                };
                // `T | None` and `None | T` are optional types (TYP-06); any other union waits
                // for `isinstance` narrowing.
                match (is_none(left), is_none(right)) {
                    (false, true) => self.annotation(left),
                    (true, false) => self.annotation(right),
                    _ => self.report(
                        *span,
                        planned(
                            "union annotations such as `int | str`",
                            "0.4",
                            RuleId::TYP_14,
                        ),
                    ),
                }
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
    pub(super) fn expression(&mut self, expression: &Expr) {
        if let Expr::Name { id, .. } = expression {
            self.used_name(id);
        }
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
                left, comparators, ..
            } => {
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
                    // A method call: the checker decides whether the value has the method.
                    Expr::Attribute { value, .. } => self.expression(value),
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
                | Constant::Str(_)
                | Constant::Bytes(_) => {}
            },
            Expr::Subscript { value, index, .. } => {
                self.expression(value);
                self.expression(index);
            }
            Expr::Name { .. } => {}
            Expr::Slice {
                lower, upper, step, ..
            } => {
                for bound in [lower, upper, step].into_iter().flatten() {
                    self.expression(bound);
                }
            }
            Expr::Tuple { elements, .. }
            | Expr::List { elements, .. }
            | Expr::Set { elements, .. } => {
                for element in elements {
                    self.expression(element);
                }
            }
            Expr::ListComp {
                element,
                generators,
                ..
            }
            | Expr::SetComp {
                element,
                generators,
                ..
            } => {
                self.expression(element);
                self.generators(generators);
            }
            Expr::DictComp {
                key,
                value,
                generators,
                ..
            } => {
                self.expression(key);
                self.expression(value);
                self.generators(generators);
            }
            Expr::Dict { keys, values, .. } => {
                for (key, value) in keys.iter().zip(values) {
                    match key {
                        Some(key) => self.expression(key),
                        None => self.report(
                            value.span(),
                            planned("unpacking with `**`", "1.0", RuleId::EXP_17),
                        ),
                    }
                    self.expression(value);
                }
            }
            Expr::Attribute { value, .. } => self.expression(value),
            Expr::Quoted { annotation, .. } => self.annotation(annotation),
            Expr::FString { parts, .. } => self.fstring(parts),
            Expr::Other { kind, span } => self.report(*span, feature(*kind)),
        }
    }
}
