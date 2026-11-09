//! Lowers the methods of `str`, and `bytes.decode` (LIB-03). An argument a call leaves out takes Python's default;
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
        bytes: bool,
        method: Method,
        given: &[Option<Lowered>],
        result: TyirType,
        span: Span,
    ) -> Lowered {
        if let Some(mode) = case_mode(method) {
            return self.str_case(text, bytes, mode, span);
        }
        if let Some(question) = test_question(method) {
            return self.str_test(text, bytes, question, span);
        }
        let argument = |index: usize| given.get(index).copied().flatten();
        match method {
            Method::StrFind
            | Method::StrRfind
            | Method::StrIndex
            | Method::StrRindex
            | Method::StrCount
            | Method::StrStartswith
            | Method::StrEndswith => self.str_search(text, bytes, method, given, span),
            Method::StrSplit
            | Method::StrRsplit
            | Method::StrSplitlines
            | Method::StrPartition
            | Method::StrRpartition
            | Method::StrJoin => self.str_split(text, bytes, method, given, result, span),
            Method::StrStrip | Method::StrLstrip | Method::StrRstrip => {
                let (chars, flags) = self.optional_str(argument(0), bytes, span);
                let sides = match method {
                    Method::StrLstrip => 2,
                    Method::StrRstrip => 4,
                    _ => 6,
                };
                let flags = self.int(flags | sides, span);
                self.text_call(
                    bytes,
                    RuntimeFunction::StrStrip,
                    vec![text, chars, flags],
                    span,
                )
            }
            Method::StrRemoveprefix | Method::StrRemovesuffix => {
                let suffix = self.int(i64::from(method == Method::StrRemovesuffix), span);
                let affix = required(argument(0));
                self.text_call(
                    bytes,
                    RuntimeFunction::StrRemoveAffix,
                    vec![text, affix, suffix],
                    span,
                )
            }
            Method::StrReplace => {
                let (old, new) = (required(argument(0)), required(argument(1)));
                let count = self.int_or(argument(2), -1, span);
                self.text_call(
                    bytes,
                    RuntimeFunction::StrReplace,
                    vec![text, old, new, count],
                    span,
                )
            }
            Method::StrCenter | Method::StrLjust | Method::StrRjust => {
                self.str_pad(text, bytes, method, argument(0), argument(1), span)
            }
            Method::StrZfill => {
                let width = required(argument(0));
                self.text_call(bytes, RuntimeFunction::StrZfill, vec![text, width], span)
            }
            Method::StrExpandtabs => {
                let size = self.int_or(argument(0), 8, span);
                self.text_call(
                    bytes,
                    RuntimeFunction::StrExpandtabs,
                    vec![text, size],
                    span,
                )
            }
            Method::StrIsascii => {
                self.text_call(bytes, RuntimeFunction::StrIsascii, vec![text], span)
            }
            Method::BytesHex => self.text_call(bytes, RuntimeFunction::BytesHex, vec![text], span),
            Method::StrEncode | Method::BytesDecode => {
                self.codec(text, bytes, method, argument(0), argument(1), span)
            }
            other => unreachable!("`{}` is not a method of `str`", other.name()),
        }
    }

    /// Calls `center`, `ljust` or `rjust`, with a space as the fill when none is given.
    fn str_pad(
        &mut self,
        text: ValueId,
        bytes: bool,
        method: Method,
        width: Option<Lowered>,
        fill: Option<Lowered>,
        span: Span,
    ) -> Lowered {
        let mode = match method {
            Method::StrCenter => 0,
            Method::StrLjust => 1,
            _ => 2,
        };
        let width = required(width);
        let fill = match fill {
            Some(fill) => fill.value,
            None => self.text_constant(bytes, " ", span),
        };
        let mode = self.int(mode, span);
        self.text_call(
            bytes,
            RuntimeFunction::StrPad,
            vec![text, width, fill, mode],
            span,
        )
    }

    /// Calls `encode` or `decode` with the numbers of the codec and the error handler, `utf-8`
    /// and `strict` when they are left out.
    fn codec(
        &mut self,
        text: ValueId,
        bytes: bool,
        method: Method,
        codec: Option<Lowered>,
        handler: Option<Lowered>,
        span: Span,
    ) -> Lowered {
        let codec = self.int_or(codec, 0, span);
        let handler = self.int_or(handler, 0, span);
        let function = if method == Method::StrEncode {
            RuntimeFunction::StrEncode
        } else {
            RuntimeFunction::BytesDecode
        };
        self.text_call(bytes, function, vec![text, codec, handler], span)
    }

    /// Calls `function`, or its `bytes` twin when the text is a `bytes`.
    fn text_call(
        &mut self,
        bytes: bool,
        function: RuntimeFunction,
        arguments: Vec<ValueId>,
        span: Span,
    ) -> Lowered {
        let function = if bytes {
            bytes_twin(function)
        } else {
            function
        };
        self.call_runtime(function, arguments, span)
    }

    /// Calls the case method numbered `mode`, as `typy_rt_str_case` numbers them.
    fn str_case(&mut self, text: ValueId, bytes: bool, mode: i64, span: Span) -> Lowered {
        let mode = self.int(mode, span);
        self.text_call(bytes, RuntimeFunction::StrCase, vec![text, mode], span)
    }

    /// Calls the `is` method numbered `question`, as `typy_rt_str_test` numbers them.
    fn str_test(&mut self, text: ValueId, bytes: bool, question: i64, span: Span) -> Lowered {
        let question = self.int(question, span);
        self.text_call(bytes, RuntimeFunction::StrTest, vec![text, question], span)
    }

    /// Calls `split`, `rsplit`, `splitlines`, `partition`, `rpartition` or `join`.
    fn str_split(
        &mut self,
        text: ValueId,
        bytes: bool,
        method: Method,
        given: &[Option<Lowered>],
        result: TyirType,
        span: Span,
    ) -> Lowered {
        let argument = |index: usize| given.get(index).copied().flatten();
        match method {
            Method::StrSplit | Method::StrRsplit => {
                let (separator, mut flags) = self.optional_str(argument(0), bytes, span);
                if method == Method::StrRsplit {
                    flags |= 2;
                }
                let maxsplit = self.int_or(argument(1), -1, span);
                let flags = self.int(flags, span);
                self.text_call(
                    bytes,
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
                self.text_call(
                    bytes,
                    RuntimeFunction::StrSplitlines,
                    vec![text, keep],
                    span,
                )
            }
            Method::StrPartition | Method::StrRpartition => {
                let backwards = self.int(i64::from(method == Method::StrRpartition), span);
                let separator = required(argument(0));
                let parts = self.text_call(
                    bytes,
                    RuntimeFunction::StrPartition,
                    vec![text, separator, backwards],
                    span,
                );
                self.three_parts(parts, result, span)
            }
            Method::StrJoin => {
                let parts = required(argument(0));
                self.text_call(bytes, RuntimeFunction::StrJoin, vec![text, parts], span)
            }
            other => unreachable!("`{}` does not split or join", other.name()),
        }
    }

    /// Calls a search, `find` to `count`, or a match, `startswith` or `endswith`.
    fn str_search(
        &mut self,
        text: ValueId,
        bytes: bool,
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
        self.text_call(
            bytes,
            function,
            vec![text, target.value, start, end, flags],
            span,
        )
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

    /// Returns a text argument that may be `None` or left out, with flag bit 0 set when it is
    /// given; otherwise an empty text of the receiver's type stands in for it.
    fn optional_str(
        &mut self,
        argument: Option<Lowered>,
        bytes: bool,
        span: Span,
    ) -> (ValueId, i64) {
        match argument {
            Some(argument) if argument.ty != TyirType::Unit => (argument.value, 1),
            _ => (self.text_constant(bytes, "", span), 0),
        }
    }

    /// Returns a constant of the text's type: a `str`, or a `bytes` of the same ASCII.
    fn text_constant(&mut self, bytes: bool, text: &str, span: Span) -> ValueId {
        if bytes {
            self.bytes_constant(text.as_bytes(), span)
        } else {
            self.string(text, span)
        }
    }

    /// Returns a tuple of three parts, strings or `bytes`, from the new list of three the
    /// runtime gives, and releases the list.
    fn three_parts(&mut self, parts: Lowered, result: TyirType, span: Span) -> Lowered {
        let TyirType::Tuple(id) = result else {
            unreachable!("`partition` gives a tuple")
        };
        let part = self
            .tuple_types
            .items(id)
            .and_then(|items| items.first().copied())
            .unwrap_or(TyirType::STR);
        let items: Vec<ValueId> = (0..3)
            .map(|index| {
                let position = self.int(index, span);
                let list = Self::borrowed(parts.value, TyirType::LIST);
                let index = Self::borrowed(position, TyirType::Int);
                let item = self.list_item(list, index, part, span);
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

/// Returns the number `typy_rt_str_case` gives a case method, or `None` for another method.
fn case_mode(method: Method) -> Option<i64> {
    let order = [
        Method::StrUpper,
        Method::StrLower,
        Method::StrCasefold,
        Method::StrSwapcase,
        Method::StrCapitalize,
        Method::StrTitle,
    ];
    position(&order, method)
}

/// Returns the number `typy_rt_str_test` gives an `is` method, or `None` for another method.
fn test_question(method: Method) -> Option<i64> {
    let order = [
        Method::StrIsalpha,
        Method::StrIsalnum,
        Method::StrIsdecimal,
        Method::StrIsdigit,
        Method::StrIsnumeric,
        Method::StrIsspace,
        Method::StrIsupper,
        Method::StrIslower,
        Method::StrIstitle,
        Method::StrIsidentifier,
        Method::StrIsprintable,
    ];
    position(&order, method)
}

fn position(order: &[Method], method: Method) -> Option<i64> {
    order
        .iter()
        .position(|each| *each == method)
        .and_then(|index| i64::try_from(index).ok())
}

/// Returns the runtime function of `bytes` that does what `function` does for `str`.
fn bytes_twin(function: RuntimeFunction) -> RuntimeFunction {
    use RuntimeFunction as F;
    match function {
        F::StrSearch => F::BytesSearch,
        F::StrMatch => F::BytesMatch,
        F::StrMatchAny => F::BytesMatchAny,
        F::StrSplit => F::BytesSplit,
        F::StrSplitlines => F::BytesSplitlines,
        F::StrPartition => F::BytesPartition,
        F::StrJoin => F::BytesJoin,
        F::StrStrip => F::BytesStrip,
        F::StrRemoveAffix => F::BytesRemoveAffix,
        F::StrReplace => F::BytesReplace,
        F::StrPad => F::BytesPad,
        F::StrZfill => F::BytesZfill,
        F::StrExpandtabs => F::BytesExpandtabs,
        F::StrIsascii => F::BytesIsascii,
        F::StrCase => F::BytesCase,
        F::StrTest => F::BytesTest,
        other => other,
    }
}
