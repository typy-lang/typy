//! Lowers indexes of strings, and slices of strings and lists (DAT-05, DAT-12). The runtime
//! receives each slice bound as an `int` word, and a mask of the bounds written, so a missing
//! bound takes Python's default.

use typy_check::{TypedExpr, TypedExprKind};
use typy_diagnostics::Span;
use typy_tyir::{Constant, Operation, RuntimeFunction, TyirType};

use crate::function::FunctionLowerer;
use crate::ownership::Lowered;

/// The mask bits of the start, the stop and the step, as the runtime numbers them.
const MASK_BITS: [i64; 3] = [1, 2, 4];

impl FunctionLowerer<'_> {
    /// Returns `text[index]`, a `str` of one code point; both operands are released.
    pub(crate) fn str_index(
        &mut self,
        value: &TypedExpr,
        index: &TypedExpr,
        span: Span,
    ) -> Lowered {
        let text = self.expression(value);
        let position = self.expression(index);
        let function = if text.ty == TyirType::BYTES {
            RuntimeFunction::BytesGet
        } else {
            RuntimeFunction::StrIndex
        };
        let result = self.call_runtime(function, vec![text.value, position.value], span);
        self.release(position, span);
        self.release(text, span);
        result
    }

    /// Returns `value[lower:upper:step]`, a new `str` or list; the bounds are evaluated in
    /// order after the value, and everything is released afterwards.
    pub(crate) fn slice(
        &mut self,
        value: &TypedExpr,
        bounds: [Option<&TypedExpr>; 3],
        ty: TyirType,
        span: Span,
    ) -> Lowered {
        let function = match ty {
            TyirType::STR => RuntimeFunction::StrSlice,
            TyirType::BYTES => RuntimeFunction::BytesSlice,
            _ => RuntimeFunction::ListSlice,
        };
        self.call_with_slice(function, value, bounds, span)
    }

    /// Calls `function` with `value`, each bound as an `int` word, 0 when missing, and the
    /// mask of the bounds written; the bounds are evaluated in order after the value, and
    /// everything is released afterwards.
    pub(crate) fn call_with_slice(
        &mut self,
        function: RuntimeFunction,
        value: &TypedExpr,
        bounds: [Option<&TypedExpr>; 3],
        span: Span,
    ) -> Lowered {
        let sliced = self.expression(value);
        let mut mask = 0;
        let mut lowered = Vec::with_capacity(3);
        for (bound, bit) in bounds.into_iter().zip(MASK_BITS) {
            lowered.push(bound.map(|bound| {
                mask |= bit;
                self.expression(bound)
            }));
        }
        let mut arguments = vec![sliced.value];
        for bound in &lowered {
            let word = match bound {
                Some(bound) => bound.value,
                None => self.constant(Constant::Int("0".to_owned()), span),
            };
            arguments.push(word);
        }
        arguments.push(self.constant(Constant::Int(mask.to_string()), span));
        let result = self.call_runtime(function, arguments, span);
        for bound in lowered.into_iter().flatten() {
            self.release(bound, span);
        }
        self.release(sliced, span);
        result
    }

    /// Lowers one target of `del`: a list item, a list slice or a dict key, which is removed
    /// and released (SMT-05).
    pub(crate) fn delete(&mut self, target: &TypedExpr, span: Span) {
        match &target.kind {
            TypedExprKind::ListItem { list, index } => {
                let list = self.expression(list);
                let index = self.expression(index);
                self.call_runtime(
                    RuntimeFunction::ListDelete,
                    vec![list.value, index.value],
                    span,
                );
                self.release(index, span);
                self.release(list, span);
            }
            TypedExprKind::Slice {
                value,
                lower,
                upper,
                step,
            } => {
                let bounds = [lower.as_deref(), upper.as_deref(), step.as_deref()];
                self.call_with_slice(RuntimeFunction::ListDeleteSlice, value, bounds, span);
            }
            TypedExprKind::DictItem { dict, key } => {
                let dict = self.expression(dict);
                let key = self.expression(key);
                let bits = self.bits(key, span);
                let value =
                    self.call_runtime(RuntimeFunction::DictPop, vec![dict.value, bits], span);
                // The dict gives up its reference to the value, which `del` drops.
                let ty = self.lowered_type(&target.ty);
                let dropped = self.builder.push_value(
                    Operation::FromBits {
                        bits: value.value,
                        ty,
                    },
                    ty,
                    span,
                );
                let owned = self.produced(dropped, ty);
                self.release(owned, span);
                self.release(key, span);
                self.release(dict, span);
            }
            _ => unreachable!("the checker allows `del` only of list items, slices and dict keys"),
        }
    }
}
