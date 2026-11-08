//! Lowers the methods of lists (LIB-03). Each runtime list function borrows its arguments;
//! one that stores an item takes a reference of its own.

use typy_check::{Method, TypedExpr};
use typy_diagnostics::Span;
use typy_tyir::{Constant, Operation, RuntimeFunction, TyirType};

use crate::function::FunctionLowerer;
use crate::ownership::Lowered;

impl FunctionLowerer<'_> {
    /// Calls `receiver.method(arguments)`, whose result has type `result`. The receiver is
    /// evaluated first, then the arguments in order, and all are released afterwards.
    pub(crate) fn method(
        &mut self,
        receiver: &TypedExpr,
        method: Method,
        arguments: &[TypedExpr],
        result: TyirType,
        span: Span,
    ) -> Lowered {
        let receiver = self.expression(receiver);
        let arguments: Vec<Lowered> = arguments
            .iter()
            .map(|argument| self.expression(argument))
            .collect();
        let list = receiver.value;
        let lowered = match (method, arguments.as_slice()) {
            (Method::ListPop, given) => self.pop(list, given.first().copied(), result, span),
            (
                Method::ListAppend | Method::ListRemove | Method::ListIndex | Method::ListCount,
                [item],
            ) => {
                let function = match method {
                    Method::ListAppend => RuntimeFunction::ListAppend,
                    Method::ListRemove => RuntimeFunction::ListRemove,
                    Method::ListIndex => RuntimeFunction::ListIndex,
                    _ => RuntimeFunction::ListCount,
                };
                let bits = self.bits(*item, span);
                self.call_runtime(function, vec![list, bits], span)
            }
            (Method::ListInsert, [index, item]) => {
                let bits = self.bits(*item, span);
                self.call_runtime(
                    RuntimeFunction::ListInsert,
                    vec![list, index.value, bits],
                    span,
                )
            }
            (Method::ListExtend, [other]) => {
                self.call_runtime(RuntimeFunction::ListExtend, vec![list, other.value], span)
            }
            (Method::ListClear | Method::ListCopy | Method::ListReverse, []) => {
                let function = match method {
                    Method::ListClear => RuntimeFunction::ListClear,
                    Method::ListCopy => RuntimeFunction::ListCopy,
                    _ => RuntimeFunction::ListReverse,
                };
                self.call_runtime(function, vec![list], span)
            }
            (method, _) => unreachable!("the checker gives `{}` its arguments", method.name()),
        };
        for argument in arguments {
            self.release(argument, span);
        }
        self.release(receiver, span);
        lowered
    }

    /// Returns `list.pop()` or `list.pop(index)`. The list gives up its reference to the item,
    /// so the result is owned.
    fn pop(
        &mut self,
        list: typy_tyir::ValueId,
        index: Option<Lowered>,
        item: TyirType,
        span: Span,
    ) -> Lowered {
        let (index, present) = match index {
            Some(index) => (index.value, "1"),
            None => (self.constant(Constant::Int("0".to_owned()), span), "0"),
        };
        let present = self.constant(Constant::Int(present.to_owned()), span);
        let bits = self.call_runtime(RuntimeFunction::ListPop, vec![list, index, present], span);
        let value = self.builder.push_value(
            Operation::FromBits {
                bits: bits.value,
                ty: item,
            },
            item,
            span,
        );
        self.produced(value, item)
    }
}
