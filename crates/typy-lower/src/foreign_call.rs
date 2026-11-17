//! Lowers a call of a C function a `ctypes` declaration describes: each argument becomes the
//! TyPy value its C type takes, and the call gives its result as its TyPy type (FFI-02,
//! FFI-03, ADR-0047).

use typy_check::TypedExpr;
use typy_diagnostics::Span;
use typy_tyir::{CType, Constant, Operation, OptionalBase, TyirType, TypeId};

use crate::function::FunctionLowerer;
use crate::ownership::Lowered;

impl FunctionLowerer<'_> {
    /// Lowers `library.function(arguments)` of the C function at `function`.
    pub(crate) fn foreign_call(
        &mut self,
        function: u32,
        arguments: &[TypedExpr],
        span: Span,
    ) -> Lowered {
        let declared = &self.program.foreign_functions[function as usize];
        let parameters: Vec<CType> = declared.parameters.iter().map(|ty| c_type(*ty)).collect();
        let result = declared.result.map(c_type);
        let mut held = Vec::new();
        let mut values = Vec::new();
        for (position, (argument, parameter)) in arguments.iter().zip(parameters).enumerate() {
            let lowered = self.expression(argument);
            let passed = self.foreign_argument(lowered, parameter, position + 1, span);
            values.push(passed.value);
            held.push(lowered);
            held.push(passed);
        }
        let ty = result.map_or(TyirType::Unit, CType::result_type);
        let operation = Operation::CallForeign {
            function,
            arguments: values,
        };
        let returned = if ty == TyirType::Unit {
            self.builder.push(operation, span);
            Self::borrowed(self.constant(Constant::Unit, span), TyirType::Unit)
        } else {
            let value = self.builder.push_value(operation, ty, span);
            self.produced(value, ty)
        };
        // A conversion may give the argument itself back; each value is released once.
        let mut released: Vec<typy_tyir::ValueId> = Vec::new();
        for lowered in held.into_iter().rev() {
            if !released.contains(&lowered.value) {
                released.push(lowered.value);
                self.release(lowered, span);
            }
        }
        returned
    }

    /// Returns `lowered`, the argument at `position` counted from 1, as the value C type
    /// `parameter` takes: an `int` of a `bool`, a `float` of an `int`, which raises
    /// `ctypes.ArgumentError` when it is too large, a truth of any value, and an optional
    /// `None` of `None`.
    fn foreign_argument(
        &mut self,
        lowered: Lowered,
        parameter: CType,
        position: usize,
        span: Span,
    ) -> Lowered {
        match parameter {
            CType::Float | CType::Double if lowered.ty == TyirType::Int => {
                let position = self.constant(Constant::Int(position.to_string()), span);
                self.call_runtime(
                    typy_tyir::RuntimeFunction::CtypesFloatArgument,
                    vec![lowered.value, position],
                    span,
                )
            }
            CType::Float | CType::Double => self.float_of(lowered, span),
            CType::Bool => {
                let truth = self.truth(lowered, span);
                Self::borrowed(truth, TyirType::Bool)
            }
            CType::CharPointer | CType::VoidPointer if lowered.ty == TyirType::Unit => {
                let base = if parameter == CType::CharPointer {
                    OptionalBase::Object(TypeId::BYTES)
                } else {
                    OptionalBase::Int
                };
                let none = self.constant(Constant::NoneOf(base), span);
                Self::borrowed(none, TyirType::Optional(base))
            }
            CType::CharPointer => lowered,
            _ if lowered.ty == TyirType::Bool => self.int_of(lowered, span),
            _ => lowered,
        }
    }
}

/// Returns the TyIR C type of a checked C type.
pub(crate) const fn c_type(ty: typy_check::CType) -> CType {
    match ty {
        typy_check::CType::Int8 => CType::Int8,
        typy_check::CType::UInt8 => CType::UInt8,
        typy_check::CType::Int16 => CType::Int16,
        typy_check::CType::UInt16 => CType::UInt16,
        typy_check::CType::Int32 => CType::Int32,
        typy_check::CType::UInt32 => CType::UInt32,
        typy_check::CType::Int64 => CType::Int64,
        typy_check::CType::UInt64 => CType::UInt64,
        typy_check::CType::Float => CType::Float,
        typy_check::CType::Double => CType::Double,
        typy_check::CType::Bool => CType::Bool,
        typy_check::CType::CharPointer => CType::CharPointer,
        typy_check::CType::VoidPointer => CType::VoidPointer,
    }
}
