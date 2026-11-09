//! Lowers the methods of `str` (LIB-03). An argument a call leaves out takes Python's default;
//! `start` and `end`, and a `str` that may be `None`, are passed with flag bits saying which
//! are given, as slices pass their bounds.

use typy_check::Method;
use typy_diagnostics::Span;
use typy_tyir::{Constant, Operation, RuntimeFunction, TyirType, ValueId};

use crate::function::FunctionLowerer;
use crate::ownership::Lowered;

impl FunctionLowerer<'_> {
    /// Calls a method of `str` on `text` with the arguments given for each parameter.
    pub(crate) fn str_method(
        &mut self,
        text: ValueId,
        method: Method,
        given: &[Option<Lowered>],
        result: TyirType,
        span: Span,
    ) -> Lowered {
        let argument = |index: usize| given.get(index).copied().flatten();
        match method {
            Method::StrFind
            | Method::StrRfind
            | Method::StrIndex
            | Method::StrRindex
            | Method::StrCount
            | Method::StrStartswith
            | Method::StrEndswith => self.str_search(text, method, given, span),
            Method::StrSplit | Method::StrRsplit => {
                let (separator, mut flags) = self.optional_str(argument(0), span);
                if method == Method::StrRsplit {
                    flags |= 2;
                }
                let maxsplit = self.int_or(argument(1), -1, span);
                let flags = self.int(flags, span);
                self.call_runtime(
                    RuntimeFunction::StrSplit,
                    vec![text, separator, maxsplit, flags],
                    span,
                )
            }
            Method::StrSplitlines => {
                let keep = match argument(0) {
                    Some(keep) => self.int_of(keep, span).value,
                    None => self.int(0, span),
                };
                self.call_runtime(RuntimeFunction::StrSplitlines, vec![text, keep], span)
            }
            Method::StrPartition | Method::StrRpartition => {
                let backwards = self.int(i64::from(method == Method::StrRpartition), span);
                let separator = required(argument(0));
                let parts = self.call_runtime(
                    RuntimeFunction::StrPartition,
                    vec![text, separator, backwards],
                    span,
                );
                self.three_parts(parts, result, span)
            }
            Method::StrJoin => {
                let parts = required(argument(0));
                self.call_runtime(RuntimeFunction::StrJoin, vec![text, parts], span)
            }
            Method::StrStrip | Method::StrLstrip | Method::StrRstrip => {
                let (chars, flags) = self.optional_str(argument(0), span);
                let sides = match method {
                    Method::StrLstrip => 2,
                    Method::StrRstrip => 4,
                    _ => 6,
                };
                let flags = self.int(flags | sides, span);
                self.call_runtime(RuntimeFunction::StrStrip, vec![text, chars, flags], span)
            }
            Method::StrRemoveprefix | Method::StrRemovesuffix => {
                let suffix = self.int(i64::from(method == Method::StrRemovesuffix), span);
                let affix = required(argument(0));
                self.call_runtime(
                    RuntimeFunction::StrRemoveAffix,
                    vec![text, affix, suffix],
                    span,
                )
            }
            Method::StrReplace => {
                let (old, new) = (required(argument(0)), required(argument(1)));
                let count = self.int_or(argument(2), -1, span);
                self.call_runtime(
                    RuntimeFunction::StrReplace,
                    vec![text, old, new, count],
                    span,
                )
            }
            Method::StrCenter | Method::StrLjust | Method::StrRjust => {
                let mode = match method {
                    Method::StrCenter => 0,
                    Method::StrLjust => 1,
                    _ => 2,
                };
                let width = required(argument(0));
                let fill = match argument(1) {
                    Some(fill) => fill.value,
                    None => self.string(" ", span),
                };
                let mode = self.int(mode, span);
                self.call_runtime(RuntimeFunction::StrPad, vec![text, width, fill, mode], span)
            }
            Method::StrZfill => {
                let width = required(argument(0));
                self.call_runtime(RuntimeFunction::StrZfill, vec![text, width], span)
            }
            Method::StrExpandtabs => {
                let size = self.int_or(argument(0), 8, span);
                self.call_runtime(RuntimeFunction::StrExpandtabs, vec![text, size], span)
            }
            Method::StrIsascii => self.call_runtime(RuntimeFunction::StrIsascii, vec![text], span),
            other => unreachable!("`{}` is not a method of `str`", other.name()),
        }
    }

    /// Calls a search, `find` to `count`, or a match, `startswith` or `endswith`.
    fn str_search(
        &mut self,
        text: ValueId,
        method: Method,
        given: &[Option<Lowered>],
        span: Span,
    ) -> Lowered {
        let argument = |index: usize| given.get(index).copied().flatten();
        let Some(target) = argument(0) else {
            unreachable!("the checker gives a search what it looks for")
        };
        let (function, flags) = match method {
            Method::StrStartswith | Method::StrEndswith => {
                let function = if target.ty == TyirType::LIST {
                    RuntimeFunction::StrMatchAny
                } else {
                    RuntimeFunction::StrMatch
                };
                (function, if method == Method::StrEndswith { 4 } else { 0 })
            }
            Method::StrFind => (RuntimeFunction::StrSearch, 0),
            Method::StrRfind => (RuntimeFunction::StrSearch, 1 << 2),
            Method::StrIndex => (RuntimeFunction::StrSearch, 2 << 2),
            Method::StrRindex => (RuntimeFunction::StrSearch, 3 << 2),
            _ => (RuntimeFunction::StrSearch, 4 << 2),
        };
        let (start, end, flags) = self.bounds(argument(1), argument(2), flags, span);
        self.call_runtime(function, vec![text, target.value, start, end, flags], span)
    }

    /// Returns the `int` word of a constant.
    fn int(&mut self, value: i64, span: Span) -> ValueId {
        self.constant(Constant::Int(value.to_string()), span)
    }

    /// Returns the argument, or the `int` default.
    fn int_or(&mut self, argument: Option<Lowered>, default: i64, span: Span) -> ValueId {
        match argument {
            Some(argument) => argument.value,
            None => self.int(default, span),
        }
    }

    /// Returns `start`, `end`, and the flags word with the bits of the bounds given added to
    /// `flags`.
    fn bounds(
        &mut self,
        start: Option<Lowered>,
        end: Option<Lowered>,
        flags: i64,
        span: Span,
    ) -> (ValueId, ValueId, ValueId) {
        let mask = flags | i64::from(start.is_some()) | (i64::from(end.is_some()) << 1);
        let start = self.int_or(start, 0, span);
        let end = self.int_or(end, 0, span);
        (start, end, self.int(mask, span))
    }

    /// Returns a `str` argument that may be `None` or left out, with flag bit 0 set when it is
    /// a `str`; otherwise an empty string stands in for it.
    fn optional_str(&mut self, argument: Option<Lowered>, span: Span) -> (ValueId, i64) {
        match argument {
            Some(argument) if argument.ty == TyirType::STR => (argument.value, 1),
            _ => (self.string("", span), 0),
        }
    }

    /// Returns a tuple of three strings from the new list of three the runtime gives, and
    /// releases the list.
    fn three_parts(&mut self, parts: Lowered, result: TyirType, span: Span) -> Lowered {
        let TyirType::Tuple(id) = result else {
            unreachable!("`partition` gives a tuple")
        };
        let items: Vec<ValueId> = (0..3)
            .map(|index| {
                let position = self.int(index, span);
                let list = Self::borrowed(parts.value, TyirType::LIST);
                let index = Self::borrowed(position, TyirType::Int);
                let item = self.list_item(list, index, TyirType::STR, span);
                self.take_reference(item, span)
            })
            .collect();
        let tuple = self
            .builder
            .push_value(Operation::MakeTuple { ty: id, items }, result, span);
        let tuple = self.produced(tuple, result);
        self.release(parts, span);
        tuple
    }
}

/// Returns the value of an argument the checker requires.
fn required(argument: Option<Lowered>) -> ValueId {
    match argument {
        Some(argument) => argument.value,
        None => unreachable!("the checker gives every required argument"),
    }
}
