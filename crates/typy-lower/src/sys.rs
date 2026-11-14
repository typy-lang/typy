//! Lowers `sys` and `input` (LIB-04): the values of `sys` from the runtime, `sys.exit` as
//! raising `SystemExit`, and a line of standard input.

use typy_check::{BuiltinException, ExceptionClass, SysValue, TypedExpr};
use typy_diagnostics::Span;
use typy_tyir::{Constant, RuntimeFunction, Terminator, TyirType, TypeId};

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

    /// Calls a method of a text file; a `size` not given is -1, as in CPython.
    pub(crate) fn text_method(
        &mut self,
        file: typy_tyir::ValueId,
        method: typy_check::Method,
        given: &[Option<Lowered>],
        span: Span,
    ) -> Lowered {
        use typy_check::Method;
        let argument = given.first().copied().flatten().map(|value| value.value);
        let (function, arguments) = match method {
            Method::TextWrite => (
                RuntimeFunction::TextWrite,
                vec![file, argument.unwrap_or(file)],
            ),
            Method::TextWritelines => (
                RuntimeFunction::TextWritelines,
                vec![file, argument.unwrap_or(file)],
            ),
            Method::TextRead | Method::TextReadline => {
                let size =
                    argument.unwrap_or_else(|| self.constant(Constant::Int("-1".to_owned()), span));
                let function = if method == Method::TextRead {
                    RuntimeFunction::TextRead
                } else {
                    RuntimeFunction::TextReadline
                };
                (function, vec![file, size])
            }
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
        span: Span,
    ) -> Lowered {
        let path = self.expression(path);
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
        let mode = self.string(mode, span);
        // The runtime reads the access, the codec and the handler from one word, so a call
        // keeps to six arguments.
        let how = access as i64 | options.codec << 2 | options.handler << 4;
        let how = self.constant(Constant::Int(how.to_string()), span);
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
        let file = self.call_runtime(
            RuntimeFunction::Open,
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
        for value in [newline, buffering].into_iter().flatten() {
            self.release(value, span);
        }
        self.release(path, span);
        file
    }
}
