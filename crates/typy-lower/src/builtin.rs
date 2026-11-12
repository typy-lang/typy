//! Lowers the builtins that take positional values (LIB-01, LIB-02), including `print`.

use typy_check::{Builtin, PrintCall, Type, TypedExpr};
use typy_diagnostics::Span;
use typy_tyir::{
    Comparison, Constant, Operation, RuntimeFunction, ScalarKind, Terminator, TyirType,
};

use crate::function::FunctionLowerer;
use crate::ownership::Lowered;

impl FunctionLowerer<'_> {
    /// Returns the runtime function that gives the length of a `bytes`, set, dict, view or
    /// range.
    fn length_function(ty: TyirType) -> Option<RuntimeFunction> {
        Some(match ty {
            TyirType::BYTES => RuntimeFunction::BytesLength,
            TyirType::SET => RuntimeFunction::SetLength,
            TyirType::DICT => RuntimeFunction::DictLength,
            TyirType::VIEW => RuntimeFunction::DictViewLength,
            TyirType::RANGE => RuntimeFunction::RangeLength,
            _ => return None,
        })
    }

    /// Makes a range value of one to three bounds, which start at 0 and step by 1 when not
    /// given, as `range()` does (DAT-17).
    fn range_value(&mut self, bounds: &[Lowered], span: Span) -> Lowered {
        let mut words: Vec<_> = bounds.iter().map(|bound| bound.value).collect();
        if words.len() == 1 {
            words.insert(0, self.constant(Constant::Int("0".to_owned()), span));
        }
        if words.len() == 2 {
            words.push(self.constant(Constant::Int("1".to_owned()), span));
        }
        self.call_runtime(RuntimeFunction::RangeNew, words, span)
    }

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
            (Builtin::Len, [tuple]) if matches!(tuple.ty, TyirType::Tuple(_)) => {
                let length = self.tuple_length(tuple.ty).to_string();
                Self::borrowed(self.constant(Constant::Int(length), span), TyirType::Int)
            }
            (Builtin::Len, [list]) if list.ty == TyirType::LIST => self.list_length(*list, span),
            (Builtin::Range, bounds) => self.range_value(bounds, span),
            (Builtin::Len, [sized]) if let Some(function) = Self::length_function(sized.ty) => {
                self.call_runtime(function, vec![sized.value], span)
            }
            (Builtin::Bytes, [count]) if count.ty == TyirType::Int => {
                self.call_runtime(RuntimeFunction::BytesZeros, vec![count.value], span)
            }
            (Builtin::Bytes, [values]) if values.ty == TyirType::LIST => {
                self.call_runtime(RuntimeFunction::BytesFromList, vec![values.value], span)
            }
            (Builtin::Bytes, [data]) => self.keep_or_copy(*data, span, |_, data| data),
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
            (Builtin::Round, [number]) if number.ty == TyirType::Float => {
                self.call_runtime(RuntimeFunction::FloatRound, vec![number.value], span)
            }
            (Builtin::Round, [number]) => self.keep_or_copy(*number, span, |_, number| number),
            (Builtin::Round, [number, digits]) => {
                let function = if number.ty == TyirType::Float {
                    RuntimeFunction::FloatRoundDigits
                } else {
                    RuntimeFunction::IntRoundDigits
                };
                self.call_runtime(function, vec![number.value, digits.value], span)
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

    /// `print(values, sep=sep, end=end, file=file, flush=flush)`: every argument is evaluated
    /// in source order first, then each value is converted and written.
    pub(crate) fn print(&mut self, print: &PrintCall, span: Span) -> Lowered {
        let lowered: Vec<Lowered> = print
            .values
            .iter()
            .map(|value| self.expression(value))
            .collect();
        let mut keywords: Vec<(&str, &TypedExpr)> = [
            ("sep", print.sep.as_ref()),
            ("end", print.end.as_ref()),
            ("file", print.file.as_ref()),
            ("flush", print.flush.as_ref()),
        ]
        .into_iter()
        .filter_map(|(name, value)| Some((name, value?)))
        .collect();
        keywords.sort_by_key(|(_, value)| value.span.start());
        let mut separator = None;
        let mut ending = None;
        let mut file = None;
        let mut flush = None;
        for (name, value) in keywords {
            let evaluated = Some(self.expression(value));
            match name {
                "sep" => separator = evaluated,
                "end" => ending = evaluated,
                "file" => file = evaluated,
                _ => flush = evaluated,
            }
        }
        // Without a file, `print` writes to standard output through its buffer.
        let target = file.map(|file| file.value);
        let separator_value = match separator {
            Some(value) => value.value,
            None => self.string(" ", span),
        };
        for (position, value) in lowered.iter().enumerate() {
            if position > 0 {
                self.write_text(target, separator_value, span);
            }
            let text = self.string_of(*value, span);
            self.write_text(target, text.value, span);
            self.release(text, span);
        }
        let ending_value = match ending {
            Some(value) => value.value,
            None => self.string("\n", span),
        };
        self.write_text(target, ending_value, span);
        if let Some(flush) = flush {
            self.flush_if(target, flush.value, span);
        }
        for value in ending
            .into_iter()
            .chain(separator)
            .chain(file)
            .chain(flush)
            .chain(lowered.into_iter().rev())
        {
            self.release(value, span);
        }
        Self::borrowed(self.constant(Constant::Unit, span), TyirType::Unit)
    }

    /// Writes text to a text file, or to standard output when there is none.
    fn write_text(
        &mut self,
        file: Option<typy_tyir::ValueId>,
        text: typy_tyir::ValueId,
        span: Span,
    ) {
        match file {
            Some(file) => {
                self.call_runtime(RuntimeFunction::TextWrite, vec![file, text], span);
            }
            None => {
                self.call_runtime(RuntimeFunction::WriteStdout, vec![text], span);
            }
        }
    }

    /// Writes out what a text file holds, standard output when there is none, when `flush`
    /// is true.
    fn flush_if(
        &mut self,
        file: Option<typy_tyir::ValueId>,
        flush: typy_tyir::ValueId,
        span: Span,
    ) {
        let yes = self.builder.create_block();
        let after = self.builder.create_block();
        self.builder.terminate(Terminator::Branch {
            condition: flush,
            then: yes,
            otherwise: after,
        });
        self.builder.switch_to(yes);
        let file = match file {
            Some(file) => file,
            None => {
                self.call_runtime(RuntimeFunction::SysStdout, Vec::new(), span)
                    .value
            }
        };
        self.call_runtime(RuntimeFunction::TextFlush, vec![file], span);
        self.jump_to(after);
        self.builder.switch_to(after);
    }
}
