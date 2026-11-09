//! Checks expressions and gives each its type.

use typy_diagnostics::{Diagnostic, DiagnosticCode, RuleId};
use typy_source::syntax::{BoolOperator, Constant, Expr};

use super::BodyChecker;
use super::slice::Bounds;
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
                let ty = values[0].ty.clone();
                if let Some(other) = values.iter().find(|value| !value.ty.is_assignable_to(&ty)) {
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
                let mut operands = self.comparison_operands(left, comparators, operators);
                let comparators = operands.split_off(1);
                let left = operands.remove(0);
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
                if !orelse.ty.is_assignable_to(&body.ty) {
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
                let ty = body.ty.clone();
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
            Expr::Tuple { elements, .. } => {
                let elements: Vec<TypedExpr> = elements
                    .iter()
                    .map(|element| self.expression(element))
                    .collect();
                let ty = Type::Tuple(elements.iter().map(|element| element.ty.clone()).collect());
                Self::typed(TypedExprKind::Tuple(elements), ty, span)
            }
            Expr::Subscript { value, index, .. } => {
                let value = self.expression(value);
                if let Expr::Slice {
                    lower, upper, step, ..
                } = index.as_ref()
                {
                    let bounds = Bounds {
                        lower: lower.as_deref(),
                        upper: upper.as_deref(),
                        step: step.as_deref(),
                    };
                    return self.slice(value, &bounds, span);
                }
                if let Type::Tuple(items) = &value.ty {
                    let items = items.clone();
                    return self.tuple_item(value, &items, index, span);
                }
                if let Some(item) = value.ty.sequence_item() {
                    let item = item.clone();
                    return self.list_item(value, item, index, span);
                }
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
            } => match function.as_ref() {
                Expr::Name {
                    id,
                    span: name_span,
                } => self.call(id, *name_span, arguments, keywords, span),
                Expr::Attribute {
                    value, attribute, ..
                } => self.method_call(value, attribute, arguments, keywords, span),
                _ => Self::typed(
                    TypedExprKind::Constant(TypedConstant::None),
                    Type::Error,
                    span,
                ),
            },
            Expr::List { elements, .. } => self.list_display(elements, None, span),
            Expr::Slice { .. } => self.misplaced_slice(span),
            // An attribute that is not a method call is refused by the subset check until classes.
            Expr::Attribute { .. } | Expr::Other { .. } => Self::typed(
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

    /// Checks `tuple[index]`: the position is an integer literal, counted from the end when
    /// negative, inside the tuple, so the item and its type are known when compiling (DAT-08).
    fn tuple_item(
        &mut self,
        tuple: TypedExpr,
        items: &[Type],
        index: &Expr,
        span: typy_diagnostics::Span,
    ) -> TypedExpr {
        let Some(position) = literal_position(index) else {
            let index = self.expression(index);
            if index.ty.has_error() || tuple.ty.has_error() {
                return Self::typed(
                    TypedExprKind::Constant(TypedConstant::None),
                    Type::Error,
                    span,
                );
            }
            return self.error(
                Diagnostic::new(
                    DiagnosticCode::error(307),
                    index.span,
                    "a tuple is indexed with an integer literal, because each item can have its own type",
                )
                .with_rule(RuleId::DAT_08)
                .with_help("write the position as a number, such as `pair[0]`"),
                span,
            );
        };
        let length = i128::try_from(items.len()).unwrap_or(i128::MAX);
        let from_start = if position < 0 {
            position + length
        } else {
            position
        };
        let Some(index_from_start) = usize::try_from(from_start)
            .ok()
            .filter(|index| *index < items.len())
        else {
            return self.error(
                Diagnostic::new(
                    DiagnosticCode::error(307),
                    index.span(),
                    format!(
                        "position {position} is outside {}, which has {} items",
                        tuple.ty,
                        items.len()
                    ),
                )
                .with_rule(RuleId::DAT_08),
                span,
            );
        };
        let ty = items[index_from_start].clone();
        Self::typed(
            TypedExprKind::TupleItem {
                tuple: Box::new(tuple),
                index: index_from_start,
            },
            ty,
            span,
        )
    }

    /// Reports a value whose type is not `expected`.
    pub(super) fn expect(&mut self, value: &TypedExpr, expected: Type, what: &str) {
        // A type that holds an error was already reported where the error arose.
        if value.ty.has_error() || value.ty.is_assignable_to(&expected) {
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

/// Returns the value of an integer literal, or of a negated one, such as `2` or `-1`.
pub(super) fn literal_position(index: &Expr) -> Option<i128> {
    use typy_source::syntax::UnaryOperator;
    match index {
        Expr::Constant {
            value: Constant::Int(digits),
            ..
        } => digits.parse().ok().or(Some(i128::MAX)),
        Expr::UnaryOp {
            operator: UnaryOperator::USub,
            operand,
            ..
        } => literal_position(operand).map(|value| -value),
        _ => None,
    }
}
