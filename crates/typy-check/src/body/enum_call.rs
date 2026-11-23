//! What an enum class does as a value (CLS-44, ADR-0116): `Color(value)` finds a member by
//! its value and `Color[name]` by its name, and a loop over the class, `len` and `in` read the
//! list of its members, each through a hidden static method its expansion writes. The
//! variables of its members make each one through `__init__`.

use typy_diagnostics::{Diagnostic, DiagnosticCode, RuleId, Span};
use typy_source::syntax::{CompareOperator, Expr, Identifier, Keyword};

use super::{BodyChecker, Program};
use crate::class_type::{
    ENUM_BITS, ENUM_LOOKUP, ENUM_MEMBER, ENUM_MEMBERS, ENUM_NAMED, ENUM_VALUE,
};
use crate::typed::{ClassId, FunctionId, TypedExpr, TypedExprKind};
use crate::types::Type;

impl Program {
    /// Returns the hidden static method `name` an enum class's expansion writes, or `None`
    /// for any other class.
    pub(crate) fn enum_method(&self, class: ClassId, name: &str) -> Option<FunctionId> {
        self.classes[class.0 as usize].method(name)
    }
}

impl BodyChecker<'_> {
    /// Returns a member made where its enum class's statement runs, for the hidden name its
    /// variable calls; `None` for any other name.
    pub(super) fn enum_member_made(
        &mut self,
        name: &str,
        arguments: &[Expr],
        keywords: &[Keyword],
        span: Span,
    ) -> Option<TypedExpr> {
        let class = *self
            .program
            .class_ids
            .get(name.strip_prefix(ENUM_MEMBER)?)?;
        Some(self.instance_made(class, arguments, keywords, span))
    }

    /// Returns `Class(value)` of an enum class, which finds the member whose value equals
    /// `value`; `None` for any other class.
    pub(super) fn enum_lookup(
        &mut self,
        class: ClassId,
        arguments: &[Expr],
        keywords: &[Keyword],
        span: Span,
    ) -> Option<TypedExpr> {
        let lookup = self.program.enum_method(class, ENUM_LOOKUP)?;
        let by_keyword = matches!(keywords, [keyword]
            if keyword.name.as_ref().is_some_and(|name| name.name == "value"));
        // One value, by its position or as the keyword `value`.
        let one_value = match arguments.len() {
            1 => keywords.is_empty(),
            0 => by_keyword,
            _ => false,
        };
        if !one_value {
            for argument in arguments
                .iter()
                .chain(keywords.iter().map(|keyword| &keyword.value))
            {
                self.expression(argument);
            }
            let class_name = &self.program.classes[class.0 as usize].name;
            let message = format!(
                "`{class_name}` is called with one value, which finds the member that has it"
            );
            return Some(
                self.error(
                    Diagnostic::new(DiagnosticCode::error(404), span, message)
                        .with_rule(RuleId::CLS_44),
                    span,
                ),
            );
        }
        let name = Identifier {
            name: ENUM_LOOKUP.to_owned(),
            span,
        };
        Some(self.static_call(lookup, class, &name, arguments, keywords, span))
    }

    /// Returns `Class[name]` of an enum class, which finds the member `name` names; `None`
    /// when `value` is no enum class.
    pub(super) fn enum_named(
        &mut self,
        value: &TypedExpr,
        index: &Expr,
        span: Span,
    ) -> Option<TypedExpr> {
        let class = enum_class(self.program, &value.ty)?;
        let named = self.program.enum_method(class, ENUM_NAMED)?;
        let name = Identifier {
            name: ENUM_NAMED.to_owned(),
            span,
        };
        Some(self.static_call(named, class, &name, std::slice::from_ref(index), &[], span))
    }

    /// Returns `value`, or the list of its members when it is an enum class or a flag, which a
    /// loop, `len` and `in` read.
    pub(super) fn enum_members(&mut self, value: TypedExpr) -> TypedExpr {
        if let Some(bits) = self.flag_bits(&value) {
            let span = value.span;
            let ty = self.program.signatures[bits.0 as usize].returns.clone();
            return Self::typed(
                TypedExprKind::Call {
                    function: bits,
                    arguments: vec![crate::typed::Argument {
                        parameter: 0,
                        passed: crate::typed::Passed::Position,
                        value,
                    }],
                    dispatch: false,
                },
                ty,
                span,
            );
        }
        let Some(class) = enum_class(self.program, &value.ty) else {
            return value;
        };
        let Some(members) = self.program.enum_method(class, ENUM_MEMBERS) else {
            return value;
        };
        let name = Identifier {
            name: ENUM_MEMBERS.to_owned(),
            span: value.span,
        };
        self.static_call(members, class, &name, &[], &[], value.span)
    }

    /// Returns the operands of a comparison with each enum class that `in` or `not in` reads
    /// as the list of its members.
    pub(super) fn enum_containers(
        &mut self,
        operators: &[CompareOperator],
        operands: Vec<TypedExpr>,
    ) -> Vec<TypedExpr> {
        operands
            .into_iter()
            .enumerate()
            .map(|(position, operand)| {
                let contains = position
                    .checked_sub(1)
                    .and_then(|before| operators.get(before))
                    .is_some_and(|operator| {
                        matches!(operator, CompareOperator::In | CompareOperator::NotIn)
                    });
                if contains {
                    self.enum_members(operand)
                } else {
                    operand
                }
            })
            .collect()
    }
}

impl BodyChecker<'_> {
    /// Returns the hidden method that lists the members a flag holds, when `value` is a member
    /// or a composite of a `Flag` or an `IntFlag`.
    pub(super) fn flag_bits(&self, value: &TypedExpr) -> Option<FunctionId> {
        let Type::Instance(instance) = &value.ty else {
            return None;
        };
        self.program.enum_method(instance.id, ENUM_BITS)
    }

    /// Returns the operands of a binary operator with each member of an `IntEnum` or a
    /// `StrEnum` read as its value, but for `|`, `&` and `^` of a flag, whose methods make a
    /// flag: the flag stays, and the other operand of an `IntFlag` is its value.
    pub(super) fn enum_operands(
        &mut self,
        operator: typy_source::syntax::BinaryOperator,
        (left, right): (TypedExpr, TypedExpr),
    ) -> (TypedExpr, TypedExpr) {
        use typy_source::syntax::BinaryOperator::{BitAnd, BitOr, BitXor};
        if matches!(operator, BitOr | BitAnd | BitXor) {
            if self.flag_bits(&left).is_some() {
                return (left, self.enum_value(right));
            }
            if self.flag_bits(&right).is_some() {
                return (self.enum_value(left), right);
            }
        }
        (self.enum_value(left), self.enum_value(right))
    }

    /// Returns `value`, or the value of the member it is when it is a member of an `IntEnum`
    /// or a `StrEnum`, which CPython reads as its `int` or `str` in arithmetic, in comparisons
    /// and where one is wanted.
    pub(super) fn enum_value(&mut self, value: TypedExpr) -> TypedExpr {
        let Type::Instance(instance) = &value.ty else {
            return value;
        };
        let class = instance.id;
        let Some(method) = self.program.enum_method(class, ENUM_VALUE) else {
            return value;
        };
        let Some(index) = self.program.classes[class.0 as usize].attribute("_value_") else {
            return value;
        };
        let ty = self.program.signatures[method.0 as usize].returns.clone();
        let span = value.span;
        Self::typed(
            TypedExprKind::Attribute {
                object: Box::new(value),
                index,
            },
            ty,
            span,
        )
    }

    /// Returns `value` as its member's value where `expected` takes the value and not the
    /// member, such as an `IntEnum` member passed for an `int`.
    pub(super) fn enum_value_for(&mut self, value: TypedExpr, expected: &Type) -> TypedExpr {
        if value.ty.has_error() || value.ty.is_assignable_to(expected) {
            return value;
        }
        let read = self.enum_value(value.clone());
        if read.ty.is_assignable_to(expected) {
            read
        } else {
            value
        }
    }

    /// Returns the receiver of a method call as its member's value when its class has no
    /// method or attribute `name`, as `Mode.READ.upper()` calls `str.upper`.
    pub(super) fn enum_receiver(&mut self, receiver: TypedExpr, name: &str) -> TypedExpr {
        let Some(class) = receiver.ty.class_id() else {
            return receiver;
        };
        if self.program.method(class, name).is_some()
            || self.program.classes[class.0 as usize]
                .attribute(name)
                .is_some()
        {
            return receiver;
        }
        self.enum_value(receiver)
    }

    /// Returns the operands of a comparison with each member of an `IntEnum` or a `StrEnum`
    /// read as its value, except beside `is` and `is not`, and left of `in` and `not in` where
    /// the container holds members of its class.
    pub(super) fn enum_values_compared(
        &mut self,
        operators: &[CompareOperator],
        operands: Vec<TypedExpr>,
    ) -> Vec<TypedExpr> {
        let items: Vec<Option<Type>> = operands
            .iter()
            .map(|operand| operand.ty.iterable_item())
            .collect();
        operands
            .into_iter()
            .enumerate()
            .map(|(position, operand)| {
                let before = position.checked_sub(1).and_then(|at| operators.get(at));
                let after = operators.get(position);
                let identity = [before, after].into_iter().flatten().any(|operator| {
                    matches!(operator, CompareOperator::Is | CompareOperator::IsNot)
                });
                let held = matches!(after, Some(CompareOperator::In | CompareOperator::NotIn))
                    && items
                        .get(position + 1)
                        .and_then(Option::as_ref)
                        .is_some_and(|item| *item == operand.ty);
                if identity || held {
                    operand
                } else {
                    self.enum_value(operand)
                }
            })
            .collect()
    }
}

/// Returns the enum class a value of type `ty` is, when it is one.
fn enum_class(program: &Program, ty: &Type) -> Option<ClassId> {
    let Type::TypeObject(Some(of)) = ty else {
        return None;
    };
    let class = BodyChecker::program_class(of)?;
    program.enum_method(class, ENUM_LOOKUP).map(|_| class)
}
