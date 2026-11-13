//! Comparisons of instances whose classes define comparison methods: `a < b` calls
//! `a.__lt__(b)`, or the reflected `b.__gt__(a)` (CLS-07, ADR-0026).

use typy_diagnostics::{Diagnostic, DiagnosticCode, RuleId, Span};
use typy_source::syntax::{CompareOperator, UnaryOperator};

use super::BodyChecker;
use crate::typed::{FunctionId, OperatorCall, TypedExpr, TypedExprKind};
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
        if let Some(call) = self.comparison_call(name, reflected, &left, &right, span) {
            return Comparison::Called(call);
        }
        if operator == CompareOperator::NotEq
            && let Some(equal) = self.comparison_call("__eq__", "__eq__", &left, &right, span)
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
    fn comparison_call(
        &mut self,
        name: &str,
        reflected: &str,
        left: &TypedExpr,
        right: &TypedExpr,
        span: Span,
    ) -> Option<TypedExpr> {
        let mut forward = self.comparison_method(&left.ty, name, &right.ty);
        let mut left_reflected = None;
        let mut backward = self.comparison_method(&right.ty, reflected, &left.ty);
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
                            "a comparison whose right operand's class may be a subclass that \
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
        Some(Self::typed(
            TypedExprKind::OperatorCall(Box::new(OperatorCall {
                left: left.clone(),
                right: right.clone(),
                forward,
                reflected: backward,
                left_reflected,
                negated: false,
            })),
            Type::Bool,
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
    fn comparison_method(&self, ty: &Type, name: &str, other: &Type) -> Option<FunctionId> {
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
