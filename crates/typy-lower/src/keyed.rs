//! Lowers `sorted`, `list.sort`, `min` and `max` with a `key` (FUN-05): the key of each item
//! is called through its function value once, in order, and the items are ordered or compared
//! by their keys alone, as CPython orders them.

use typy_check::{KeyedOperation, TypedExpr};
use typy_diagnostics::Span;
use typy_tyir::{Callee, Constant, RuntimeFunction, TyirType, ValueId};

use crate::function::FunctionLowerer;
use crate::ownership::Lowered;

impl FunctionLowerer<'_> {
    /// Lowers `operation` of `list` by `key` with its option, `reverse` or `default`, which
    /// are evaluated in the order the call writes them.
    pub(crate) fn keyed(
        &mut self,
        operation: KeyedOperation,
        list: &TypedExpr,
        key: &TypedExpr,
        (option, option_first): (Option<&TypedExpr>, bool),
        span: Span,
    ) -> Lowered {
        let later: Vec<&TypedExpr> = std::iter::once(key).chain(option).collect();
        let items = self.operand_before(list, &later, span);
        let (option_value, function) = if option_first {
            let option = option.map(|option| self.expression(option));
            (option, self.expression(key))
        } else {
            let function = self.expression(key);
            (option.map(|option| self.expression(option)), function)
        };
        // A key may assign where its function value was read, so the call holds its own
        // reference to it while it runs.
        let function = self.kept_for_call(key, function, span);
        let item = self.lowered_type(list.ty.sequence_item().unwrap_or(&typy_check::Type::Error));
        let returns = key
            .ty
            .callable_parts()
            .map_or(TyirType::Unit, |(_, returns)| self.lowered_type(returns));
        let lowered = match operation {
            KeyedOperation::Sorted => {
                let copy = self.call_runtime(RuntimeFunction::ListCopy, vec![items.value], span);
                self.sort_by_keys(copy.value, function, (item, returns), option_value, span);
                copy
            }
            // The items are ordered in a copy and written back, so a key that raises leaves
            // the list as it was, as in CPython; what a key sees of the list while it is
            // sorted, CPython leaves undefined.
            KeyedOperation::Sort => {
                let copy = self.call_runtime(RuntimeFunction::ListCopy, vec![items.value], span);
                self.sort_by_keys(copy.value, function, (item, returns), option_value, span);
                self.call_runtime(RuntimeFunction::ListClear, vec![items.value], span);
                self.call_runtime(
                    RuntimeFunction::ListExtend,
                    vec![items.value, copy.value],
                    span,
                );
                self.release(copy, span);
                Self::borrowed(self.constant(Constant::Unit, span), TyirType::Unit)
            }
            KeyedOperation::Min | KeyedOperation::Max => {
                let largest = operation == KeyedOperation::Max;
                self.extreme_by_key(
                    largest,
                    items.value,
                    function,
                    (item, returns),
                    option_value,
                    span,
                )
            }
        };
        if let Some(option) = option_value {
            self.release(option, span);
        }
        self.release(function, span);
        self.release(items, span);
        lowered
    }

    /// Returns the key of `value` through `function`, with a reference of its own.
    fn key_of(
        &mut self,
        function: Lowered,
        value: Lowered,
        returns: TyirType,
        span: Span,
    ) -> Lowered {
        let key = self.invoke(
            Callee::Value,
            vec![function.value, value.value],
            returns,
            span,
        );
        self.produced(key, returns)
    }

    /// Sorts the list `list`, of items of type `item`, in place by the keys `function` gives,
    /// computed first, once for each item in order; in reverse when `reverse` is true.
    fn sort_by_keys(
        &mut self,
        list: ValueId,
        function: Lowered,
        (item, returns): (TyirType, TyirType),
        reverse: Option<Lowered>,
        span: Span,
    ) {
        let keys = self.list_display(&[], returns, span);
        self.each_list_item(list, item, span, |this, value| {
            let key = this.key_of(function, value, returns, span);
            let bits = this.bits(key, span);
            this.call_runtime(RuntimeFunction::ListAppend, vec![keys.value, bits], span);
            this.release(key, span);
        });
        let reverse = match reverse {
            Some(reverse) => self.int_of(reverse, span).value,
            None => self.constant(Constant::Int("0".to_owned()), span),
        };
        self.call_runtime(
            RuntimeFunction::ListSortKeyed,
            vec![list, keys.value, reverse],
            span,
        );
        self.release(keys, span);
    }

    /// Returns the first item of the smallest key, or the largest, of the list `list`, with
    /// a reference of its own: each item's key is compared with the best so far as soon as it
    /// is computed, as CPython's `min` and `max` compare it, and the runtime keeps the best key
    /// and its item. An empty list gives `default`, or raises `ValueError`.
    fn extreme_by_key(
        &mut self,
        largest: bool,
        list: ValueId,
        function: Lowered,
        (item, returns): (TyirType, TyirType),
        default: Option<Lowered>,
        span: Span,
    ) -> Lowered {
        let best_key = self.list_display(&[], returns, span);
        let best = self.list_display(&[], item, span);
        let flag = self.constant(Constant::Int(u8::from(largest).to_string()), span);
        self.each_list_item(list, item, span, |this, value| {
            let key = this.key_of(function, value, returns, span);
            let key_bits = this.bits(key, span);
            let item_bits = this.bits(value, span);
            this.call_runtime(
                RuntimeFunction::ListKeepExtreme,
                vec![best_key.value, key_bits, best.value, item_bits, flag],
                span,
            );
            this.release(key, span);
        });
        let found = self.list_extreme(largest, best.value, default, item, span);
        self.release(best, span);
        self.release(best_key, span);
        found
    }
}
