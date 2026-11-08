//! Lowers indexes of strings, and slices of strings and lists (DAT-05, DAT-12). The runtime
//! receives each slice bound as an `int` word, and a mask of the bounds written, so a missing
//! bound takes Python's default.

use typy_check::TypedExpr;
use typy_diagnostics::Span;
use typy_tyir::{Constant, RuntimeFunction, TyirType};

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
        let result = self.call_runtime(
            RuntimeFunction::StrIndex,
            vec![text.value, position.value],
            span,
        );
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
        let function = if ty == TyirType::STR {
            RuntimeFunction::StrSlice
        } else {
            RuntimeFunction::ListSlice
        };
        let result = self.call_runtime(function, arguments, span);
        for bound in lowered.into_iter().flatten() {
            self.release(bound, span);
        }
        self.release(sliced, span);
        result
    }
}
