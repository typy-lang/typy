//! Lowers `iter()` and `next()`: runtime iterators of the built-in iterables, and the
//! `__next__` of a class (CST-08, CST-09, ADR-0014).

use typy_check::{FunctionId, Type, TypedExpr, View};
use typy_diagnostics::Span;
use typy_tyir::{BlockId, Constant, Operation, RuntimeFunction, Terminator, TyirType, ValueId};

use crate::function::FunctionLowerer;
use crate::ownership::Lowered;

/// Returns the runtime's code for an iterator over a value of `ty`.
fn iterator_kind(ty: &Type) -> i64 {
    match ty {
        Type::List(_) => 0,
        Type::TupleOf(_) => 1,
        Type::Str => 2,
        Type::Bytes => 3,
        Type::Set(_) => 4,
        Type::DictView(View::Values, ..) => 6,
        Type::DictView(View::Items, ..) => 7,
        _ => 5,
    }
}

impl FunctionLowerer<'_> {
    /// Lowers `iter(source)`: a new iterator over a built-in iterable, which takes its own
    /// reference to the source, or to the dict of a view. An iterator of a dict's items gets
    /// the layout of the tuple of a key and a value, which it builds each item in.
    pub(crate) fn iter_of(&mut self, source: &TypedExpr, span: Span) -> Lowered {
        let lowered = self.expression(source);
        let object = if let Type::DictView(..) = source.ty {
            let dict = self.call_runtime(RuntimeFunction::DictViewDict, vec![lowered.value], span);
            self.borrowed_result(dict)
        } else {
            Self::borrowed(lowered.value, lowered.ty)
        };
        let object = self.bits(object, span);
        let kind = self.constant(Constant::Int(iterator_kind(&source.ty).to_string()), span);
        let pair = if let Type::DictView(View::Items, key, value) = &source.ty {
            let pair = self.lowered_type(&Type::Tuple(vec![
                key.as_ref().clone(),
                value.as_ref().clone(),
            ]));
            let TyirType::Tuple(id) = pair else {
                unreachable!("a tuple type lowers to a tuple");
            };
            self.constant(Constant::TupleDescriptor(id), span)
        } else {
            let unit = self.constant(Constant::Unit, span);
            self.bits(Self::borrowed(unit, TyirType::Unit), span)
        };
        let iterator =
            self.call_runtime(RuntimeFunction::IteratorNew, vec![object, kind, pair], span);
        self.release(lowered, span);
        iterator
    }

    /// Lowers `next(iterator)` and `next(iterator, default)`: the next item, with a reference
    /// of its own; at the end, the default, or `StopIteration` without one.
    pub(crate) fn next_of(
        &mut self,
        iterator: &TypedExpr,
        method: Option<FunctionId>,
        default: Option<&TypedExpr>,
        ty: TyirType,
        span: Span,
    ) -> Lowered {
        let lowered = self.expression(iterator);
        let item = match (&iterator.ty, method) {
            (_, Some(next)) => self.lowered_type(&self.program.function(next).signature.returns),
            (Type::Iterator(item), None) => self.lowered_type(item),
            _ => unreachable!("the checker gives `next` an iterator"),
        };
        let Some(default) = default else {
            let value = if let Some(next) = method {
                self.call_function(
                    next,
                    vec![(0, Self::borrowed(lowered.value, lowered.ty))],
                    item,
                    span,
                )
            } else {
                let stop = self.constant(Constant::Int("1".to_owned()), span);
                self.call_runtime(
                    RuntimeFunction::IteratorAdvance,
                    vec![lowered.value, stop],
                    span,
                );
                self.current_item(lowered.value, item, span)
            };
            self.release(lowered, span);
            return value;
        };
        let exhausted = self.builder.create_block();
        let join = self.builder.create_block();
        let result = self.builder.add_block_parameter(join, ty);
        let value = if let Some(next) = method {
            self.call_next(next, lowered.value, item, exhausted, span)
        } else {
            let keep_going = self.constant(Constant::Int("0".to_owned()), span);
            let found = self.call_runtime(
                RuntimeFunction::IteratorAdvance,
                vec![lowered.value, keep_going],
                span,
            );
            let fetch = self.builder.create_block();
            self.builder.terminate(Terminator::Branch {
                condition: found.value,
                then: fetch,
                otherwise: exhausted,
            });
            self.builder.switch_to(fetch);
            self.current_item(lowered.value, item, span)
        };
        self.jump_with(join, value, ty, span);
        self.builder.switch_to(exhausted);
        let fallback = self.expression(default);
        self.jump_with(join, fallback, ty, span);
        self.builder.switch_to(join);
        let value = self.produced(result, ty);
        self.release(lowered, span);
        value
    }

    /// Returns the item an iterator's last step found, with a reference of its own.
    pub(crate) fn current_item(
        &mut self,
        iterator: ValueId,
        item: TyirType,
        span: Span,
    ) -> Lowered {
        let bits = self.call_runtime(RuntimeFunction::IteratorItem, vec![iterator], span);
        let value = self.builder.push_value(
            Operation::FromBits {
                bits: bits.value,
                ty: item,
            },
            item,
            span,
        );
        let owned = self.take_reference(Self::borrowed(value, item), span);
        self.produced(owned, item)
    }

    /// Jumps to `join` with `value` as a value of type `ty`, which takes over its reference;
    /// a value of an optional type's base is wrapped first.
    fn jump_with(&mut self, join: BlockId, value: Lowered, ty: TyirType, span: Span) {
        let owned = self.take_reference(value, span);
        let passed = if value.ty == ty {
            owned
        } else {
            self.builder
                .push_value(Operation::Wrap { value: owned, ty }, ty, span)
        };
        self.builder.terminate(Terminator::Jump {
            target: join,
            arguments: vec![passed],
        });
    }
}
