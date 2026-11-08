//! Lowers the builtins of one iterable, `sorted`, `sum`, `any`, `all`, `min` and `max`, over a
//! list (LIB-02), and `list.sort` (LIB-03).

use typy_check::{Builtin, Type, TypedExpr};
use typy_diagnostics::Span;
use typy_tyir::{
    Comparison, Constant, Operation, RuntimeFunction, ScalarKind, Terminator, TyirType, ValueId,
};

use crate::function::FunctionLowerer;
use crate::ownership::Lowered;

impl FunctionLowerer<'_> {
    /// Lowers `builtin(iterable, option)`, whose result has type `result`; the iterable is
    /// evaluated first, then the option, and both are released afterwards.
    pub(crate) fn iterable_builtin(
        &mut self,
        builtin: Builtin,
        iterable: &TypedExpr,
        option: Option<&TypedExpr>,
        result: TyirType,
        span: Span,
    ) -> Lowered {
        let list = self.expression(iterable);
        let option = option.map(|option| self.expression(option));
        let lowered = match builtin {
            Builtin::Sorted => {
                let copy = self.call_runtime(RuntimeFunction::ListCopy, vec![list.value], span);
                self.sort(copy.value, option, span);
                copy
            }
            Builtin::Sum if iterable.ty == Type::List(Box::new(Type::Float)) => {
                let start = match option {
                    Some(start) => start.value,
                    None => self.constant(Constant::Float(0.0), span),
                };
                self.call_runtime(RuntimeFunction::ListSumFloat, vec![list.value, start], span)
            }
            Builtin::Sum => {
                let start = match option {
                    Some(start) => start.value,
                    None => self.constant(Constant::Int("0".to_owned()), span),
                };
                self.call_runtime(RuntimeFunction::ListSumInt, vec![list.value, start], span)
            }
            Builtin::Any => self.call_runtime(RuntimeFunction::ListAny, vec![list.value], span),
            Builtin::All => self.call_runtime(RuntimeFunction::ListAll, vec![list.value], span),
            Builtin::Max | Builtin::Min => {
                self.list_extreme(builtin == Builtin::Max, list.value, option, result, span)
            }
            other => unreachable!("`{}` does not take one iterable", other.name()),
        };
        if let Some(option) = option {
            self.release(option, span);
        }
        self.release(list, span);
        lowered
    }

    /// Sorts a list in place, in reverse when `reverse` is given and true.
    pub(crate) fn sort(&mut self, list: ValueId, reverse: Option<Lowered>, span: Span) {
        let reverse = match reverse {
            Some(reverse) => self.int_of(reverse, span).value,
            None => self.constant(Constant::Int("0".to_owned()), span),
        };
        self.call_runtime(RuntimeFunction::ListSort, vec![list, reverse], span);
    }

    /// Returns the largest or smallest item of a list, with a reference of its own; with a
    /// default, an empty list gives the default instead of raising `ValueError`.
    fn list_extreme(
        &mut self,
        largest: bool,
        list: ValueId,
        default: Option<Lowered>,
        item: TyirType,
        span: Span,
    ) -> Lowered {
        let flag = self.constant(Constant::Int(u8::from(largest).to_string()), span);
        let Some(default) = default else {
            return self.extreme_item(list, flag, item, span);
        };
        let length = self.call_runtime(RuntimeFunction::ListLength, vec![list], span);
        let zero = self.constant(Constant::Int("0".to_owned()), span);
        let empty = self.builder.push_value(
            Operation::Compare {
                kind: ScalarKind::Int,
                comparison: Comparison::Equal,
                left: length.value,
                right: zero,
            },
            TyirType::Bool,
            span,
        );
        let use_default = self.builder.create_block();
        let search = self.builder.create_block();
        let join = self.builder.create_block();
        let joined = self.builder.add_block_parameter(join, item);
        self.builder.terminate(Terminator::Branch {
            condition: empty,
            then: use_default,
            otherwise: search,
        });
        self.builder.switch_to(use_default);
        if self.counted(item) {
            self.builder.push(Operation::Increment(default.value), span);
        }
        self.builder.terminate(Terminator::Jump {
            target: join,
            arguments: vec![default.value],
        });
        self.builder.switch_to(search);
        let found = self.extreme_value(list, flag, item, span);
        self.builder.terminate(Terminator::Jump {
            target: join,
            arguments: vec![found],
        });
        self.builder.switch_to(join);
        self.produced(joined, item)
    }

    fn extreme_item(
        &mut self,
        list: ValueId,
        flag: ValueId,
        item: TyirType,
        span: Span,
    ) -> Lowered {
        let value = self.extreme_value(list, flag, item, span);
        self.produced(value, item)
    }

    /// Returns the item `min` or `max` finds, which the runtime gave a reference of its own.
    fn extreme_value(
        &mut self,
        list: ValueId,
        flag: ValueId,
        item: TyirType,
        span: Span,
    ) -> ValueId {
        let bits = self.call_runtime(RuntimeFunction::ListExtreme, vec![list, flag], span);
        self.builder.push_value(
            Operation::FromBits {
                bits: bits.value,
                ty: item,
            },
            item,
            span,
        )
    }
}
