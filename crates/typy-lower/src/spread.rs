//! Lowers displays that unpack (EXP-17): a list, a set or a tuple made empty, then each item
//! added and each value of an iterable unpacked with `*` added as a `for` loop reads it, in
//! order; a dict made empty, then each entry set and each dict unpacked with `**` merged.

use typy_check::{DictPart, SpreadKind, SpreadPart, Type};
use typy_diagnostics::Span;
use typy_tyir::RuntimeFunction;

use crate::function::FunctionLowerer;
use crate::ownership::Lowered;

impl FunctionLowerer<'_> {
    /// Lowers a list, set or tuple display with a `*value` item, of type `ty`.
    pub(crate) fn spread(
        &mut self,
        kind: SpreadKind,
        parts: &[SpreadPart],
        ty: &Type,
        span: Span,
    ) -> Lowered {
        let item = match ty {
            Type::List(item) | Type::Set(item) | Type::TupleOf(item) => self.lowered_type(item),
            _ => unreachable!("a display with `*` is a list, a set or a tuple"),
        };
        let collection = if kind == SpreadKind::Set {
            self.new_set(item, span)
        } else {
            self.list_display(&[], item, span)
        };
        let add = if kind == SpreadKind::Set {
            RuntimeFunction::SetAdd
        } else {
            RuntimeFunction::ListAppend
        };
        for part in parts {
            match part {
                SpreadPart::Item(value) => {
                    let value = self.expression(value);
                    let bits = self.bits(value, span);
                    self.call_runtime(add, vec![collection.value, bits], span);
                    self.release(value, span);
                }
                SpreadPart::Each(source) => {
                    self.each_value(source, span, |lowerer, value| {
                        let bits = lowerer.bits(value, span);
                        lowerer.call_runtime(add, vec![collection.value, bits], span);
                    });
                }
            }
        }
        if kind == SpreadKind::Tuple {
            self.call_runtime(RuntimeFunction::TupleSeal, vec![collection.value], span);
        }
        collection
    }

    /// Lowers a dict display with a `**mapping` item, of type `ty`.
    pub(crate) fn dict_spread(&mut self, parts: &[DictPart], ty: &Type, span: Span) -> Lowered {
        let (key_type, value_type) = self.dict_types(ty);
        let dict = self.new_dict(key_type, value_type, 0, span);
        for part in parts {
            match part {
                DictPart::Entry(key, value) => {
                    let key = self.operand_before(key, &[value], span);
                    let value = self.expression(value);
                    self.set_entry(dict.value, key, value, span);
                    self.release(value, span);
                    self.release(key, span);
                }
                DictPart::Merge(other) => {
                    let other = self.expression(other);
                    self.dict_update(dict.value, other.value, span);
                    self.release(other, span);
                }
            }
        }
        dict
    }
}
