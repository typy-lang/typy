//! Values of type `object` (TYP-23, ADR-0090): a value becomes an object, boxed when TyPy holds
//! it by value, and an `object` narrowed by a test becomes the value again.

use typy_check::Type;
use typy_diagnostics::Span;
use typy_tyir::{Operation, RuntimeFunction, Terminator, TyirType};

use crate::function::FunctionLowerer;
use crate::ownership::Lowered;

impl FunctionLowerer<'_> {
    /// Returns `value`, of the checked type `source`, as an `object`: an object as it is, and
    /// a value held by value boxed. The result takes over the value's reference.
    pub(crate) fn boxed(&mut self, value: Lowered, source: &Type, span: Span) -> Lowered {
        let object = TyirType::UNION;
        if value.ty == object {
            return value;
        }
        let made = match value.ty {
            TyirType::Int => RuntimeFunction::BoxInt,
            TyirType::Float => RuntimeFunction::BoxFloat,
            TyirType::Bool => RuntimeFunction::BoxBool,
            TyirType::Unit => RuntimeFunction::BoxNone,
            TyirType::Optional(_) => {
                let inner = match source {
                    Type::Optional(inner) => inner.as_ref().clone(),
                    other => other.clone(),
                };
                let result = self.optional_branch(
                    value,
                    object,
                    span,
                    |this| this.call_runtime(RuntimeFunction::BoxNone, Vec::new(), span),
                    |this, held| this.boxed(held, &inner, span),
                );
                self.release(value, span);
                return result;
            }
            // A fixed tuple becomes a tuple object of its items, each an `object`, as CPython
            // holds every tuple.
            TyirType::Tuple(_) => {
                let items = match source {
                    Type::Tuple(items) => items.clone(),
                    other => unreachable!("a fixed tuple's checked type is a tuple, not {other}"),
                };
                let (_, types) = self.item_types(value.ty);
                let list = self.list_display(&[], object, span);
                for (index, ty) in types.into_iter().enumerate() {
                    let item = self.borrowed_item(value, index, ty, span);
                    let item = self.boxed(item, &items[index], span);
                    let bits = self.bits(item, span);
                    self.call_runtime(RuntimeFunction::ListAppend, vec![list.value, bits], span);
                    self.release(item, span);
                }
                self.release(value, span);
                self.call_runtime(RuntimeFunction::TupleSeal, vec![list.value], span);
                let upcast = self.builder.push_value(
                    Operation::Upcast {
                        value: list.value,
                        ty: object,
                    },
                    object,
                    span,
                );
                return self.transfer(list, upcast, object);
            }
            ty if self.is_tagged(ty) => {
                let members = source.members().to_vec();
                let result =
                    self.each_member_of(value, source, object, span, |this, index, held| {
                        this.boxed(held, &members[index], span)
                    });
                self.release(value, span);
                return result;
            }
            _ => {
                // An object of any other type is the same object, held as an `object`.
                let upcast = self.builder.push_value(
                    Operation::Upcast {
                        value: value.value,
                        ty: object,
                    },
                    object,
                    span,
                );
                return self.transfer(value, upcast, object);
            }
        };
        let arguments = if value.ty == TyirType::Unit {
            Vec::new()
        } else {
            vec![value.value]
        };
        let boxed = self.call_runtime(made, arguments, span);
        self.release(value, span);
        boxed
    }

    /// Returns `value`, an `object` a test found to be of `target`, as `target` holds it, as
    /// `ty`: an object as it is, a boxed value unboxed, and a union member by member. The
    /// result takes over the value's reference.
    pub(crate) fn unboxed(
        &mut self,
        value: Lowered,
        target: &Type,
        ty: TyirType,
        span: Span,
    ) -> Lowered {
        let read = match ty {
            TyirType::Int => RuntimeFunction::UnboxInt,
            TyirType::Float => RuntimeFunction::UnboxFloat,
            TyirType::Bool => RuntimeFunction::UnboxBool,
            TyirType::Unit => {
                self.release(value, span);
                let unit = self.constant(typy_tyir::Constant::Unit, span);
                return Self::borrowed(unit, TyirType::Unit);
            }
            _ if matches!(target, Type::Union(_) | Type::Optional(_)) => {
                let result = self.unboxed_members(value, target, ty, span);
                self.release(value, span);
                return result;
            }
            _ => {
                let downcast = self.builder.push_value(
                    Operation::Downcast {
                        value: value.value,
                        ty,
                    },
                    ty,
                    span,
                );
                return self.transfer(value, downcast, ty);
            }
        };
        let unboxed = self.call_runtime(read, vec![value.value], span);
        self.release(value, span);
        unboxed
    }

    /// Returns the `object` `value` as the member of the union `target` its class says it is,
    /// held as `ty`: each member but the last is tested in the union's order, which puts `bool`
    /// before `int`, as a `bool` is an `int` too.
    fn unboxed_members(
        &mut self,
        value: Lowered,
        target: &Type,
        ty: TyirType,
        span: Span,
    ) -> Lowered {
        let mut members: Vec<Type> = target.members().to_vec();
        if let Type::Optional(inner) = target {
            members = std::iter::once(Type::NoneType)
                .chain(inner.members().iter().cloned())
                .collect();
        }
        let join = self.builder.create_block();
        let result = self.builder.add_block_parameter(join, ty);
        let borrowed = Self::borrowed(value.value, value.ty);
        let last = members.len() - 1;
        for (position, member) in members.iter().enumerate() {
            if position < last {
                let found = self.object_is(borrowed, member, span);
                let this = self.builder.create_block();
                let next = self.builder.create_block();
                self.builder.terminate(Terminator::Branch {
                    condition: found,
                    then: this,
                    otherwise: next,
                });
                self.builder.switch_to(this);
                let held = self.member_unboxed(borrowed, member, target, ty, span);
                self.join_with(held, join, true, span);
                self.builder.switch_to(next);
            } else {
                let held = self.member_unboxed(borrowed, member, target, ty, span);
                self.join_with(held, join, true, span);
            }
        }
        self.joined(join, Some(result), ty, span)
    }

    /// Returns the borrowed `object` `value` as `member`, then as the union `target` holds it.
    fn member_unboxed(
        &mut self,
        value: Lowered,
        member: &Type,
        target: &Type,
        ty: TyirType,
        span: Span,
    ) -> Lowered {
        let member_ty = self.lowered_type(member);
        let held = self.unboxed(value, member, member_ty, span);
        self.union_converted(held, member, target, ty, true, span)
    }

    /// Returns whether the `object` `value` is of `member`: `None` by its identity, and any
    /// other member as its class says.
    fn object_is(&mut self, value: Lowered, member: &Type, span: Span) -> typy_tyir::ValueId {
        if *member == Type::NoneType {
            let none = self.call_runtime(RuntimeFunction::BoxNone, Vec::new(), span);
            let none = self.borrowed_result(none);
            return self.identical(value, none, span);
        }
        let test = typy_check::member_test(member).unwrap_or(typy_check::MemberTest::Class(
            typy_check::TypeObjectOf::Object,
        ));
        self.member_found(value, &test, span)
    }
}
