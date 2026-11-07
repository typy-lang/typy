//! Lowers the builtins of 0.1 (LIB-01), including `print`.

use typy_check::{Builtin, Type, TypedExpr};
use typy_diagnostics::Span;
use typy_tyir::{
    Comparison, Constant, Operation, RuntimeFunction, ScalarKind, Terminator, TyirType,
};

use crate::function::FunctionLowerer;
use crate::ownership::Lowered;

impl FunctionLowerer<'_> {
    /// Calls a builtin with arguments already evaluated in order; the arguments are released afterwards.
    pub(crate) fn builtin(
        &mut self,
        builtin: Builtin,
        values: Vec<Lowered>,
        types: &[Type],
        ty: TyirType,
        span: Span,
    ) -> Lowered {
        let result = match (builtin, values.as_slice()) {
            (Builtin::Int, []) => Self::borrowed(
                self.constant(Constant::Int("0".to_owned()), span),
                TyirType::Int,
            ),
            (Builtin::Float, []) => {
                Self::borrowed(self.constant(Constant::Float(0.0), span), TyirType::Float)
            }
            (Builtin::Str, []) => Self::borrowed(self.string("", span), TyirType::STR),
            (Builtin::Bool, []) => {
                Self::borrowed(self.constant(Constant::Bool(false), span), TyirType::Bool)
            }
            (Builtin::Int, [text, base]) => self.call_runtime(
                RuntimeFunction::IntFromStr,
                vec![text.value, base.value],
                span,
            ),
            (Builtin::Int, [value]) => {
                self.keep_or_copy(*value, span, |lowerer, value| lowerer.int_of(value, span))
            }
            (Builtin::Float, [value]) => {
                self.keep_or_copy(*value, span, |lowerer, value| lowerer.float_of(value, span))
            }
            (Builtin::Str, [value]) => self.keep_or_copy(*value, span, |lowerer, value| {
                lowerer.string_of(value, span)
            }),
            (Builtin::Repr, [value]) => {
                self.keep_or_copy(*value, span, |lowerer, value| lowerer.repr_of(value, span))
            }
            (Builtin::Bool, [value]) => {
                let truth = self.truth(*value, span);
                Self::borrowed(truth, TyirType::Bool)
            }
            (Builtin::Len, [text]) => {
                self.call_runtime(RuntimeFunction::StrLength, vec![text.value], span)
            }
            (Builtin::Abs, [value]) if value.ty == TyirType::Float => {
                self.call_runtime(RuntimeFunction::FloatAbsolute, vec![value.value], span)
            }
            (Builtin::Abs, [value]) => {
                self.call_runtime(RuntimeFunction::IntAbsolute, vec![value.value], span)
            }
            (Builtin::Ord, [text]) => {
                self.call_runtime(RuntimeFunction::Ord, vec![text.value], span)
            }
            (Builtin::Chr, [code]) => {
                self.call_runtime(RuntimeFunction::Chr, vec![code.value], span)
            }
            (Builtin::Min | Builtin::Max, _) => {
                self.extreme(builtin == Builtin::Max, &values, types, ty, span)
            }
            _ => unreachable!(
                "the checker refuses `{}` with {} arguments",
                builtin.name(),
                values.len()
            ),
        };
        for value in values.into_iter().rev() {
            self.release(value, span);
        }
        result
    }

    /// Runs a conversion; when it returns the argument itself, the result is counted again, because the argument is released afterwards.
    fn keep_or_copy(
        &mut self,
        value: Lowered,
        span: Span,
        convert: impl FnOnce(&mut Self, Lowered) -> Lowered,
    ) -> Lowered {
        let converted = convert(self, value);
        if converted.value == value.value && value.owned {
            self.builder.push(Operation::Increment(value.value), span);
            return self.produced(value.value, value.ty);
        }
        converted
    }

    /// `min` and `max` of two or more values: the first value wins a tie, as CPython keeps the first it found.
    fn extreme(
        &mut self,
        maximum: bool,
        values: &[Lowered],
        types: &[Type],
        ty: TyirType,
        span: Span,
    ) -> Lowered {
        let kind = match types.first() {
            Some(Type::Float) => ScalarKind::Float,
            Some(Type::Str) => ScalarKind::Str,
            _ => ScalarKind::Int,
        };
        let comparison = if maximum {
            Comparison::Greater
        } else {
            Comparison::Less
        };
        let best = self.hidden_slot(if maximum { "max" } else { "min" }, ty);
        self.store(best, Self::borrowed(values[0].value, ty), span);
        for candidate in &values[1..] {
            let current = self.load(best, span);
            let better = self.builder.push_value(
                Operation::Compare {
                    kind,
                    comparison,
                    left: candidate.value,
                    right: current.value,
                },
                TyirType::Bool,
                span,
            );
            let replace = self.builder.create_block();
            let keep = self.builder.create_block();
            self.builder.terminate(Terminator::Branch {
                condition: better,
                then: replace,
                otherwise: keep,
            });
            self.builder.switch_to(replace);
            self.store(best, Self::borrowed(candidate.value, ty), span);
            self.builder.terminate(Terminator::Jump {
                target: keep,
                arguments: Vec::new(),
            });
            self.builder.switch_to(keep);
        }
        let result = self.load(best, span);
        let owned = self.take_reference(result, span);
        self.produced(owned, ty)
    }

    /// `print(values, sep=sep, end=end)`: every argument is evaluated in source order first, then each value is converted and written.
    pub(crate) fn print(
        &mut self,
        values: &[TypedExpr],
        sep: Option<&TypedExpr>,
        end: Option<&TypedExpr>,
        span: Span,
    ) -> Lowered {
        let lowered: Vec<Lowered> = values.iter().map(|value| self.expression(value)).collect();
        let mut keywords: Vec<(&str, &TypedExpr)> = sep
            .map(|value| ("sep", value))
            .into_iter()
            .chain(end.map(|value| ("end", value)))
            .collect();
        keywords.sort_by_key(|(_, value)| value.span.start());
        let mut separator = None;
        let mut ending = None;
        for (name, value) in keywords {
            let evaluated = self.expression(value);
            if name == "sep" {
                separator = Some(evaluated);
            } else {
                ending = Some(evaluated);
            }
        }
        let separator_value = match separator {
            Some(value) => value.value,
            None => self.string(" ", span),
        };
        for (position, value) in lowered.iter().enumerate() {
            if position > 0 {
                self.call_runtime(RuntimeFunction::WriteStdout, vec![separator_value], span);
            }
            let text = self.string_of(*value, span);
            self.call_runtime(RuntimeFunction::WriteStdout, vec![text.value], span);
            self.release(text, span);
        }
        let ending_value = match ending {
            Some(value) => value.value,
            None => self.string("\n", span),
        };
        self.call_runtime(RuntimeFunction::WriteStdout, vec![ending_value], span);
        for value in ending
            .into_iter()
            .chain(separator)
            .chain(lowered.into_iter().rev())
        {
            self.release(value, span);
        }
        Self::borrowed(self.constant(Constant::Unit, span), TyirType::Unit)
    }
}
