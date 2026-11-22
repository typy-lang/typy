//! Lowers `isinstance` tests and the narrowed reads they allow (CLS-09, TYP-07).

use typy_check::{IsInstanceTest, Type, TypedExpr};
use typy_diagnostics::Span;
use typy_tyir::{BoolOperator, Constant, Operation, RuntimeFunction, TyirType, ValueId};

use crate::function::FunctionLowerer;
use crate::ownership::Lowered;

/// A class held in a value that `isinstance` or `issubclass` tests against: one class, or a
/// `tuple[type, ...]` whose items the runtime goes over.
#[derive(Clone, Copy)]
enum Held {
    Class(Lowered),
    Classes(Lowered),
}

impl FunctionLowerer<'_> {
    /// Returns `isinstance(value, classes)`: `None` gives what the checker found for it, and
    /// any other value is an instance when every value of its static type is one, or when
    /// the runtime finds its class derives from one of the classes left to check.
    pub(crate) fn isinstance(&mut self, test: &IsInstanceTest, span: Span) -> Lowered {
        let lowered = self.expression(&test.value);
        let result = self.tested(Self::borrowed(lowered.value, lowered.ty), test, span);
        self.release(lowered, span);
        result
    }

    /// Returns the outcome of `test` for `lowered`, its value evaluated already.
    fn tested(&mut self, lowered: Lowered, test: &IsInstanceTest, span: Span) -> Lowered {
        // The classes held in values are evaluated after the value, as CPython evaluates the
        // arguments of `isinstance` (CLS-18).
        let (owners, held) = self.held_classes(&test.held, span);
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
        for owner in owners.into_iter().rev() {
            self.release(owner, span);
        }
        result
    }

    /// Returns `cast(target, value)` checked when the program runs (TYP-24, ADR-0091): the
    /// value as `target` when the test finds it one, and otherwise the `TypeError` that names
    /// the cast's type and the value's class.
    pub(crate) fn cast(
        &mut self,
        cast: &typy_check::CastCheck,
        target: &Type,
        ty: TyirType,
        span: Span,
    ) -> Lowered {
        let value = self.expression(&cast.test.value);
        let holds = self.tested(Self::borrowed(value.value, value.ty), &cast.test, span);
        // The class the message names is found before the branch, which a tagged union reads
        // through blocks of its own.
        let name = self.string(&cast.target, span);
        let found = self.runtime_type(
            Self::borrowed(value.value, value.ty),
            &cast.test.value.ty,
            span,
        );
        let fails = self.builder.create_block();
        let passes = self.builder.create_block();
        self.builder.terminate(typy_tyir::Terminator::Branch {
            condition: holds.value,
            then: passes,
            otherwise: fails,
        });
        self.builder.switch_to(fails);
        self.call_runtime(RuntimeFunction::CastFailed, vec![name, found.value], span);
        self.builder.terminate(typy_tyir::Terminator::Unreachable);
        self.builder.switch_to(passes);
        self.cast_narrowed(value, &cast.test.value.ty, target, ty, span)
    }

    /// Returns `value`, of `source`, as the `target` a test found it to be, held as `ty`; the
    /// result takes over the value's reference.
    fn cast_narrowed(
        &mut self,
        value: Lowered,
        source: &Type,
        target: &Type,
        ty: TyirType,
        span: Span,
    ) -> Lowered {
        if value.ty == ty {
            return value;
        }
        if *source == Type::Object {
            return self.unboxed(value, target, ty, span);
        }
        if let Type::Optional(inner) = source
            && !matches!(target, Type::Optional(_))
        {
            let base = self.base_type(value.ty);
            let held = self
                .builder
                .push_value(Operation::Unwrap(value.value), base, span);
            let held = self.transfer(value, held, base);
            return self.cast_narrowed(held, inner, target, ty, span);
        }
        if self.involves_tag(value.ty) || self.involves_tag(ty) {
            return self.union_converted(value, source, target, ty, false, span);
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

    /// Returns the values of `classes`, evaluated in order, and the classes they hold: a class,
    /// or each item of a fixed tuple of classes, borrowed from the value, or a
    /// `tuple[type, ...]` the runtime goes over (CLS-18, CLS-19).
    fn held_classes(&mut self, classes: &[TypedExpr], span: Span) -> (Vec<Lowered>, Vec<Held>) {
        let mut owners = Vec::new();
        let mut held = Vec::new();
        for class in classes {
            let lowered = self.expression(class);
            match &class.ty {
                Type::Tuple(items) => {
                    for (index, item) in items.iter().enumerate() {
                        let ty = self.lowered_type(item);
                        held.push(Held::Class(self.borrowed_item(lowered, index, ty, span)));
                    }
                }
                Type::TupleOf(_) => {
                    held.push(Held::Classes(Self::borrowed(lowered.value, lowered.ty)));
                }
                _ => held.push(Held::Class(Self::borrowed(lowered.value, lowered.ty))),
            }
            owners.push(lowered);
        }
        (owners, held)
    }

    /// Returns whether `value`, of the checked type `ty` and not `None`, is an instance of one
    /// of the classes named or held.
    fn instance_test(
        &mut self,
        value: Lowered,
        ty: &Type,
        test: &IsInstanceTest,
        held: &[Held],
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

    /// Returns `issubclass(class, bases)`: the class and each base evaluated in order, true
    /// when the class is or derives from one of them (CLS-19).
    pub(crate) fn issubclass(
        &mut self,
        class: &TypedExpr,
        bases: &[TypedExpr],
        span: Span,
    ) -> Lowered {
        let lowered = self.expression(class);
        let (owners, held) = self.held_classes(bases, span);
        let found = self.held_test(lowered, &held, span);
        let result = found.unwrap_or_else(|| self.constant(Constant::Bool(false), span));
        for owner in owners.into_iter().rev() {
            self.release(owner, span);
        }
        self.release(lowered, span);
        Self::borrowed(result, TyirType::Bool)
    }

    /// Returns whether the type object `class` is or derives from one of `held`, or `None` when
    /// there are none (CLS-18).
    fn held_test(&mut self, class: Lowered, held: &[Held], span: Span) -> Option<ValueId> {
        let mut result = None;
        for each in held {
            let (each, function) = match each {
                Held::Class(each) => (each, RuntimeFunction::TypeDerivesFrom),
                Held::Classes(each) => (each, RuntimeFunction::TypeDerivesFromAny),
            };
            let class_bits = self.bits(Self::borrowed(class.value, class.ty), span);
            let base = self.bits(Self::borrowed(each.value, each.ty), span);
            let found = self
                .call_runtime(function, vec![class_bits, base], span)
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
