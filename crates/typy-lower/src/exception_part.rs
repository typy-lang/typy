//! Lowers the operations on `args` of an exception and `code` of a `SystemExit` (EXC-11,
//! EXC-13, ADR-0030): each calls the runtime with the exception, and `==` passes the other
//! value as a tuple of one, whose layout tells the runtime its type. Also lowers the
//! attributes of an `OSError` (EXC-14).

use typy_check::{ExceptionPart, PartOperation, TypedExpr};
use typy_diagnostics::Span;
use typy_tyir::{Constant, RuntimeFunction, TyirType};

use crate::function::FunctionLowerer;
use crate::ownership::Lowered;

impl FunctionLowerer<'_> {
    /// Returns `operation` on `part` of the exception `exception`.
    pub(crate) fn exception_part(
        &mut self,
        exception: &TypedExpr,
        part: ExceptionPart,
        operation: &PartOperation,
        span: Span,
    ) -> Lowered {
        let exception = self.expression(exception);
        let code = match part {
            ExceptionPart::Arguments => "0",
            ExceptionPart::Code => "1",
        };
        let part = self.constant(Constant::Int(code.to_owned()), span);
        let result = match operation {
            PartOperation::Text { repr } => {
                let repr = self.constant(Constant::Int(u8::from(*repr).to_string()), span);
                self.call_runtime(
                    RuntimeFunction::ExceptionPartText,
                    vec![exception.value, part, repr],
                    span,
                )
            }
            PartOperation::Length => self.call_runtime(
                RuntimeFunction::ExceptionArgsLength,
                vec![exception.value],
                span,
            ),
            PartOperation::Truth => self.call_runtime(
                RuntimeFunction::ExceptionPartTruth,
                vec![exception.value, part],
                span,
            ),
            PartOperation::Equal(other) => {
                let id = self.tuple_type_id(std::slice::from_ref(&other.ty));
                let descriptor = self.constant(Constant::TupleDescriptor(id), span);
                let tuple = self.tuple_display(
                    std::slice::from_ref(other.as_ref()),
                    TyirType::Tuple(id),
                    span,
                );
                let address = self.bits(Self::borrowed(tuple.value, tuple.ty), span);
                let equal = self.call_runtime(
                    RuntimeFunction::ExceptionPartEqual,
                    vec![exception.value, part, descriptor, address],
                    span,
                );
                self.release(tuple, span);
                equal
            }
        };
        self.release(exception, span);
        result
    }
}
