//! Lowers `sys` and `input` (LIB-04): the values of `sys` from the runtime, `sys.exit` as
//! raising `SystemExit`, and a line of standard input.

use typy_check::{BuiltinException, ExceptionClass, SysValue, TypedExpr};
use typy_diagnostics::Span;
use typy_tyir::{Constant, RuntimeFunction, Terminator, TyirType, TypeId};

use crate::function::FunctionLowerer;
use crate::ownership::Lowered;

impl FunctionLowerer<'_> {
    /// Returns `sys.argv`, `sys.stdout` or `sys.stderr`, which the runtime keeps for the whole
    /// run, so the value is borrowed.
    pub(crate) fn sys_value(&mut self, value: SysValue, span: Span) -> Lowered {
        let (function, ty) = match value {
            SysValue::Argv => (RuntimeFunction::SysArgv, TyirType::LIST),
            SysValue::Stdout => (RuntimeFunction::SysStdout, TyirType::TEXT_FILE),
            SysValue::Stderr => (RuntimeFunction::SysStderr, TyirType::TEXT_FILE),
        };
        let found = self.call_runtime(function, Vec::new(), span);
        self.borrowed_result(Lowered { ty, ..found })
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
            _ => (RuntimeFunction::TextFlush, vec![file]),
        };
        self.call_runtime(function, arguments, span)
    }

    /// Lowers `open(path, mode)`: a new text file, or the `OSError` of opening it.
    pub(crate) fn open(
        &mut self,
        path: &TypedExpr,
        mode: &str,
        access: typy_check::OpenAccess,
        span: Span,
    ) -> Lowered {
        let path = self.expression(path);
        let mode = self.string(mode, span);
        let access = self.constant(Constant::Int((access as i64).to_string()), span);
        let file = self.call_runtime(RuntimeFunction::Open, vec![path.value, mode, access], span);
        self.release(path, span);
        file
    }
}
