//! Lowers the methods of lists and of `tuple[T, ...]` (LIB-03, DAT-14). Each runtime list
//! function borrows its arguments; one that stores an item takes a reference of its own.

use typy_check::{Argument, Method, TypedExpr};
use typy_diagnostics::Span;
use typy_tyir::{Constant, Operation, RuntimeFunction, TyirType, ValueId};

use crate::function::FunctionLowerer;
use crate::ownership::Lowered;

impl FunctionLowerer<'_> {
    /// Calls `receiver.method(arguments)`, whose result has type `result`. The receiver is
    /// evaluated first, then the arguments in source order, and all are released afterwards.
    pub(crate) fn method(
        &mut self,
        receiver: &TypedExpr,
        method: Method,
        arguments: &[Argument],
        result: TyirType,
        span: Span,
    ) -> Lowered {
        let receiver = self.expression(receiver);
        // Every value in source order, and each parameter's value, which is the last given for
        // a parameter that takes many.
        let mut values: Vec<Lowered> = Vec::with_capacity(arguments.len());
        let mut given: Vec<Option<Lowered>> = Vec::new();
        for argument in arguments {
            let value = self.expression(&argument.value);
            values.push(value);
            if given.len() <= argument.parameter {
                given.resize(argument.parameter + 1, None);
            }
            given[argument.parameter] = Some(value);
        }
        let lowered = if receiver.ty == TyirType::STR || receiver.ty == TyirType::BYTES {
            let bytes = receiver.ty == TyirType::BYTES;
            self.str_method(receiver.value, bytes, method, &given, result, span)
        } else if receiver.ty == TyirType::SET {
            self.set_method(receiver.value, method, &values, result, span)
        } else {
            let arguments: Vec<Lowered> = given.iter().flatten().copied().collect();
            self.list_method(receiver.value, method, &arguments, result, span)
        };
        for value in values {
            self.release(value, span);
        }
        self.release(receiver, span);
        lowered
    }

    /// Calls a method of a list or a `tuple[T, ...]` with its arguments in parameter order.
    fn list_method(
        &mut self,
        list: ValueId,
        method: Method,
        arguments: &[Lowered],
        result: TyirType,
        span: Span,
    ) -> Lowered {
        match (method, arguments) {
            (Method::ListPop, given) => self.pop(list, given.first().copied(), result, span),
            (
                Method::ListAppend
                | Method::ListRemove
                | Method::ListIndex
                | Method::ListCount
                | Method::TupleIndex
                | Method::TupleCount,
                [item],
            ) => {
                // A `tuple[T, ...]` is a list object, so its `index` and `count` are the list's.
                let function = match method {
                    Method::ListAppend => RuntimeFunction::ListAppend,
                    Method::ListRemove => RuntimeFunction::ListRemove,
                    Method::ListIndex | Method::TupleIndex => RuntimeFunction::ListIndex,
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
            (Method::ListSort, given) => {
                self.sort(list, given.first().copied(), span);
                Self::borrowed(self.constant(Constant::Unit, span), TyirType::Unit)
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
        }
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
