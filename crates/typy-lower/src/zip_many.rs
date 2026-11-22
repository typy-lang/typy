//! Lowers `zip(*iterables)` (EXP-22, ADR-0083): the runtime makes an iterator of each iterable
//! of the list and steps them together, each item a new `tuple[T, ...]`.

use typy_check::{Type, TypedExpr};
use typy_diagnostics::Span;
use typy_tyir::{Constant, RuntimeFunction, TyirType};

use crate::function::FunctionLowerer;
use crate::ownership::Lowered;

impl FunctionLowerer<'_> {
    /// Returns the zip of the list `iterables`, an iterator of type `ty`, whose iterables of
    /// different lengths raise when `strict` is true.
    pub(crate) fn zip_many(
        &mut self,
        iterables: &TypedExpr,
        strict: Option<&TypedExpr>,
        ty: &Type,
        span: Span,
    ) -> Lowered {
        // The iterator gives `tuple[T, ...]`, whose items are `T`.
        let item = ty
            .iterable_item()
            .and_then(|tuple| tuple.iterable_item())
            .map_or(TyirType::Int, |item| self.lowered_type(&item));
        let list = self.expression(iterables);
        let strict = match strict {
            Some(strict) => self.expression(strict),
            None => Self::borrowed(self.constant(Constant::Bool(false), span), TyirType::Bool),
        };
        let (kind, descriptor) = self.item_layout(item, span);
        let zip = self.call_runtime(
            RuntimeFunction::ZipMany,
            vec![list.value, kind, descriptor, strict.value],
            span,
        );
        self.release(list, span);
        zip
    }
}
