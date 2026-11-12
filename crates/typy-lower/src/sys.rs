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

    /// Calls `write` or `flush` of a text file.
    pub(crate) fn text_method(
        &mut self,
        file: typy_tyir::ValueId,
        method: typy_check::Method,
        arguments: &[Lowered],
        span: Span,
    ) -> Lowered {
        if method == typy_check::Method::TextWrite {
            return self.call_runtime(
                RuntimeFunction::TextWrite,
                vec![file, arguments[0].value],
                span,
            );
        }
        self.call_runtime(RuntimeFunction::TextFlush, vec![file], span)
    }
}
