//! Lowers `dict` with keyword arguments and `dict.fromkeys`, with CPython's order of
//! evaluation: every argument first, then the source is read (DAT-16).

use typy_check::{LoopSource, Type, TypedExpr};
use typy_diagnostics::Span;
use typy_tyir::{Constant, RuntimeFunction, TyirType};

use crate::function::FunctionLowerer;
use crate::ownership::Lowered;

impl FunctionLowerer<'_> {
    /// Returns `dict(source, name=value, ...)`: an empty dict updated from the source, a dict
    /// merged or any other source's pairs set in order, then each name set to its value, as
    /// CPython's `dict_vectorcall` does once every argument is evaluated.
    pub(crate) fn dict_with(
        &mut self,
        source: Option<&LoopSource>,
        entries: &[(String, TypedExpr)],
        ty: &Type,
        span: Span,
    ) -> Lowered {
        let (key_type, value_type) = self.dict_types(ty);
        let dict = self.new_dict(key_type, value_type, 0, span);
        let keep = |lowerer: &mut Self| {
            entries
                .iter()
                .map(|(_, value)| {
                    let ty = lowerer.lowered_type(&value.ty);
                    let slot = lowerer.hidden_slot("keyword", ty);
                    let lowered = lowerer.expression(value);
                    lowerer.store(slot, lowered, span);
                    slot
                })
                .collect::<Vec<_>>()
        };
        let slots = match source {
            None => keep(self),
            Some(LoopSource::Items(sequence)) if matches!(sequence.ty, Type::Dict(..)) => {
                let other_type = self.lowered_type(&sequence.ty);
                let other = self.hidden_slot("source", other_type);
                let lowered = self.expression(sequence);
                self.store(other, lowered, span);
                let slots = keep(self);
                let other = self.load(other, span);
                let address = self.address(other.value, span);
                self.call_runtime(RuntimeFunction::DictUpdate, vec![dict.value, address], span);
                slots
            }
            Some(source) => self.each_value_after(source, span, keep, |lowerer, _, pair| {
                let key = lowerer.borrowed_item(pair, 0, key_type, span);
                let value = lowerer.borrowed_item(pair, 1, value_type, span);
                lowerer.set_entry(dict.value, key, value, span);
            }),
        };
        for ((name, _), slot) in entries.iter().zip(slots) {
            let key = Self::borrowed(self.string(name, span), TyirType::STR);
            let value = self.load(slot, span);
            self.set_entry(dict.value, key, value, span);
        }
        dict
    }

    /// Returns `dict.fromkeys(source, value)`: each item of the source, read after the value is
    /// evaluated, set as a key to the one value, or to `None`.
    pub(crate) fn dict_from_keys(
        &mut self,
        source: &LoopSource,
        value: Option<&TypedExpr>,
        ty: &Type,
        span: Span,
    ) -> Lowered {
        let (key_type, value_type) = self.dict_types(ty);
        let dict = self.new_dict(key_type, value_type, 0, span);
        let keep = |lowerer: &mut Self| {
            value.map(|value| {
                let slot = lowerer.hidden_slot("value", value_type);
                let lowered = lowerer.expression(value);
                lowerer.store(slot, lowered, span);
                slot
            })
        };
        self.each_value_after(source, span, keep, |lowerer, slot, key| {
            let value = match slot {
                Some(slot) => lowerer.load(*slot, span),
                None => Self::borrowed(lowerer.constant(Constant::Unit, span), TyirType::Unit),
            };
            lowerer.set_entry(dict.value, key, value, span);
        });
        dict
    }
}
