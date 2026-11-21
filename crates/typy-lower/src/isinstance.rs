//! Lowers `isinstance` tests and the narrowed reads they allow (CLS-09, TYP-07).

use typy_check::{IsInstanceTest, Type, TypedExpr};
use typy_diagnostics::Span;
use typy_tyir::{BoolOperator, Constant, Operation, RuntimeFunction, TyirType, ValueId};

use crate::function::FunctionLowerer;
use crate::ownership::Lowered;

impl FunctionLowerer<'_> {
    /// Returns `isinstance(value, classes)`: `None` gives what the checker found for it, and
    /// any other value is an instance when every value of its static type is one, or when
    /// the runtime finds its class derives from one of the classes left to check.
    pub(crate) fn isinstance(&mut self, test: &IsInstanceTest, span: Span) -> Lowered {
        let lowered = self.expression(&test.value);
        // The classes held in values are evaluated after the value, as CPython evaluates the
        // arguments of `isinstance` (CLS-18).
        let held: Vec<Lowered> = test
            .held
            .iter()
            .map(|class| self.expression(class))
            .collect();
        let result = if let Type::Optional(inner) = &test.value.ty {
            let none_matches = test.none_matches;
            let held_classes = &held;
            self.optional_branch(
                lowered,
                TyirType::Bool,
                span,
                |this| {
                    let value = this.constant(Constant::Bool(none_matches), span);
                    let none = this.type_object(&typy_check::TypeObjectOf::NoneType, span);
                    let found = this.held_test(none, held_classes, span);
                    let value = this.or(value, found, span);
                    Self::borrowed(value, TyirType::Bool)
                },
                |this, inside| this.instance_test(inside, inner, test, held_classes, span),
            )
        } else {
            let value = Self::borrowed(lowered.value, lowered.ty);
            self.instance_test(value, &test.value.ty, test, &held, span)
        };
        for class in held.into_iter().rev() {
            self.release(class, span);
        }
        self.release(lowered, span);
        result
    }

    /// Returns whether `value`, of the checked type `ty` and not `None`, is an instance of one
    /// of the classes named or held.
    fn instance_test(
        &mut self,
        value: Lowered,
        ty: &Type,
        test: &IsInstanceTest,
        held: &[Lowered],
        span: Span,
    ) -> Lowered {
        // A tagged union is tested as the member it holds (TYP-14).
        if self.is_tagged(value.ty) {
            let members = match ty {
                Type::Union(members) => members.clone(),
                _ => Vec::new(),
            };
            return self.tag_switch(value, TyirType::Bool, span, |this, position, slot| {
                let (always, checks) = &test.members[position];
                let named = this.classes_test(slot, *always, checks, span);
                let class = match members.get(position) {
                    Some(member) if !held.is_empty() => this.runtime_type(slot, member, span),
                    _ => return named,
                };
                let found = this.held_test(class, held, span);
                let value = this.or(named.value, found, span);
                Self::borrowed(value, TyirType::Bool)
            });
        }
        let named = self.classes_test(value, test.always, &test.checks, span);
        if held.is_empty() {
            return named;
        }
        let class = self.runtime_type(value, ty, span);
        let found = self.held_test(class, held, span);
        let value = self.or(named.value, found, span);
        Self::borrowed(value, TyirType::Bool)
    }

    /// Returns whether the type object `class` is or derives from one of `held`, or `None` when
    /// there are none (CLS-18).
    fn held_test(&mut self, class: Lowered, held: &[Lowered], span: Span) -> Option<ValueId> {
        let mut result = None;
        for each in held {
            let class_bits = self.bits(Self::borrowed(class.value, class.ty), span);
            let base = self.bits(Self::borrowed(each.value, each.ty), span);
            let found = self
                .call_runtime(
                    RuntimeFunction::TypeDerivesFrom,
                    vec![class_bits, base],
                    span,
                )
                .value;
            result = Some(self.or(found, result, span));
        }
        result
    }

    /// Returns `left or right`, or `left` alone when there is no `right`.
    fn or(&mut self, left: ValueId, right: Option<ValueId>, span: Span) -> ValueId {
        let Some(right) = right else {
            return left;
        };
        self.builder.push_value(
            Operation::BoolBinary {
                operator: BoolOperator::Or,
                left,
                right,
            },
            TyirType::Bool,
            span,
        )
    }

    /// Returns whether `value`, which is not `None`, is an instance: always, or when the
    /// runtime finds its class derives from one of `checks`.
    fn classes_test(
        &mut self,
        value: Lowered,
        always: bool,
        checks: &[typy_check::MemberTest],
        span: Span,
    ) -> Lowered {
        if always || checks.is_empty() {
            let constant = self.constant(Constant::Bool(always), span);
            return Self::borrowed(constant, TyirType::Bool);
        }
        let mut result = None;
        for test in checks {
            let found = self.member_found(value, test, span);
            result = Some(match result {
                None => found,
                Some(before) => self.builder.push_value(
                    Operation::BoolBinary {
                        operator: BoolOperator::Or,
                        left: before,
                        right: found,
                    },
                    TyirType::Bool,
                    span,
                ),
            });
        }
        let result = result.unwrap_or_else(|| self.constant(Constant::Bool(false), span));
        Self::borrowed(result, TyirType::Bool)
    }

    /// Returns an instance, which a test showed is of a class derived from its static class,
    /// as one of type `ty`; the result takes over the value's reference.
    pub(crate) fn downcast(&mut self, inner: &TypedExpr, ty: TyirType, span: Span) -> Lowered {
        let value = self.expression(inner);
        if value.ty == ty {
            return value;
        }
        // An iterable found an iterator is the same object, held as one (TYP-21).
        if value.ty == TyirType::ITERABLE && ty == TyirType::ITERATOR {
            let bits = self.bits(value, span);
            let iterator = self
                .builder
                .push_value(Operation::FromBits { bits, ty }, ty, span);
            return self.transfer(value, iterator, ty);
        }
        let downcast = self.builder.push_value(
            Operation::Downcast {
                value: value.value,
                ty,
            },
            ty,
            span,
        );
        self.transfer(value, downcast, ty)
    }
}
