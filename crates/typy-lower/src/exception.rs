//! Lowers exceptions: a call of an exception class, `raise`, and `str` and `repr` of an
//! exception (SMT-06, EXC-04, EXC-08, ADR-0013).

use typy_check::{ExceptionClass, RaiseCause, Type, TypedExpr};
use typy_diagnostics::Span;
use typy_tyir::{Constant, RuntimeFunction, Terminator, TyirType, TypeObjectRef};

use crate::function::FunctionLowerer;
use crate::ownership::Lowered;

/// Returns the type object of an exception class.
pub(crate) fn exception_class(class: &ExceptionClass) -> TypeObjectRef {
    let ExceptionClass::Builtin(builtin) = class;
    TypeObjectRef::Exception(builtin.name())
}

impl FunctionLowerer<'_> {
    /// Lowers a call of an exception class: the arguments are evaluated in order into a tuple,
    /// which the new exception copies, and the tuple is released.
    pub(crate) fn new_exception(
        &mut self,
        class: &ExceptionClass,
        arguments: &[TypedExpr],
        span: Span,
    ) -> Lowered {
        let class = self.constant(Constant::TypeObject(exception_class(class)), span);
        let types = arguments.iter().map(|value| value.ty.clone()).collect();
        let layout = self.lowered_type(&Type::Tuple(types));
        let TyirType::Tuple(id) = layout else {
            unreachable!("a tuple type lowers to a tuple");
        };
        let descriptor = self.constant(Constant::TupleDescriptor(id), span);
        if arguments.is_empty() {
            let unit = self.constant(Constant::Unit, span);
            let nothing = self.bits(Self::borrowed(unit, TyirType::Unit), span);
            return self.call_runtime(
                RuntimeFunction::ExceptionNew,
                vec![class, descriptor, nothing],
                span,
            );
        }
        let tuple = self.tuple_display(arguments, layout, span);
        let address = self.bits(Self::borrowed(tuple.value, tuple.ty), span);
        let exception = self.call_runtime(
            RuntimeFunction::ExceptionNew,
            vec![class, descriptor, address],
            span,
        );
        self.release(tuple, span);
        exception
    }

    /// Lowers `raise`: the exception, or the one being handled for a bare `raise`, takes its
    /// cause, then the runtime raises it and the error path of the call takes it on. Nothing
    /// after it runs.
    pub(crate) fn raise_statement(
        &mut self,
        exception: Option<&TypedExpr>,
        cause: &RaiseCause,
        span: Span,
    ) {
        let exception = match exception {
            Some(exception) => self.expression(exception),
            None => self.call_runtime(RuntimeFunction::ExceptionHandled, Vec::new(), span),
        };
        match cause {
            RaiseCause::Absent => {}
            RaiseCause::None => {
                let unit = self.constant(Constant::Unit, span);
                let none = self.bits(Self::borrowed(unit, TyirType::Unit), span);
                self.call_runtime(
                    RuntimeFunction::ExceptionSetCause,
                    vec![exception.value, none],
                    span,
                );
            }
            RaiseCause::Exception(cause) => {
                let cause = self.expression(cause);
                let bits = self.bits(Self::borrowed(cause.value, cause.ty), span);
                self.call_runtime(
                    RuntimeFunction::ExceptionSetCause,
                    vec![exception.value, bits],
                    span,
                );
                self.release(cause, span);
            }
        }
        self.call_runtime(RuntimeFunction::ExceptionThrow, vec![exception.value], span);
        self.builder.terminate(Terminator::Unreachable);
    }

    /// Returns `str(exception)`, or `repr(exception)` when `repr` is set, through the
    /// exception's class. The exception is not released.
    pub(crate) fn exception_text(&mut self, lowered: Lowered, repr: bool, span: Span) -> Lowered {
        let flag = self.constant(Constant::Int(i64::from(repr).to_string()), span);
        self.call_runtime(
            RuntimeFunction::ExceptionText,
            vec![lowered.value, flag],
            span,
        )
    }

    /// Returns `type(exception)`, the class the exception was made from, which may derive from
    /// its static class. The exception is not released.
    pub(crate) fn exception_type(&mut self, lowered: &Lowered, span: Span) -> Lowered {
        let class = self.call_runtime(RuntimeFunction::ExceptionClass, vec![lowered.value], span);
        Self::borrowed(class.value, TyirType::TYPE)
    }
}
