//! Lowers unions with a member held by value: a tuple of an `int` tag and a slot for each
//! member, the slot the tag names holding the value and every other slot a value counting
//! passes over (TYP-14, ADR-0027). Owning the tuple is owning its member's reference.

use typy_check::Type;
use typy_diagnostics::Span;
use typy_tyir::{
    Comparison, Constant, Operation, RuntimeFunction, ScalarKind, Terminator, TyirType, ValueId,
};

use crate::function::FunctionLowerer;
use crate::ownership::Lowered;

impl FunctionLowerer<'_> {
    /// Returns true when values of `ty` are tagged unions.
    pub(crate) fn is_tagged(&self, ty: TyirType) -> bool {
        matches!(ty, TyirType::Tuple(id) if self.tuple_types.is_union(id))
    }

    /// Returns a tagged union of type `ty` that holds `member`, a value of the member at
    /// `position`; the union takes over the member's reference.
    fn tagged(&mut self, member: Lowered, position: usize, ty: TyirType, span: Span) -> Lowered {
        let (id, items) = self.item_types(ty);
        let slot = items[position + 1];
        let member = self.cast_to(member, slot, span);
        let given = self.take_reference(member, span);
        let mut values = vec![self.constant(Constant::Int(position.to_string()), span)];
        for (index, item) in items.iter().skip(1).enumerate() {
            let value = if index == position {
                given
            } else {
                self.zero_value(*item, span)
            };
            values.push(value);
        }
        let union = self.builder.push_value(
            Operation::MakeTuple {
                ty: id,
                items: values,
            },
            ty,
            span,
        );
        self.produced(union, ty)
    }

    /// Returns `value` as a value of `ty`, which holds it as it is: a class's instance as its
    /// base's, or any object as a union of objects.
    fn cast_to(&mut self, value: Lowered, ty: TyirType, span: Span) -> Lowered {
        if value.ty == ty {
            return value;
        }
        let cast = self.builder.push_value(
            Operation::Upcast {
                value: value.value,
                ty,
            },
            ty,
            span,
        );
        self.transfer(value, cast, ty)
    }

    /// Returns what `arm` gives for the member a tagged union holds: each arm takes the
    /// member's position and its slot, borrowed from the union, and the results join as a
    /// value of `ty`, or as nothing for `unit`. The union is not released.
    pub(crate) fn tag_switch(
        &mut self,
        union: Lowered,
        ty: TyirType,
        span: Span,
        mut arm: impl FnMut(&mut Self, usize, Lowered) -> Lowered,
    ) -> Lowered {
        let (_, items) = self.item_types(union.ty);
        let tag = self.borrowed_item(union, 0, TyirType::Int, span).value;
        let join = self.builder.create_block();
        let gives = ty != TyirType::Unit;
        let result = gives.then(|| self.builder.add_block_parameter(join, ty));
        let last = items.len() - 2;
        for position in 0..=last {
            if position < last {
                let expected = self.constant(Constant::Int(position.to_string()), span);
                let found = self.builder.push_value(
                    Operation::Compare {
                        kind: ScalarKind::Int,
                        comparison: Comparison::Equal,
                        left: tag,
                        right: expected,
                    },
                    TyirType::Bool,
                    span,
                );
                let this = self.builder.create_block();
                let next = self.builder.create_block();
                self.builder.terminate(Terminator::Branch {
                    condition: found,
                    then: this,
                    otherwise: next,
                });
                self.builder.switch_to(this);
                let slot = self.borrowed_item(union, position + 1, items[position + 1], span);
                let outcome = arm(self, position, slot);
                self.join_with(outcome, join, gives, span);
                self.builder.switch_to(next);
            } else {
                let slot = self.borrowed_item(union, position + 1, items[position + 1], span);
                let outcome = arm(self, position, slot);
                self.join_with(outcome, join, gives, span);
            }
        }
        self.joined(join, result, ty, span)
    }

    /// Continues at `join`, where the branches' result arrives, or a unit value when they give
    /// nothing.
    pub(crate) fn joined(
        &mut self,
        join: typy_tyir::BlockId,
        result: Option<ValueId>,
        ty: TyirType,
        span: Span,
    ) -> Lowered {
        self.builder.switch_to(join);
        if let Some(result) = result {
            return self.produced(result, ty);
        }
        let unit = self.constant(Constant::Unit, span);
        Self::borrowed(unit, TyirType::Unit)
    }

    /// Ends a branch, passing `outcome` with its reference to `join`, or releasing it when the
    /// branches give nothing.
    pub(crate) fn join_with(
        &mut self,
        outcome: Lowered,
        join: typy_tyir::BlockId,
        gives: bool,
        span: Span,
    ) {
        let arguments = if gives {
            vec![self.take_reference(outcome, span)]
        } else {
            self.release(outcome, span);
            Vec::new()
        };
        self.builder.terminate(Terminator::Jump {
            target: join,
            arguments,
        });
    }

    /// Returns `value`, of the checked type `source`, as a value of the union `target`, held
    /// as `ty`: a member is tagged with its position, and a union is tagged member by member.
    /// The result takes over the value's reference.
    pub(crate) fn widened_to_union(
        &mut self,
        value: Lowered,
        source: &Type,
        target: &Type,
        ty: TyirType,
        span: Span,
    ) -> Lowered {
        let Type::Union(members) = source else {
            let position = position_in(target, source);
            return if self.is_tagged(ty) {
                self.tagged(value, position, ty, span)
            } else {
                self.cast_to(value, ty, span)
            };
        };
        let members = members.clone();
        let result = self.each_member_of(value, source, ty, span, |this, index, held| {
            this.widened_to_union(held, &members[index], target, ty, span)
        });
        self.release(value, span);
        result
    }

    /// Returns `value`, of the union `source`, narrowed to `target`, one of its members or a
    /// union of some of them, held as `ty`; a member `target` does not hold never reaches here
    /// when the program runs. The result takes over the value's reference.
    pub(crate) fn narrowed_union(
        &mut self,
        value: Lowered,
        source: &Type,
        target: &Type,
        ty: TyirType,
        span: Span,
    ) -> Lowered {
        let members = source.members().to_vec();
        // One member, or a class derived from one, is read from that member's slot.
        if !matches!(target, Type::Union(_))
            && let Some(position) = members
                .iter()
                .position(|member| target.is_assignable_to(member))
        {
            let (_, items) = self.item_types(value.ty);
            let slot = self.borrowed_item(value, position + 1, items[position + 1], span);
            let narrowed = if slot.ty == ty {
                slot.value
            } else {
                self.builder.push_value(
                    Operation::Downcast {
                        value: slot.value,
                        ty,
                    },
                    ty,
                    span,
                )
            };
            return self.transfer(value, narrowed, ty);
        }
        let result = self.tag_switch(value, ty, span, |this, index, held| {
            let member = &members[index];
            if member.is_assignable_to(target) {
                this.widened_to_union(held, member, target, ty, span)
            } else {
                let zero = this.zero_value(ty, span);
                Self::borrowed(zero, ty)
            }
        });
        self.release(value, span);
        result
    }

    /// Returns what `arm` gives for the member a union holds, tagged or of objects: an arm
    /// takes the member's position and the value as that member, borrowed.
    pub(crate) fn each_member_of(
        &mut self,
        value: Lowered,
        union: &Type,
        ty: TyirType,
        span: Span,
        mut arm: impl FnMut(&mut Self, usize, Lowered) -> Lowered,
    ) -> Lowered {
        if self.is_tagged(value.ty) {
            return self.tag_switch(value, ty, span, arm);
        }
        let members = union.members().to_vec();
        let join = self.builder.create_block();
        let gives = ty != TyirType::Unit;
        let result = gives.then(|| self.builder.add_block_parameter(join, ty));
        let last = members.len() - 1;
        for (position, member) in members.iter().enumerate() {
            let held_ty = self.lowered_type(member);
            if position < last {
                let found = self.is_member(value, member, span);
                let this = self.builder.create_block();
                let next = self.builder.create_block();
                self.builder.terminate(Terminator::Branch {
                    condition: found,
                    then: this,
                    otherwise: next,
                });
                self.builder.switch_to(this);
                let held = self.narrowed_object(value, held_ty, span);
                let outcome = arm(self, position, held);
                self.join_with(outcome, join, gives, span);
                self.builder.switch_to(next);
            } else {
                let held = self.narrowed_object(value, held_ty, span);
                let outcome = arm(self, position, held);
                self.join_with(outcome, join, gives, span);
            }
        }
        self.joined(join, result, ty, span)
    }

    /// Returns whether the object `value` holds is of `member`, as its class says.
    fn is_member(&mut self, value: Lowered, member: &Type, span: Span) -> ValueId {
        // `NotImplemented` is the one object of its type (CLS-21).
        if *member == Type::NotImplemented {
            let found = self.call_runtime(RuntimeFunction::NotImplemented, Vec::new(), span);
            let found = self.borrowed_result(found);
            return self.identical(value, found, span);
        }
        let test = typy_check::member_test(member).unwrap_or(typy_check::MemberTest::Class(
            typy_check::TypeObjectOf::Object,
        ));
        self.member_found(value, &test, span)
    }

    /// Returns the object of a union of objects as a value of `ty`, borrowed.
    fn narrowed_object(&mut self, value: Lowered, ty: TyirType, span: Span) -> Lowered {
        if value.ty == ty {
            return Self::borrowed(value.value, ty);
        }
        let downcast = self.builder.push_value(
            Operation::Downcast {
                value: value.value,
                ty,
            },
            ty,
            span,
        );
        Self::borrowed(downcast, ty)
    }
}

impl FunctionLowerer<'_> {
    /// Lowers a conversion the checker made between a value of `inner`'s type and `target`,
    /// held as `ty`: wider when `up`, narrower after a test otherwise. A conversion that
    /// involves a tagged union, also inside an optional, tags or reads the member; any other
    /// is the cast between object types it always was.
    pub(crate) fn converted(
        &mut self,
        inner: &typy_check::TypedExpr,
        target: &Type,
        ty: TyirType,
        up: bool,
        span: Span,
    ) -> Lowered {
        let source = self.lowered_type(&inner.ty);
        if !self.involves_tag(source) && !self.involves_tag(ty) {
            return if up {
                self.upcast(inner, ty, span)
            } else {
                self.downcast(inner, ty, span)
            };
        }
        let value = self.expression(inner);
        self.union_converted(value, &inner.ty, target, ty, up, span)
    }

    /// Returns true when values of `ty` are tagged unions, or optional ones.
    pub(crate) fn involves_tag(&self, ty: TyirType) -> bool {
        match ty {
            TyirType::Optional(typy_tyir::OptionalBase::Tuple(pair)) => self.is_tagged(
                self.tuple_types
                    .optional_value(typy_tyir::OptionalBase::Tuple(pair)),
            ),
            other => self.is_tagged(other),
        }
    }

    /// Converts `value`, of the checked type `source`, to `target`, held as `ty`; the result
    /// takes over the value's reference.
    pub(crate) fn union_converted(
        &mut self,
        value: Lowered,
        source: &Type,
        target: &Type,
        ty: TyirType,
        up: bool,
        span: Span,
    ) -> Lowered {
        if value.ty == ty {
            return value;
        }
        if let (Type::Optional(source), Type::Optional(target)) = (source, target) {
            let (source, target) = (source.as_ref().clone(), target.as_ref().clone());
            let inner_ty = self.lowered_type(&target);
            let result = self.optional_branch(
                value,
                ty,
                span,
                |this| Self::borrowed(this.none_of(ty, span), ty),
                |this, held| {
                    let converted =
                        this.union_converted(held, &source, &target, inner_ty, up, span);
                    let wrapped = this.builder.push_value(
                        Operation::Wrap {
                            value: converted.value,
                            ty,
                        },
                        ty,
                        span,
                    );
                    this.transfer(converted, wrapped, ty)
                },
            );
            self.release(value, span);
            return result;
        }
        if up {
            self.widened_to_union(value, source, target, ty, span)
        } else {
            self.narrowed_union(value, source, target, ty, span)
        }
    }
}

/// Returns the position of the member of `union` that holds a value of `source`: the first
/// member that accepts it.
fn position_in(union: &Type, source: &Type) -> usize {
    union
        .members()
        .iter()
        .position(|member| source.is_assignable_to(member))
        .unwrap_or(0)
}
