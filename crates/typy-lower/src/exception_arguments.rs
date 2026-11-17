//! Lowers `args` of an exception read as a value: the items of its arguments, read by their
//! layout. For a union of tuples, the runtime compares the arguments' layout with each
//! member's in turn, and the member whose layout they have is read and tagged (EXC-15,
//! ADR-0034).

use typy_check::{Type, TypedExpr};
use typy_diagnostics::Span;
use typy_tyir::{Constant, Operation, RuntimeFunction, Terminator, TyirType};

use crate::function::FunctionLowerer;
use crate::ownership::Lowered;

impl FunctionLowerer<'_> {
    /// Returns `args` of `exception`, or its arguments as it holds them when `raw`, of the
    /// checked type `checked`, held as `ty`.
    pub(crate) fn exception_arguments(
        &mut self,
        exception: &TypedExpr,
        raw: bool,
        checked: &Type,
        ty: TyirType,
        span: Span,
    ) -> Lowered {
        let exception = self.expression(exception);
        let result = match checked {
            Type::Union(members) => {
                self.arguments_of_union(exception, raw, members, checked, ty, span)
            }
            member => self.arguments_as(exception, member, span),
        };
        self.release(exception, span);
        result
    }

    /// Returns the arguments of `exception` as the union `checked` of `members`, held as
    /// `ty`: the first member whose layout the arguments have, which the last one has when
    /// no other does.
    fn arguments_of_union(
        &mut self,
        exception: Lowered,
        raw: bool,
        members: &[Type],
        checked: &Type,
        ty: TyirType,
        span: Span,
    ) -> Lowered {
        let join = self.builder.create_block();
        let result = self.builder.add_block_parameter(join, ty);
        let last = members.len() - 1;
        for (position, member) in members.iter().enumerate() {
            let next = (position < last).then(|| {
                let Type::Tuple(items) = member else {
                    unreachable!("`args` is a union of tuples")
                };
                let id = self.tuple_type_id(items);
                let layout = self.constant(Constant::TupleDescriptor(id), span);
                let raw = self.constant(Constant::Int(u8::from(raw).to_string()), span);
                let matches = self
                    .call_runtime(
                        RuntimeFunction::ExceptionArgumentsMatch,
                        vec![exception.value, layout, raw],
                        span,
                    )
                    .value;
                let this = self.builder.create_block();
                let next = self.builder.create_block();
                self.builder.terminate(Terminator::Branch {
                    condition: matches,
                    then: this,
                    otherwise: next,
                });
                self.builder.switch_to(this);
                next
            });
            let read = self.arguments_as(exception, member, span);
            let tagged = self.widened_to_union(read, member, checked, ty, span);
            self.join_with(tagged, join, true, span);
            if let Some(next) = next {
                self.builder.switch_to(next);
            }
        }
        self.joined(join, Some(result), ty, span)
    }

    /// Returns the arguments of `exception` as the tuple `member`, with a reference of their
    /// own, so they outlive the exception.
    fn arguments_as(&mut self, exception: Lowered, member: &Type, span: Span) -> Lowered {
        let ty = self.lowered_type(member);
        if matches!(member, Type::Tuple(items) if items.is_empty()) {
            let empty = self.zero_value(ty, span);
            return Self::borrowed(empty, ty);
        }
        let address = self
            .call_runtime(
                RuntimeFunction::ExceptionArgumentsAddress,
                vec![exception.value],
                span,
            )
            .value;
        let items = self
            .builder
            .push_value(Operation::FromBits { bits: address, ty }, ty, span);
        self.owned_copy(items, ty, span)
    }
}
