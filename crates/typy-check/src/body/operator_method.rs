//! Operators on instances whose classes define operator or comparison methods: `a < b` calls
//! `a.__lt__(b)`, or the reflected `b.__gt__(a)`, and a method that returns `NotImplemented`
//! gives way to the other (CLS-07, CLS-21, ADR-0026, ADR-0038).

use typy_diagnostics::{Diagnostic, DiagnosticCode, RuleId, Span};
use typy_source::syntax::{BinaryOperator, CompareOperator, UnaryOperator};

use super::BodyChecker;
use crate::typed::{Argument, FunctionId, OperatorCall, OperatorKind, TypedExpr, TypedExprKind};
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
        let kind = OperatorKind::Arithmetic(binary_symbol(operator, false));
        match self.operator_method_call((name, reflected), &left, &right, kind, span) {
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
                    passed: crate::typed::Passed::Position,
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
        if let CompareOperator::In | CompareOperator::NotIn = operator
            && let Some(call) =
                self.contains_call(&left, &right, operator == CompareOperator::NotIn, span)
        {
            return Comparison::Called(call);
        }
        let Some((name, reflected, kind)) = method_names(operator) else {
            return Comparison::Builtin(left, right);
        };
        if let Some(call) = self.operator_method_call((name, reflected), &left, &right, kind, span)
        {
            return Comparison::Called(call);
        }
        let equality = OperatorKind::Equality { different: false };
        if operator == CompareOperator::NotEq
            && let Some(equal) =
                self.operator_method_call(("__eq__", "__eq__"), &left, &right, equality, span)
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
    /// classes defines the method and it takes the other operand. Which runs first, and what
    /// follows a method that gives way, is the runtime's to decide from the objects' classes,
    /// as `kind` says; the call's type is what the methods return other than `NotImplemented`.
    pub(super) fn operator_method_call(
        &mut self,
        (name, reflected): (&str, &str),
        left: &TypedExpr,
        right: &TypedExpr,
        kind: OperatorKind,
        span: Span,
    ) -> Option<TypedExpr> {
        let forward = self.operator_method(&left.ty, name, &right.ty);
        let mut left_reflected = None;
        let mut backward = self.operator_method(&right.ty, reflected, &left.ty);
        // A class derived from the right operand's may define the reflected method its own
        // class lacks, which CPython calls for an object of that class (CLS-07).
        let reported = self.diagnostics.len();
        let below = match &right.ty {
            Type::Instance(right_class) if backward.is_none() => {
                self.reflected_below(right_class.id, reflected, &left.ty, span)
            }
            _ => Vec::new(),
        };
        if forward.is_none() && backward.is_none() && below.is_empty() {
            // A derived class's method that does not take the operand was reported already.
            return (self.diagnostics.len() > reported).then(|| Self::invalid_call(span));
        }
        if let (Type::Instance(left_class), Type::Instance(_)) = (&left.ty, &right.ty) {
            // Which runs is the runtime's to decide, from the objects' classes, which may
            // derive from the static ones.
            left_reflected = self.program.method(left_class.id, reflected);
        } else {
            // The left operand is not an instance, so only the reflected method applies.
            backward = backward.filter(|_| forward.is_none());
        }
        let returns = |method: FunctionId| {
            self.program.signatures[method.0 as usize]
                .returns
                .implemented()
        };
        let ty = match (forward, backward) {
            (Some(forward), Some(backward)) => {
                let ty = returns(forward);
                // The reflected method's result is the operator's too when it may run (CLS-22).
                let methods = (forward, backward);
                if self.reflected_may_run(methods, (reflected, left_reflected), left, right) {
                    Type::union(vec![ty, returns(backward)])
                } else {
                    ty
                }
            }
            (Some(method), None) | (None, Some(method)) => returns(method),
            (None, None) => Type::union(below.iter().map(|(_, method)| returns(*method)).collect()),
        };
        // Each method a derived class defines may give the operator's value.
        let ty = if forward.is_some() && !below.is_empty() {
            let mut members = vec![ty];
            members.extend(below.iter().map(|(_, method)| returns(*method)));
            Type::union(members)
        } else {
            ty
        };
        Some(Self::typed(
            TypedExprKind::OperatorCall(Box::new(OperatorCall {
                left: left.clone(),
                right: right.clone(),
                forward,
                reflected: backward,
                left_reflected,
                negated: false,
                in_place: None,
                reflected_below: below,
                kind,
            })),
            ty,
            span,
        ))
    }

    /// Returns the classes derived from `class`, which has no method `reflected`, that define
    /// it, the nearest of each line, each with its method, after reporting one that does not
    /// take a value of `left`, which CPython would pass it.
    fn reflected_below(
        &mut self,
        class: crate::typed::ClassId,
        reflected: &str,
        left: &Type,
        span: Span,
    ) -> Vec<(crate::typed::ClassId, FunctionId)> {
        if self.program.method(class, reflected).is_some() {
            return Vec::new();
        }
        let definers: Vec<(crate::typed::ClassId, FunctionId, String)> = self
            .program
            .classes
            .iter()
            .filter(|other| other.id != class)
            .filter_map(|other| Some((other.id, other.method(reflected)?, other.name.clone())))
            .filter(|(other, _, _)| {
                matches!(self.program.instance_type(*other), Type::Instance(derived)
                if derived.ancestors.iter().any(|(id, _)| *id == class)
                    && derived.ancestors.first().is_none_or(|(base, _)| {
                        self.program.method(*base, reflected).is_none()
                    }))
            })
            .collect();
        let mut below = Vec::new();
        for (other, method, other_name) in definers {
            let takes = self.program.signatures[method.0 as usize]
                .parameters
                .get(1)
                .map(|parameter| parameter.ty.clone());
            if takes
                .as_ref()
                .is_some_and(|takes| left.is_assignable_to(takes))
            {
                below.push((other, method));
                continue;
            }
            self.diagnostics.push(
                Diagnostic::new(
                    DiagnosticCode::error(404),
                    span,
                    format!(
                        "`{other_name}.{reflected}`, which CPython would call for an object of \
                         `{other_name}`, does not take {}",
                        left.with_article()
                    ),
                )
                .with_rule(RuleId::CLS_07),
            );
        }
        below
    }

    /// Returns true when the reflected method of an operator that has both may run: when the
    /// method may give way, or when the right object's class may derive from the left
    /// object's and have another reflected method, which then runs first.
    fn reflected_may_run(
        &self,
        (forward, backward): (FunctionId, FunctionId),
        (reflected, left_reflected): (&str, Option<FunctionId>),
        left: &TypedExpr,
        right: &TypedExpr,
    ) -> bool {
        if self.program.signatures[forward.0 as usize]
            .returns
            .may_give_way()
        {
            return true;
        }
        let (Type::Instance(left_class), Type::Instance(right_class)) = (&left.ty, &right.ty)
        else {
            return false;
        };
        let related = left_class.derives_from(right_class) || right_class.derives_from(left_class);
        related
            && (Some(backward) != left_reflected || self.defined_below(left_class.id, reflected))
    }

    /// Returns true when a class derived from `class`, other than it, defines `name`.
    pub(super) fn defined_below(&self, class: crate::typed::ClassId, name: &str) -> bool {
        self.program.classes.iter().any(|other| {
            other.id != class
                && other.method(name).is_some()
                && matches!(self.program.instance_type(other.id), Type::Instance(derived)
                    if derived.ancestors.iter().any(|(id, _)| *id == class))
        })
    }

    /// Returns the comparison method `name` of the class of an instance of type `ty`, when
    /// its class defines or inherits one that takes a value of type `other`.
    pub(super) fn operator_method(
        &self,
        ty: &Type,
        name: &str,
        other: &Type,
    ) -> Option<FunctionId> {
        let class = ty.class_id().filter(|_| matches!(ty, Type::Instance(_)))?;
        let method = self.program.method(class, name)?;
        let parameter = self.program.signatures[method.0 as usize]
            .parameters
            .get(1)?;
        other.is_assignable_to(&parameter.ty).then_some(method)
    }
}

/// Returns the method a comparison calls, the reflected one, `a > b` being `b < a`, and the
/// kind of operator it is.
fn method_names(operator: CompareOperator) -> Option<(&'static str, &'static str, OperatorKind)> {
    Some(match operator {
        CompareOperator::Eq => (
            "__eq__",
            "__eq__",
            OperatorKind::Equality { different: false },
        ),
        CompareOperator::NotEq => (
            "__ne__",
            "__ne__",
            OperatorKind::Equality { different: true },
        ),
        CompareOperator::Lt => ("__lt__", "__gt__", OperatorKind::Ordering("<")),
        CompareOperator::LtE => ("__le__", "__ge__", OperatorKind::Ordering("<=")),
        CompareOperator::Gt => ("__gt__", "__lt__", OperatorKind::Ordering(">")),
        CompareOperator::GtE => ("__ge__", "__le__", OperatorKind::Ordering(">=")),
        _ => return None,
    })
}

/// Returns the symbol CPython's `TypeError` names for a binary operator, or for its augmented
/// assignment when `in_place`.
pub(super) const fn binary_symbol(operator: BinaryOperator, in_place: bool) -> &'static str {
    let (plain, augmented) = match operator {
        BinaryOperator::Add => ("+", "+="),
        BinaryOperator::Sub => ("-", "-="),
        BinaryOperator::Mult => ("*", "*="),
        BinaryOperator::MatMult => ("@", "@="),
        BinaryOperator::Div => ("/", "/="),
        BinaryOperator::Mod => ("%", "%="),
        BinaryOperator::Pow => ("** or pow()", "**="),
        BinaryOperator::LShift => ("<<", "<<="),
        BinaryOperator::RShift => (">>", ">>="),
        BinaryOperator::BitOr => ("|", "|="),
        BinaryOperator::BitXor => ("^", "^="),
        BinaryOperator::BitAnd => ("&", "&="),
        BinaryOperator::FloorDiv => ("//", "//="),
    };
    if in_place { augmented } else { plain }
}

/// Returns the method a binary operator calls and the reflected one.
pub(super) fn binary_names(operator: BinaryOperator) -> (&'static str, &'static str) {
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
