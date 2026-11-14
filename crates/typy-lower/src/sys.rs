//! Lowers `sys` and `input` (LIB-04): the values of `sys` from the runtime, `sys.exit` as
//! raising `SystemExit`, and a line of standard input.

use typy_check::{BuiltinException, ExceptionClass, SysValue, TypedExpr};
use typy_diagnostics::Span;
use typy_tyir::{Constant, RuntimeFunction, Terminator, TyirType, TypeId, ValueId};

use crate::function::FunctionLowerer;
use crate::ownership::Lowered;

impl FunctionLowerer<'_> {
    /// Returns `sys.argv`, `sys.stdout`, `sys.stderr` or `sys.stdin`, which the runtime keeps for the whole
    /// run, so the value is borrowed.
    pub(crate) fn sys_value(&mut self, value: SysValue, span: Span) -> Lowered {
        let (function, ty) = match value {
            SysValue::Argv => (RuntimeFunction::SysArgv, TyirType::LIST),
            SysValue::Stdout => (RuntimeFunction::SysStdout, TyirType::TEXT_FILE),
            SysValue::Stderr => (RuntimeFunction::SysStderr, TyirType::TEXT_FILE),
            SysValue::Stdin => (RuntimeFunction::SysStdin, TyirType::TEXT_FILE),
        };
        let found = self.call_runtime(function, Vec::new(), span);
        self.borrowed_result(Lowered { ty, ..found })
    }

    /// Returns `sys.getrecursionlimit()`, or sets the limit for `sys.setrecursionlimit(limit)`,
    /// whose `bool` becomes an `int` first (LIB-13).
    pub(crate) fn recursion_limit(&mut self, limit: Option<&TypedExpr>, span: Span) -> Lowered {
        let Some(limit) = limit else {
            return self.call_runtime(RuntimeFunction::SysGetRecursionLimit, Vec::new(), span);
        };
        let value = self.expression(limit);
        let integer = self.int_of(value, span);
        let result = self.call_runtime(
            RuntimeFunction::SysSetRecursionLimit,
            vec![integer.value],
            span,
        );
        if integer.value != value.value {
            self.release(integer, span);
        }
        self.release(value, span);
        result
    }

    /// Lowers `sys.exit(code)`: raises `SystemExit` with the code, or with no argument.
    pub(crate) fn sys_exit(&mut self, code: Option<&TypedExpr>, span: Span) -> Lowered {
        let class = ExceptionClass::Builtin(BuiltinException::SystemExit);
        let arguments: Vec<TypedExpr> = code.into_iter().cloned().collect();
        let exception = self.new_exception(&class, &arguments, None, span);
        self.call_runtime(RuntimeFunction::ExceptionThrow, vec![exception.value], span);
        // The error path released the exception, and no code follows the throw.
        self.untrack(exception.value);
        self.builder.terminate(Terminator::Unreachable);
        let after = self.builder.create_block();
        self.builder.switch_to(after);
        Self::borrowed(self.constant(Constant::Unit, span), TyirType::Unit)
    }

    /// Returns `input(prompt)`: `str(prompt)` written first, when there is a prompt.
    pub(crate) fn input(&mut self, prompt: Option<&TypedExpr>, span: Span) -> Lowered {
        let text = match prompt {
            Some(prompt) => {
                let value = self.expression(prompt);
                let text = self.string_of(value, span);
                self.release(value, span);
                Some(text)
            }
            None => None,
        };
        let argument = match text {
            Some(text) => text.value,
            None => self.constant(Constant::NoInstance(TypeId::STR), span),
        };
        let line = self.call_runtime(RuntimeFunction::Input, vec![argument], span);
        if let Some(text) = text {
            self.release(text, span);
        }
        line
    }

    /// Calls a method of a text file, or of a binary one when `binary`; a `size` not given, or
    /// `None`, is -1, as in CPython.
    pub(crate) fn text_method(
        &mut self,
        file: typy_tyir::ValueId,
        binary: bool,
        method: typy_check::Method,
        given: &[Option<Lowered>],
        span: Span,
    ) -> Lowered {
        use typy_check::Method;
        let argument = given.first().copied().flatten().map(|value| value.value);
        let (function, arguments) = match method {
            Method::TextWrite => (
                if binary {
                    RuntimeFunction::BinaryWrite
                } else {
                    RuntimeFunction::TextWrite
                },
                vec![file, argument.unwrap_or(file)],
            ),
            Method::TextWritelines => (
                RuntimeFunction::TextWritelines,
                vec![file, argument.unwrap_or(file)],
            ),
            Method::TextRead | Method::TextReadline => {
                let size = given
                    .first()
                    .copied()
                    .flatten()
                    .filter(|size| size.ty == TyirType::Int)
                    .map_or_else(
                        || self.constant(Constant::Int("-1".to_owned()), span),
                        |size| size.value,
                    );
                let function = match (method == Method::TextRead, binary) {
                    (true, false) => RuntimeFunction::TextRead,
                    (false, false) => RuntimeFunction::TextReadline,
                    (true, true) => RuntimeFunction::BinaryRead,
                    (false, true) => RuntimeFunction::BinaryReadline,
                };
                (function, vec![file, size])
            }
            Method::TextReadlines if binary => (RuntimeFunction::BinaryReadlines, vec![file]),
            Method::TextReadlines => (RuntimeFunction::TextReadlines, vec![file]),
            Method::TextClose => (RuntimeFunction::TextClose, vec![file]),
            Method::TextFileno => (RuntimeFunction::TextFileno, vec![file]),
            Method::TextTell => (RuntimeFunction::TextTell, vec![file]),
            Method::TextIsatty
            | Method::TextReadable
            | Method::TextWritable
            | Method::TextSeekable => {
                let question = match method {
                    Method::TextIsatty => "0",
                    Method::TextReadable => "1",
                    Method::TextWritable => "2",
                    _ => "3",
                };
                let question = self.constant(Constant::Int(question.to_owned()), span);
                (RuntimeFunction::TextState, vec![file, question])
            }
            Method::TextSeek => {
                let whence = given.get(1).copied().flatten().map_or_else(
                    || self.constant(Constant::Int("0".to_owned()), span),
                    |whence| whence.value,
                );
                (
                    RuntimeFunction::TextSeek,
                    vec![file, argument.unwrap_or(file), whence],
                )
            }
            Method::TextTruncate => {
                // `None` given or left out cuts at the file's position.
                let size = given
                    .first()
                    .copied()
                    .flatten()
                    .filter(|size| size.ty == TyirType::Int);
                let flag = if size.is_some() { "1" } else { "0" };
                let flag = self.constant(Constant::Int(flag.to_owned()), span);
                let size = size.map_or_else(
                    || self.constant(Constant::Int("0".to_owned()), span),
                    |size| size.value,
                );
                (RuntimeFunction::TextTruncate, vec![file, size, flag])
            }
            _ => (RuntimeFunction::TextFlush, vec![file]),
        };
        self.call_runtime(function, arguments, span)
    }

    /// Returns the word `typy_rt_open` reads: `how` as the checker found it, with the bit of
    /// `closefd=False` set when `closefd` is false, a literal or a value tested here.
    fn open_how(&mut self, how: i64, closefd: Option<&TypedExpr>, span: Span) -> ValueId {
        let keep_open = how | 1 << 11;
        let Some(closefd) = closefd else {
            return self.constant(Constant::Int(how.to_string()), span);
        };
        if let typy_check::TypedExprKind::Constant(typy_check::TypedConstant::Bool(value)) =
            &closefd.kind
        {
            let chosen = if *value { how } else { keep_open };
            return self.constant(Constant::Int(chosen.to_string()), span);
        }
        let value = self.expression(closefd);
        let slot = self.hidden_slot("how", TyirType::Int);
        let closing = self.constant(Constant::Int(how.to_string()), span);
        self.store(slot, Self::borrowed(closing, TyirType::Int), span);
        let keeping = self.builder.create_block();
        let after = self.builder.create_block();
        self.builder.terminate(Terminator::Branch {
            condition: value.value,
            then: after,
            otherwise: keeping,
        });
        self.builder.switch_to(keeping);
        let kept = self.constant(Constant::Int(keep_open.to_string()), span);
        self.store(slot, Self::borrowed(kept, TyirType::Int), span);
        self.jump_to(after);
        self.builder.switch_to(after);
        self.release(value, span);
        self.load(slot, span).value
    }

    /// Returns `name`, `mode` or `closed` of a text file (LIB-14).
    pub(crate) fn file_attribute(
        &mut self,
        file: &TypedExpr,
        attribute: typy_check::FileAttribute,
        span: Span,
    ) -> Lowered {
        let file = self.expression(file);
        let value = match attribute {
            typy_check::FileAttribute::Name => {
                self.call_runtime(RuntimeFunction::TextName, vec![file.value], span)
            }
            typy_check::FileAttribute::Mode => {
                self.call_runtime(RuntimeFunction::TextMode, vec![file.value], span)
            }
            typy_check::FileAttribute::Descriptor => {
                self.call_runtime(RuntimeFunction::TextDescriptorName, vec![file.value], span)
            }
            typy_check::FileAttribute::Closed => {
                let question = self.constant(Constant::Int("4".to_owned()), span);
                self.call_runtime(RuntimeFunction::TextState, vec![file.value, question], span)
            }
        };
        self.release(file, span);
        value
    }

    /// Lowers `open(path, mode, buffering, encoding, errors, newline)`: a new text file, or the
    /// error of opening it. Its arguments are evaluated in the order `open` takes them.
    pub(crate) fn open(
        &mut self,
        path: &TypedExpr,
        mode: &str,
        access: typy_check::OpenAccess,
        options: &typy_check::OpenOptions,
        file: &typy_check::Type,
        span: Span,
    ) -> Lowered {
        let path = self.expression(path);
        let mode_value = options.mode.as_deref().map(|value| self.expression(value));
        let buffering = options
            .buffering
            .as_deref()
            .map(|value| self.expression(value));
        let optional_str = TyirType::Optional(typy_tyir::OptionalBase::Object(TypeId::STR));
        let newline = options.newline.as_deref().map(|value| {
            if value.ty == typy_check::Type::Str {
                self.wrap(value, optional_str, span)
            } else {
                self.expression(value)
            }
        });
        let mode = mode_value.map_or_else(|| self.string(mode, span), |value| value.value);
        // The runtime reads the access, with `+` in its third bit, the codec, the handler and
        // whether the file is binary or a `FileIO` from one word, so a call keeps to six
        // arguments.
        let (binary, raw) = file.binary_kind().map_or((0, 0), |kind| {
            (1, i64::from(kind == typy_check::BinaryKind::Raw))
        });
        let how = access as i64
            | options.codec << 3
            | options.handler << 5
            | binary << 8
            | raw << 9
            | i64::from(mode_value.is_some()) << 10;
        let how = self.open_how(how, options.closefd.as_deref(), span);
        let buffering_value = buffering.map_or_else(
            || self.constant(Constant::Int("-1".to_owned()), span),
            |value| value.value,
        );
        let encoding = match &options.encoding {
            Some(name) => self.string(name, span),
            None => self.constant(Constant::NoInstance(TypeId::STR), span),
        };
        let newline_value =
            newline.map_or_else(|| self.none_of(optional_str, span), |value| value.value);
        let function = if path.ty == TyirType::Int {
            RuntimeFunction::OpenDescriptor
        } else {
            RuntimeFunction::Open
        };
        let file = self.call_runtime(
            function,
            vec![
                path.value,
                mode,
                how,
                buffering_value,
                encoding,
                newline_value,
            ],
            span,
        );
        for value in [newline, buffering, mode_value].into_iter().flatten() {
            self.release(value, span);
        }
        self.release(path, span);
        file
    }
}
