//! The types of operators, with Python's results (EXP-02 to EXP-09).

use typy_diagnostics::{Diagnostic, DiagnosticCode, RuleId, Span};
use typy_source::syntax::{BinaryOperator, CompareOperator, UnaryOperator};

use super::BodyChecker;
use crate::typed::{TypedConstant, TypedExpr, TypedExprKind};
use crate::types::Type;

impl BodyChecker<'_> {
    /// Returns true when `left ** right` with a float exponent cannot be complex in CPython: the exponent is an integral literal, or the base is a non-negative literal (EXP-04).
    fn float_power_is_real(left: &TypedExpr, right: &TypedExpr) -> bool {
        let integral_exponent = matches!(&right.kind, TypedExprKind::Constant(TypedConstant::Float(value)) if value.fract() == 0.0);
        let literal_base = matches!(&left.kind, TypedExprKind::Constant(TypedConstant::Int(_)))
            || matches!(&left.kind, TypedExprKind::Constant(TypedConstant::Float(value)) if *value >= 0.0);
        integral_exponent || literal_base
    }

    /// Returns the type of a binary operation, with Python's results (EXP-02 to EXP-05).
    pub(super) fn binary_type(
        &mut self,
        operator: BinaryOperator,
        left: &TypedExpr,
        right: &TypedExpr,
        span: Span,
    ) -> Type {
        use BinaryOperator as Op;
        use Type::{Bool, Error, Float, Int, Str};
        let (l, r) = (left.ty, right.ty);
        if l == Error || r == Error {
            return Error;
        }
        let symbol = binary_symbol(operator);
        match operator {
            Op::BitAnd | Op::BitOr | Op::BitXor if l == Bool && r == Bool => Bool,
            _ if l == Bool || r == Bool => self.bool_arithmetic(span),
            Op::Add if l == Str && r == Str => Str,
            Op::Mult if (l == Str && r == Int) || (l == Int && r == Str) => Str,
            Op::Add | Op::Sub | Op::Mult | Op::FloorDiv | Op::Mod
                if l.is_numeric() && r.is_numeric() =>
            {
                if l == Int && r == Int {
                    Int
                } else {
                    Float
                }
            }
            Op::Div if l.is_numeric() && r.is_numeric() => Float,
            Op::Pow if l.is_numeric() && r.is_numeric() => {
                if l == Int && r == Int {
                    if !matches!(&right.kind, TypedExprKind::Constant(TypedConstant::Int(_))) {
                        self.diagnostics.push(
                            Diagnostic::new(
                                DiagnosticCode::error(302),
                                right.span,
                                "the exponent of `int ** int` must be a non-negative integer literal, because CPython returns a float for a negative exponent",
                            )
                            .with_rule(RuleId::EXP_04)
                            .with_help("write the base as a float, such as `2.0 ** n`"),
                        );
                        return Error;
                    }
                    Int
                } else if r == Float && !Self::float_power_is_real(left, right) {
                    self.diagnostics.push(
                        Diagnostic::new(
                            DiagnosticCode::error(302),
                            right.span,
                            "a float exponent needs an integral literal or a non-negative literal base, because CPython returns a complex number for a negative base",
                        )
                        .with_rule(RuleId::EXP_04),
                    );
                    Error
                } else {
                    Float
                }
            }
            Op::LShift | Op::RShift | Op::BitAnd | Op::BitOr | Op::BitXor
                if l == Int && r == Int =>
            {
                Int
            }
            Op::Mod if l == Str => {
                self.diagnostics.push(
                    Diagnostic::new(
                        DiagnosticCode::error(201),
                        span,
                        "formatting strings with `%` is planned for TyPy 0.2 (LIB-03)",
                    )
                    .with_rule(RuleId::LIB_03),
                );
                Error
            }
            _ => self.operator_error(symbol, l, r, span),
        }
    }

    pub(super) fn unary_type(
        &mut self,
        operator: UnaryOperator,
        operand: &TypedExpr,
        span: Span,
    ) -> Type {
        let ty = operand.ty;
        match operator {
            _ if ty == Type::Error => Type::Error,
            UnaryOperator::Not => Type::Bool,
            _ if ty == Type::Bool => self.bool_arithmetic(span),
            UnaryOperator::USub | UnaryOperator::UAdd if ty.is_numeric() => ty,
            UnaryOperator::Invert if ty == Type::Int => Type::Int,
            _ => {
                let symbol = match operator {
                    UnaryOperator::USub => "-",
                    UnaryOperator::UAdd => "+",
                    UnaryOperator::Invert | UnaryOperator::Not => "~",
                };
                self.diagnostics.push(
                    Diagnostic::new(
                        DiagnosticCode::error(401),
                        span,
                        format!("unary `{symbol}` is not defined for {ty}"),
                    )
                    .with_rule(RuleId::EXP_02),
                );
                Type::Error
            }
        }
    }

    /// Checks one link of a comparison chain (EXP-06, EXP-08).
    pub(super) fn compare_types(
        &mut self,
        operator: CompareOperator,
        left: &TypedExpr,
        right: &TypedExpr,
    ) {
        let (l, r) = (left.ty, right.ty);
        if l == Type::Error || r == Type::Error {
            return;
        }
        let span = right.span;
        match operator {
            CompareOperator::Is | CompareOperator::IsNot => {
                if l != Type::NoneType && r != Type::NoneType {
                    self.diagnostics.push(
                        Diagnostic::new(
                            DiagnosticCode::error(301),
                            span,
                            format!("`is` compares identity, which is defined only against None here; these are {l} and {r}"),
                        )
                        .with_rule(RuleId::EXP_08)
                        .with_help("use `==` to compare values"),
                    );
                }
            }
            CompareOperator::Eq | CompareOperator::NotEq => {
                if !(l == r || (l.is_numeric() && r.is_numeric())) {
                    self.comparison_error(l, r, span);
                }
            }
            CompareOperator::Lt
            | CompareOperator::LtE
            | CompareOperator::Gt
            | CompareOperator::GtE => {
                let ordered = (l == r && l != Type::NoneType) || (l.is_numeric() && r.is_numeric());
                if !ordered {
                    self.comparison_error(l, r, span);
                }
            }
            CompareOperator::In | CompareOperator::NotIn => {}
        }
    }

    pub(super) fn operator_error(
        &mut self,
        symbol: &str,
        left: Type,
        right: Type,
        span: Span,
    ) -> Type {
        self.diagnostics.push(
            Diagnostic::new(
                DiagnosticCode::error(401),
                span,
                format!("`{symbol}` is not defined for {left} and {right}"),
            )
            .with_rule(RuleId::EXP_02),
        );
        Type::Error
    }

    pub(super) fn bool_arithmetic(&mut self, span: Span) -> Type {
        self.diagnostics.push(
            Diagnostic::new(
                DiagnosticCode::error(406),
                span,
                "arithmetic on bool values is not supported",
            )
            .with_rule(RuleId::DAT_03)
            .with_help("convert with `int(...)` first"),
        );
        Type::Error
    }

    pub(super) fn comparison_error(&mut self, left: Type, right: Type, span: Span) {
        self.diagnostics.push(
            Diagnostic::new(
                DiagnosticCode::error(407),
                span,
                format!("{left} and {right} cannot be compared"),
            )
            .with_rule(RuleId::EXP_06),
        );
    }
}

fn binary_symbol(operator: BinaryOperator) -> &'static str {
    match operator {
        BinaryOperator::Add => "+",
        BinaryOperator::Sub => "-",
        BinaryOperator::Mult => "*",
        BinaryOperator::MatMult => "@",
        BinaryOperator::Div => "/",
        BinaryOperator::Mod => "%",
        BinaryOperator::Pow => "**",
        BinaryOperator::LShift => "<<",
        BinaryOperator::RShift => ">>",
        BinaryOperator::BitOr => "|",
        BinaryOperator::BitXor => "^",
        BinaryOperator::BitAnd => "&",
        BinaryOperator::FloorDiv => "//",
    }
}
