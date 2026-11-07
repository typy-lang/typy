//! Checks expressions and gives each its type.

use typy_diagnostics::{Diagnostic, DiagnosticCode, RuleId};
use typy_source::syntax::{BoolOperator, Constant, Expr};

use super::BodyChecker;
use crate::typed::{TypedConstant, TypedExpr, TypedExprKind};
use crate::types::Type;

impl BodyChecker<'_> {
    #[allow(clippy::too_many_lines, reason = "one arm per expression kind")]
    pub(super) fn expression(&mut self, expression: &Expr) -> TypedExpr {
        let span = expression.span();
        match expression {
            Expr::Constant { value, .. } => {
                let constant = match value {
                    Constant::None => TypedConstant::None,
                    Constant::Bool(value) => TypedConstant::Bool(*value),
                    Constant::Int(digits) => TypedConstant::Int(digits.clone()),
                    Constant::Float(value) => TypedConstant::Float(*value),
                    Constant::Str(value) => TypedConstant::Str(value.clone()),
                    Constant::Complex { .. } | Constant::Bytes(_) | Constant::Ellipsis => {
                        return Self::typed(
                            TypedExprKind::Constant(TypedConstant::None),
                            Type::Error,
                            span,
                        );
                    }
                };
                let ty = constant.ty();
                Self::typed(TypedExprKind::Constant(constant), ty, span)
            }
            Expr::Name { id, .. } => self.name(id, span),
            Expr::BinOp {
                left,
                operator,
                right,
                ..
            } => {
                let left = self.expression(left);
                let right = self.expression(right);
                let ty = self.binary_type(*operator, &left, &right, span);
                Self::typed(
                    TypedExprKind::Binary {
                        left: Box::new(left),
                        operator: *operator,
                        right: Box::new(right),
                    },
                    ty,
                    span,
                )
            }
            Expr::UnaryOp {
                operator, operand, ..
            } => {
                let operand = self.expression(operand);
                let ty = self.unary_type(*operator, &operand, span);
                Self::typed(
                    TypedExprKind::Unary {
                        operator: *operator,
                        operand: Box::new(operand),
                    },
                    ty,
                    span,
                )
            }
            Expr::BoolOp {
                operator, values, ..
            } => {
                let values: Vec<TypedExpr> =
                    values.iter().map(|value| self.truth_test(value)).collect();
                let ty = values[0].ty;
                if let Some(other) = values.iter().find(|value| !value.ty.is_assignable_to(ty)) {
                    let word = if *operator == BoolOperator::And {
                        "and"
                    } else {
                        "or"
                    };
                    return self.error(
                        Diagnostic::new(
                            DiagnosticCode::error(303),
                            other.span,
                            format!(
                                "the operands of `{word}` must have one type; they are {ty} and {}",
                                other.ty
                            ),
                        )
                        .with_rule(RuleId::EXP_09),
                        span,
                    );
                }
                Self::typed(
                    TypedExprKind::BoolOp {
                        operator: *operator,
                        values,
                    },
                    ty,
                    span,
                )
            }
            Expr::Compare {
                left,
                operators,
                comparators,
                ..
            } => {
                let left = self.expression(left);
                let comparators: Vec<TypedExpr> = comparators
                    .iter()
                    .map(|value| self.expression(value))
                    .collect();
                let mut previous = &left;
                for (operator, right) in operators.iter().zip(&comparators) {
                    self.compare_types(*operator, previous, right);
                    previous = right;
                }
                Self::typed(
                    TypedExprKind::Compare {
                        left: Box::new(left),
                        operators: operators.clone(),
                        comparators,
                    },
                    Type::Bool,
                    span,
                )
            }
            Expr::IfExp {
                test, body, orelse, ..
            } => {
                let test = self.truth_test(test);
                let body = self.expression(body);
                let orelse = self.expression(orelse);
                if !orelse.ty.is_assignable_to(body.ty) {
                    return self.error(
                        Diagnostic::new(
                            DiagnosticCode::error(305),
                            orelse.span,
                            format!(
                                "both branches must have one type; they are {} and {}",
                                body.ty, orelse.ty
                            ),
                        )
                        .with_rule(RuleId::EXP_11),
                        span,
                    );
                }
                let ty = body.ty;
                Self::typed(
                    TypedExprKind::IfExp {
                        test: Box::new(test),
                        body: Box::new(body),
                        orelse: Box::new(orelse),
                    },
                    ty,
                    span,
                )
            }
            Expr::Subscript { value, index, .. } => {
                let value = self.expression(value);
                let index = self.expression(index);
                if value.ty == Type::Error || index.ty == Type::Error {
                    return Self::typed(
                        TypedExprKind::Constant(TypedConstant::None),
                        Type::Error,
                        span,
                    );
                }
                if value.ty != Type::Str {
                    return self.error(
                        Diagnostic::new(
                            DiagnosticCode::error(401),
                            value.span,
                            format!("{} cannot be indexed", value.ty.with_article()),
                        )
                        .with_rule(RuleId::EXP_02),
                        span,
                    );
                }
                self.expect(&index, Type::Int, "an index");
                Self::typed(
                    TypedExprKind::Index {
                        value: Box::new(value),
                        index: Box::new(index),
                    },
                    Type::Str,
                    span,
                )
            }
            Expr::Call {
                function,
                arguments,
                keywords,
                ..
            } => {
                let Expr::Name {
                    id,
                    span: name_span,
                } = function.as_ref()
                else {
                    return Self::typed(
                        TypedExprKind::Constant(TypedConstant::None),
                        Type::Error,
                        span,
                    );
                };
                self.call(id, *name_span, arguments, keywords, span)
            }
            Expr::Other { .. } => Self::typed(
                TypedExprKind::Constant(TypedConstant::None),
                Type::Error,
                span,
            ),
        }
    }

    pub(super) fn truth_test(&mut self, test: &Expr) -> TypedExpr {
        let test = self.expression(test);
        if !test.ty.is_truth_testable() {
            self.diagnostics.push(
                Diagnostic::new(
                    DiagnosticCode::error(304),
                    test.span,
                    format!("{} cannot be used as a condition", test.ty.with_article()),
                )
                .with_rule(RuleId::EXP_10),
            );
        }
        test
    }

    /// Reports a value whose type is not `expected`.
    pub(super) fn expect(&mut self, value: &TypedExpr, expected: Type, what: &str) {
        if value.ty.is_assignable_to(expected) {
            return;
        }
        let rule = if expected == Type::Float && value.ty == Type::Int {
            RuleId::TYP_09
        } else {
            RuleId::TYP_01
        };
        let mut diagnostic = Diagnostic::new(
            DiagnosticCode::error(404),
            value.span,
            format!(
                "{what} must be {}, and this is {}",
                expected.with_article(),
                value.ty.with_article()
            ),
        )
        .with_rule(rule);
        if let (TypedExprKind::Constant(TypedConstant::Int(digits)), Type::Float) =
            (&value.kind, expected)
        {
            diagnostic = diagnostic.with_fix(value.span, format!("{digits}.0"));
        }
        self.diagnostics.push(diagnostic);
    }
}
