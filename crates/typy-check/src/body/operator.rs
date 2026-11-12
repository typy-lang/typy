//! The types of operators, with Python's results (EXP-02 to EXP-09).

use typy_diagnostics::{Diagnostic, DiagnosticCode, RuleId, Span};
use typy_source::syntax::{BinaryOperator, CompareOperator, UnaryOperator};

use super::BodyChecker;
use super::view_operator::is_set_view;
use crate::typed::{TypedConstant, TypedExpr, TypedExprKind};
use crate::types::Type;

impl BodyChecker<'_> {
    /// Returns true when `left ** right` with a float exponent cannot be complex in CPython: the exponent is an integral literal, or the base is a non-negative literal (EXP-04).
    fn float_power_is_real(left: &TypedExpr, right: &TypedExpr) -> bool {
        let integral_exponent = matches!(&right.kind, TypedExprKind::Constant(TypedConstant::Float(value)) if value.fract() == 0.0);
        // A literal is never negative: `-1.5` is the negation of the literal `1.5`, which is an
        // operation, so a constant base is a non-negative literal.
        let literal_base = matches!(
            &left.kind,
            TypedExprKind::Constant(TypedConstant::Int(_) | TypedConstant::Float(_))
        );
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
        let (l, r) = (left.ty.clone(), right.ty.clone());
        if l == Error || r == Error {
            return Error;
        }
        let symbol = binary_symbol(operator);
        if matches!(operator, Op::BitOr | Op::BitAnd | Op::Sub | Op::BitXor)
            && (is_set_view(&l) || is_set_view(&r))
        {
            return self.view_operation(symbol, &l, &r, span);
        }
        if let (Op::Add, Type::Tuple(first), Type::Tuple(second)) = (operator, &l, &r) {
            return Type::Tuple(first.iter().chain(second).cloned().collect());
        }
        match operator {
            Op::BitAnd | Op::BitOr | Op::BitXor if l == Bool && r == Bool => Bool,
            _ if l == Bool || r == Bool => self.bool_arithmetic(span),
            Op::Add if l == Str && r == Str => Str,
            Op::Add if l == Type::Bytes && r == Type::Bytes => Type::Bytes,
            Op::Mult if (l == Type::Bytes && r == Int) || (l == Int && r == Type::Bytes) => {
                Type::Bytes
            }
            // A list joins only a list of its own type, because the runtime stores each item by
            // the kind of the item type (DAT-07).
            Op::Add if l.sequence_item().is_some() && l == r => l,
            // The runtime combines two sets by the item type of the first (DAT-10).
            Op::BitOr | Op::BitAnd | Op::Sub | Op::BitXor
                if matches!(l, Type::Set(_)) && l == r =>
            {
                l
            }
            // `|` merges two dicts of one type into a new one (DAT-09).
            Op::BitOr if matches!(l, Type::Dict(..)) && l == r => l,
            Op::Mult if l.sequence_item().is_some() && r == Int => l,
            Op::Mult if l == Int && r.sequence_item().is_some() => r,
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
                        "formatting with `%` a format that is not a string literal is planned \
                         for TyPy 0.4 (LIB-11)",
                    )
                    .with_rule(RuleId::LIB_11)
                    .with_help("write the format as a literal, or write an f-string"),
                );
                Error
            }
            _ => self.operator_error(symbol, &l, &r, span),
        }
    }

    pub(super) fn unary_type(
        &mut self,
        operator: UnaryOperator,
        operand: &TypedExpr,
        span: Span,
    ) -> Type {
        let ty = operand.ty.clone();
        match operator {
            _ if ty == Type::Error => Type::Error,
            UnaryOperator::Not => {
                self.check_truth(operand);
                Type::Bool
            }
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
        let (l, r) = (left.ty.clone(), right.ty.clone());
        if l == Type::Error || r == Type::Error {
            return;
        }
        let span = right.span;
        let compares = !matches!(
            operator,
            CompareOperator::In
                | CompareOperator::NotIn
                | CompareOperator::Is
                | CompareOperator::IsNot
        );
        if compares && (matches!(l, Type::DictView(..)) || matches!(r, Type::DictView(..))) {
            let equality = matches!(operator, CompareOperator::Eq | CompareOperator::NotEq);
            if !Self::view_comparison(&l, &r, equality) {
                self.comparison_error(&l, &r, span);
            }
            return;
        }
        match operator {
            CompareOperator::Is | CompareOperator::IsNot => {
                // Two instances, or optional instances, are the same object or not. `True`,
                // `False` and `None` are each the only object of its value, so `bool` and
                // `bool | None` values compare by identity too (EXP-08).
                let instance = |ty: &Type| match ty {
                    Type::Optional(inner) => matches!(
                        inner.as_ref(),
                        Type::Instance(..)
                            | Type::Exception(_)
                            | Type::Iterator(_)
                            | Type::TypeObject(_)
                            | Type::Module(_)
                            | Type::AnyModule
                            | Type::TextFile
                            | Type::Traceback
                    ),
                    other => matches!(
                        other,
                        Type::Instance(..)
                            | Type::Exception(_)
                            | Type::Iterator(_)
                            | Type::Traceback
                            | Type::Module(_)
                            | Type::AnyModule
                            | Type::TextFile
                            | Type::TypeObject(_)
                    ),
                };
                let singleton =
                    |ty: &Type| *ty == Type::Bool || *ty == Type::Optional(Box::new(Type::Bool));
                let booleans = singleton(&l) && singleton(&r);
                if l != Type::NoneType
                    && r != Type::NoneType
                    && !booleans
                    && !(instance(&l) && instance(&r))
                {
                    self.diagnostics.push(
                        Diagnostic::new(
                            DiagnosticCode::error(301),
                            span,
                            format!("`is` compares identity, which is defined only against None, between bools or between objects here; these are {l} and {r}"),
                        )
                        .with_rule(RuleId::EXP_08)
                        .with_help("use `==` to compare values"),
                    );
                }
            }
            CompareOperator::Eq | CompareOperator::NotEq => {
                if !l.can_test_equality(&r) {
                    self.comparison_error(&l, &r, span);
                }
            }
            CompareOperator::Lt
            | CompareOperator::LtE
            | CompareOperator::Gt
            | CompareOperator::GtE => {
                if !l.can_order(&r) {
                    self.comparison_error(&l, &r, span);
                }
            }
            // `x in xs` tests each item with `==`, so `x` compares for equality with the items
            // (EXP-07).
            CompareOperator::In | CompareOperator::NotIn => match &r {
                // The runtime compares the value with each item by the item's kind, so the value
                // has the item type.
                Type::List(item) | Type::TupleOf(item) | Type::Set(item) | Type::Dict(item, _)
                    if l.is_assignable_to(item) => {}
                // A view finds a key, a value, or a `(key, value)` pair.
                Type::DictView(view, key, value) if l.is_assignable_to(&view.item(key, value)) => {}
                // A substring test: `in` on a `str` needs a `str` on its left.
                Type::Str if l == Type::Str => {}
                // `in` on a range tests an `int` without stepping (DAT-17).
                Type::Range if l == Type::Int => {}
                // `in` on `bytes` looks for a byte, an `int`, or for `bytes` within them.
                Type::Bytes if matches!(l, Type::Int | Type::Bytes) => {}
                // A tuple compares the value with each item for equality, so each item has to
                // compare with it.
                Type::Tuple(items) if items.iter().all(|item| l.can_test_equality(item)) => {}
                other => self.diagnostics.push(
                    Diagnostic::new(
                        DiagnosticCode::error(407),
                        span,
                        format!(
                            "`in` looks for {} in {}, which cannot hold it",
                            l.with_article(),
                            other.with_article()
                        ),
                    )
                    .with_rule(RuleId::EXP_07),
                ),
            },
        }
    }

    pub(super) fn operator_error(
        &mut self,
        symbol: &str,
        left: &Type,
        right: &Type,
        span: Span,
    ) -> Type {
        let diagnostic = Diagnostic::new(
            DiagnosticCode::error(401),
            span,
            format!("`{symbol}` is not defined for {left} and {right}"),
        )
        .with_rule(RuleId::EXP_02);
        self.diagnostics
            .push(with_narrowing_help(diagnostic, left, right));
        Type::Error
    }

    /// Reports arithmetic on `bool` values, which TyPy refuses (DAT-03).
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

    pub(super) fn comparison_error(&mut self, left: &Type, right: &Type, span: Span) {
        let diagnostic = Diagnostic::new(
            DiagnosticCode::error(407),
            span,
            format!("{left} and {right} cannot be compared"),
        )
        .with_rule(RuleId::EXP_06);
        self.diagnostics
            .push(with_narrowing_help(diagnostic, left, right));
    }
}

/// Adds to an error about an optional operand how to use the value it holds (TYP-13).
fn with_narrowing_help(diagnostic: Diagnostic, left: &Type, right: &Type) -> Diagnostic {
    if matches!(left, Type::Optional(_)) || matches!(right, Type::Optional(_)) {
        diagnostic.with_help(
            "an optional value is used after a test shows it is not None, such as `if value is not None:`",
        )
    } else {
        diagnostic
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
