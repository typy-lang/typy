//! Comparisons of instances whose classes define comparison methods: `a < b` calls
//! `a.__lt__(b)`, or the reflected `b.__gt__(a)` (CLS-07, ADR-0026).

use typy_diagnostics::{Diagnostic, DiagnosticCode, RuleId, Span};
use typy_source::syntax::{BinaryOperator, CompareOperator, UnaryOperator};

use super::BodyChecker;
use crate::typed::{Argument, FunctionId, OperatorCall, TypedExpr, TypedExprKind};
use crate::types::Type;

/// The outcome of looking for a comparison method.
pub(super) enum Comparison {
    /// A call of a comparison method.
    Called(TypedExpr),
    /// No class of the operands defines the method, so the comparison is the built-in one,
    /// with the operands given back.
    Builtin(TypedExpr, TypedExpr),
}

impl BodyChecker<'_> {
    /// Returns `left op right` as a call of an arithmetic method when the class of an operand
    /// defines one, `__add__` of the left operand or `__radd__` of the right (CLS-07).
    pub(super) fn operator_binary(
        &mut self,
        operator: BinaryOperator,
        left: TypedExpr,
        right: TypedExpr,
        span: Span,
    ) -> Comparison {
        if !matches!(left.ty, Type::Instance(_)) && !matches!(right.ty, Type::Instance(_)) {
            return Comparison::Builtin(left, right);
        }
        let (name, reflected) = binary_names(operator);
        match self.operator_method_call(name, reflected, &left, &right, span) {
            Some(call) => Comparison::Called(call),
            None => Comparison::Builtin(left, right),
        }
    }

    /// Returns `op operand` as a call of `__neg__`, `__pos__` or `__invert__`, or `abs(operand)`
    /// as one of `__abs__`, when the operand's class defines the method (CLS-07).
    pub(super) fn operator_unary(
        &mut self,
        name: &str,
        operand: &TypedExpr,
        span: Span,
    ) -> Option<TypedExpr> {
        let Type::Instance(class) = &operand.ty else {
            return None;
        };
        let method = self.program.method(class.id, name)?;
        let signature = &self.program.signatures[method.0 as usize];
        let receiver = Self::converted(
            operand.clone(),
            signature.parameters.first().map(|parameter| &parameter.ty),
        );
        let returns = signature.returns.clone();
        Some(Self::typed(
            TypedExprKind::Call {
                function: method,
                arguments: vec![Argument {
                    parameter: 0,
                    value: receiver,
                }],
                dispatch: true,
            },
            returns,
            span,
        ))
    }

    /// Returns `left op right` as a call of a comparison method when the class of an operand
    /// defines one: the left operand's method when it takes the right operand, otherwise the
    /// reflected method of the right operand's class. `!=` without `__ne__` is `not` of
    /// `__eq__`.
    pub(super) fn operator_comparison(
        &mut self,
        operator: CompareOperator,
        left: TypedExpr,
        right: TypedExpr,
        span: Span,
    ) -> Comparison {
        let Some((name, reflected)) = method_names(operator) else {
            return Comparison::Builtin(left, right);
        };
        if let Some(call) = self.operator_method_call(name, reflected, &left, &right, span) {
            return Comparison::Called(call);
        }
        if operator == CompareOperator::NotEq
            && let Some(equal) = self.operator_method_call("__eq__", "__eq__", &left, &right, span)
        {
            let negated = match equal.kind {
                TypedExprKind::OperatorCall(mut call) => {
                    call.negated = true;
                    TypedExprKind::OperatorCall(call)
                }
                other => TypedExprKind::Unary {
                    operator: UnaryOperator::Not,
                    operand: Box::new(Self::typed(other, Type::Bool, span)),
                },
            };
            return Comparison::Called(Self::typed(negated, Type::Bool, span));
        }
        Comparison::Builtin(left, right)
    }

    /// Returns the call of `left.name(right)`, or of `right.reflected(left)`, when one of the
    /// classes defines the method and it takes the other operand. When the right operand's
    /// class is a subclass of the left's that defines the reflected method otherwise, CPython
    /// tries it first; the runtime decides that when the method is overridden, and the static
    /// classes decide it otherwise.
    pub(super) fn operator_method_call(
        &mut self,
        name: &str,
        reflected: &str,
        left: &TypedExpr,
        right: &TypedExpr,
        span: Span,
    ) -> Option<TypedExpr> {
        let mut forward = self.operator_method(&left.ty, name, &right.ty);
        let mut left_reflected = None;
        let mut backward = self.operator_method(&right.ty, reflected, &left.ty);
        forward.or(backward)?;
        if let (Type::Instance(left_class), Type::Instance(right_class)) = (&left.ty, &right.ty) {
            left_reflected = self.program.method(left_class.id, reflected);
            let subclass = right_class.id != left_class.id && right_class.derives_from(left_class);
            if subclass && backward.is_some() && backward != left_reflected {
                forward = None;
            }
            if backward.is_none() && self.defined_below(right_class.id, reflected) {
                self.diagnostics.push(
                    Diagnostic::new(
                        DiagnosticCode::error(201),
                        span,
                        format!(
                            "an operator whose right operand's class may be a subclass that \
                             defines `{reflected}`, which CPython would call first, is planned \
                             for TyPy 0.5 (CLS-07)"
                        ),
                    )
                    .with_rule(RuleId::CLS_07)
                    .with_help(format!("define `{reflected}` in `{}`", right_class.name)),
                );
            }
        } else {
            // The left operand is not an instance, so only the reflected method applies.
            backward = backward.filter(|_| forward.is_none());
        }
        let returns =
            |method: FunctionId| self.program.signatures[method.0 as usize].returns.clone();
        let ty = match (forward, backward) {
            (Some(forward), Some(backward)) => {
                let (ty, other) = (returns(forward), returns(backward));
                if !ty.matches(&other) {
                    self.diagnostics.push(
                        Diagnostic::new(
                            DiagnosticCode::error(201),
                            span,
                            format!(
                                "an operator whose method `{name}` returns {ty} and whose \
                                 reflected `{reflected}` returns {other} is planned for TyPy 0.5 \
                                 (CLS-07)"
                            ),
                        )
                        .with_rule(RuleId::CLS_07),
                    );
                }
                ty
            }
            (Some(method), None) | (None, Some(method)) => returns(method),
            (None, None) => return None,
        };
        Some(Self::typed(
            TypedExprKind::OperatorCall(Box::new(OperatorCall {
                left: left.clone(),
                right: right.clone(),
                forward,
                reflected: backward,
                left_reflected,
                negated: false,
            })),
            ty,
            span,
        ))
    }

    /// Returns true when a class derived from `class`, other than it, defines `name`.
    fn defined_below(&self, class: crate::typed::ClassId, name: &str) -> bool {
        self.program.classes.iter().any(|other| {
            other.id != class
                && other.method(name).is_some()
                && matches!(self.program.instance_type(other.id), Type::Instance(derived)
                    if derived.ancestors.iter().any(|(id, _)| *id == class))
        })
    }

    /// Returns the comparison method `name` of the class of an instance of type `ty`, when
    /// its class defines or inherits one that takes a value of type `other`.
    fn operator_method(&self, ty: &Type, name: &str, other: &Type) -> Option<FunctionId> {
        let class = ty.class_id().filter(|_| matches!(ty, Type::Instance(_)))?;
        let method = self.program.method(class, name)?;
        let parameter = self.program.signatures[method.0 as usize]
            .parameters
            .get(1)?;
        other.is_assignable_to(&parameter.ty).then_some(method)
    }
}

/// Returns the method an operator calls and the reflected one, `a > b` being `b < a`.
fn method_names(operator: CompareOperator) -> Option<(&'static str, &'static str)> {
    Some(match operator {
        CompareOperator::Eq => ("__eq__", "__eq__"),
        CompareOperator::NotEq => ("__ne__", "__ne__"),
        CompareOperator::Lt => ("__lt__", "__gt__"),
        CompareOperator::LtE => ("__le__", "__ge__"),
        CompareOperator::Gt => ("__gt__", "__lt__"),
        CompareOperator::GtE => ("__ge__", "__le__"),
        _ => return None,
    })
}

/// Returns the method a binary operator calls and the reflected one.
fn binary_names(operator: BinaryOperator) -> (&'static str, &'static str) {
    match operator {
        BinaryOperator::Add => ("__add__", "__radd__"),
        BinaryOperator::Sub => ("__sub__", "__rsub__"),
        BinaryOperator::Mult => ("__mul__", "__rmul__"),
        BinaryOperator::MatMult => ("__matmul__", "__rmatmul__"),
        BinaryOperator::Div => ("__truediv__", "__rtruediv__"),
        BinaryOperator::Mod => ("__mod__", "__rmod__"),
        BinaryOperator::Pow => ("__pow__", "__rpow__"),
        BinaryOperator::LShift => ("__lshift__", "__rlshift__"),
        BinaryOperator::RShift => ("__rshift__", "__rrshift__"),
        BinaryOperator::BitOr => ("__or__", "__ror__"),
        BinaryOperator::BitXor => ("__xor__", "__rxor__"),
        BinaryOperator::BitAnd => ("__and__", "__rand__"),
        BinaryOperator::FloorDiv => ("__floordiv__", "__rfloordiv__"),
    }
}

/// Returns the method a unary operator calls, when one does: `not` tests truth instead.
pub(super) fn unary_name(operator: UnaryOperator) -> Option<&'static str> {
    match operator {
        UnaryOperator::USub => Some("__neg__"),
        UnaryOperator::UAdd => Some("__pos__"),
        UnaryOperator::Invert => Some("__invert__"),
        UnaryOperator::Not => None,
    }
}
