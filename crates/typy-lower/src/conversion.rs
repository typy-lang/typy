//! Converts values to `str`, `int`, `float` and `bool`, as the builtins of the same names do (LIB-01).

use typy_diagnostics::Span;
use typy_tyir::{Constant, RuntimeFunction, Terminator, TyirType, TypeId, ValueId};

use crate::function::FunctionLowerer;
use crate::ownership::Lowered;

impl FunctionLowerer<'_> {
    /// Returns `str(value)`; the value itself is not released.
    pub(crate) fn string_of(&mut self, lowered: Lowered, span: Span) -> Lowered {
        match lowered.ty {
            TyirType::Int => {
                self.call_runtime(RuntimeFunction::IntToStr, vec![lowered.value], span)
            }
            TyirType::Float => {
                self.call_runtime(RuntimeFunction::FloatToStr, vec![lowered.value], span)
            }
            TyirType::Object(TypeId::LIST) => {
                self.call_runtime(RuntimeFunction::ListRepr, vec![lowered.value], span)
            }
            TyirType::Object(TypeId::BYTES) => {
                self.call_runtime(RuntimeFunction::BytesRepr, vec![lowered.value], span)
            }
            TyirType::Object(TypeId::SET) => {
                self.call_runtime(RuntimeFunction::SetRepr, vec![lowered.value], span)
            }
            TyirType::Object(TypeId::DICT) => {
                self.call_runtime(RuntimeFunction::DictRepr, vec![lowered.value], span)
            }
            TyirType::Object(TypeId::DICT_VIEW) => {
                self.call_runtime(RuntimeFunction::DictViewText, vec![lowered.value], span)
            }
            TyirType::Object(id) if id.class_index().is_some() => {
                self.instance_text(lowered, false, span)
            }
            TyirType::TYPE => {
                self.call_runtime(RuntimeFunction::TypeText, vec![lowered.value], span)
            }
            TyirType::EXCEPTION => self.exception_text(lowered, false, span),
            TyirType::TRACEBACK | TyirType::ITERATOR | TyirType::RANGE => {
                let object = self.bits(Self::borrowed(lowered.value, lowered.ty), span);
                self.call_runtime(RuntimeFunction::ObjectRepr, vec![object], span)
            }
            TyirType::Object(_) => Self::borrowed(lowered.value, lowered.ty),
            TyirType::Bits => unreachable!("list items are converted before they are written"),
            TyirType::Unit => {
                let value = self.string("None", span);
                Self::borrowed(value, TyirType::STR)
            }
            TyirType::Bool => {
                let yes = self.string("True", span);
                let no = self.string("False", span);
                let value = self.select(lowered.value, yes, no, TyirType::STR);
                Self::borrowed(value, TyirType::STR)
            }
            TyirType::Tuple(_) => self.tuple_text(lowered, span),
            TyirType::Optional(_) => self.optional_text(lowered, false, span),
        }
    }

    /// Returns `repr(value)`, which differs from `str` only for strings, and for tuples, whose
    /// `str` is their `repr`.
    pub(crate) fn repr_of(&mut self, lowered: Lowered, span: Span) -> Lowered {
        if lowered.ty == TyirType::STR {
            self.call_runtime(RuntimeFunction::StrRepr, vec![lowered.value], span)
        } else if lowered.ty == TyirType::EXCEPTION {
            self.exception_text(lowered, true, span)
        } else if let TyirType::Optional(_) = lowered.ty {
            self.optional_text(lowered, true, span)
        } else if let TyirType::Object(id) = lowered.ty
            && id.class_index().is_some()
        {
            self.instance_text(lowered, true, span)
        } else {
            self.string_of(lowered, span)
        }
    }

    /// Returns `int(value)` for a value of one argument; the value itself is not released.
    pub(crate) fn int_of(&mut self, lowered: Lowered, span: Span) -> Lowered {
        match lowered.ty {
            TyirType::Int => lowered,
            TyirType::Float => {
                self.call_runtime(RuntimeFunction::FloatToInt, vec![lowered.value], span)
            }
            TyirType::Bool => {
                let one = self.constant(Constant::Int("1".to_owned()), span);
                let zero = self.constant(Constant::Int("0".to_owned()), span);
                let value = self.select(lowered.value, one, zero, TyirType::Int);
                Self::borrowed(value, TyirType::Int)
            }
            TyirType::Object(_) => {
                let base = self.constant(Constant::Int("10".to_owned()), span);
                self.call_runtime(RuntimeFunction::IntFromStr, vec![lowered.value, base], span)
            }
            TyirType::Unit | TyirType::Tuple(_) | TyirType::Bits | TyirType::Optional(_) => {
                unreachable!("the checker refuses int() of None, a tuple and an optional value")
            }
        }
    }

    /// Returns `float(value)`; the value itself is not released.
    pub(crate) fn float_of(&mut self, lowered: Lowered, span: Span) -> Lowered {
        match lowered.ty {
            TyirType::Float => lowered,
            TyirType::Int => {
                self.call_runtime(RuntimeFunction::IntToFloat, vec![lowered.value], span)
            }
            TyirType::Bool => {
                let one = self.constant(Constant::Float(1.0), span);
                let zero = self.constant(Constant::Float(0.0), span);
                let value = self.select(lowered.value, one, zero, TyirType::Float);
                Self::borrowed(value, TyirType::Float)
            }
            TyirType::Object(_) => {
                self.call_runtime(RuntimeFunction::FloatFromStr, vec![lowered.value], span)
            }
            TyirType::Unit | TyirType::Tuple(_) | TyirType::Bits | TyirType::Optional(_) => {
                unreachable!("the checker refuses float() of None, a tuple and an optional value")
            }
        }
    }

    /// Returns `then` when `condition` is true and `otherwise` when it is false; both are already computed.
    pub(crate) fn select(
        &mut self,
        condition: ValueId,
        then: ValueId,
        otherwise: ValueId,
        ty: TyirType,
    ) -> ValueId {
        let yes = self.builder.create_block();
        let no = self.builder.create_block();
        let join = self.builder.create_block();
        let result = self.builder.add_block_parameter(join, ty);
        self.builder.terminate(Terminator::Branch {
            condition,
            then: yes,
            otherwise: no,
        });
        self.builder.switch_to(yes);
        self.builder.terminate(Terminator::Jump {
            target: join,
            arguments: vec![then],
        });
        self.builder.switch_to(no);
        self.builder.terminate(Terminator::Jump {
            target: join,
            arguments: vec![otherwise],
        });
        self.builder.switch_to(join);
        result
    }
}
