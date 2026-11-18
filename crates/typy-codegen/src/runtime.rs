//! Declarations of the runtime's functions, as generated code calls them.

use inkwell::AddressSpace;
use inkwell::attributes::{Attribute, AttributeLoc};
use inkwell::context::Context;
use inkwell::module::{Linkage, Module};
use inkwell::values::FunctionValue;
use typy_tyir::{RuntimeFunction, TupleTypes, TyirType};

use crate::abi::function_type;

/// A runtime function that generated code calls directly, beside those TyIR names.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Internal {
    /// Adds a reference to an object.
    Increment,
    /// Drops a reference to an object.
    Decrement,
    /// Adds a reference to an `int` word when it points to a large integer.
    IntIncrement,
    /// Drops a reference to an `int` word when it points to a large integer.
    IntDecrement,
    /// `int` addition, subtraction, multiplication and bitwise operations.
    IntAdd,
    IntSubtract,
    IntMultiply,
    IntAnd,
    IntOr,
    IntXor,
    IntNegate,
    IntInvert,
    /// Compares two `int` words; the comparison is a tagged `int`.
    IntCompare,
    /// Adds a traceback entry to an exception: function name, file name and line.
    TracebackAdd,
}

impl Internal {
    fn name(self) -> &'static str {
        match self {
            Self::Increment => "typy_rt_increment",
            Self::Decrement => "typy_rt_decrement",
            Self::IntIncrement => "typy_rt_int_increment",
            Self::IntDecrement => "typy_rt_int_decrement",
            Self::IntAdd => "typy_rt_int_add",
            Self::IntSubtract => "typy_rt_int_subtract",
            Self::IntMultiply => "typy_rt_int_multiply",
            Self::IntAnd => "typy_rt_int_and",
            Self::IntOr => "typy_rt_int_or",
            Self::IntXor => "typy_rt_int_xor",
            Self::IntNegate => "typy_rt_int_negate",
            Self::IntInvert => "typy_rt_int_invert",
            Self::IntCompare => "typy_rt_int_compare",
            Self::TracebackAdd => "typy_rt_traceback_add",
        }
    }
}

fn declare<'ctx>(
    module: &Module<'ctx>,
    name: &str,
    ty: inkwell::types::FunctionType<'ctx>,
) -> FunctionValue<'ctx> {
    module
        .get_function(name)
        .unwrap_or_else(|| module.add_function(name, ty, Some(Linkage::External)))
}

/// Returns the declaration of a runtime function that TyIR names.
pub(crate) fn runtime_function<'ctx>(
    context: &'ctx Context,
    module: &Module<'ctx>,
    function: RuntimeFunction,
) -> FunctionValue<'ctx> {
    let signature = function.signature();
    // Runtime functions take and return no tuples.
    let ty = function_type(
        context,
        &TupleTypes::default(),
        signature.parameters,
        signature.result,
        signature.may_raise,
        true,
    );
    let declaration = declare(module, &format!("typy_rt_{}", function.name()), ty);
    // Rust declares a `bool` parameter `zeroext i1`, so the caller widens it (ADR-0053).
    let zero_extend =
        context.create_enum_attribute(Attribute::get_named_enum_kind_id("zeroext"), 0);
    for (index, parameter) in (0_u32..).zip(declaration.get_param_iter()) {
        if parameter.get_type() == context.bool_type().into() {
            declaration.add_attribute(AttributeLoc::Param(index), zero_extend);
        }
    }
    declaration
}

/// Returns the declaration of a runtime function that generated code calls directly.
pub(crate) fn internal_function<'ctx>(
    context: &'ctx Context,
    module: &Module<'ctx>,
    function: Internal,
) -> FunctionValue<'ctx> {
    let pointer = context.ptr_type(AddressSpace::default());
    let word = context.i64_type();
    let ty = match function {
        Internal::Increment | Internal::Decrement => {
            context.void_type().fn_type(&[pointer.into()], false)
        }
        Internal::IntIncrement | Internal::IntDecrement => {
            context.void_type().fn_type(&[word.into()], false)
        }
        Internal::IntNegate | Internal::IntInvert => word.fn_type(&[word.into()], false),
        Internal::IntCompare => function_type(
            context,
            &TupleTypes::default(),
            &[TyirType::Int, TyirType::Int, TyirType::Int],
            TyirType::Bool,
            false,
            true,
        ),
        Internal::TracebackAdd => context.void_type().fn_type(
            &[
                pointer.into(),
                pointer.into(),
                pointer.into(),
                context.i32_type().into(),
            ],
            false,
        ),
        _ => word.fn_type(&[word.into(), word.into()], false),
    };
    declare(module, function.name(), ty)
}

#[cfg(test)]
mod tests {
    use inkwell::attributes::{Attribute, AttributeLoc};
    use inkwell::context::Context;
    use inkwell::types::BasicTypeEnum;
    use typy_tyir::RuntimeFunction;

    use super::runtime_function;

    #[test]
    fn no_runtime_function_returns_a_float_beside_an_exception() {
        let context = Context::create();
        let module = context.create_module("runtime");
        for function in RuntimeFunction::ALL {
            let declaration = runtime_function(&context, &module, function);
            let returned = declaration.get_type().get_return_type();
            if let Some(BasicTypeEnum::StructType(pair)) = returned {
                assert!(
                    !pair
                        .get_field_types()
                        .iter()
                        .any(|field| field.is_float_type()),
                    "{} returns {pair}",
                    function.name()
                );
            }
        }
    }

    #[test]
    fn every_bool_parameter_of_the_runtime_is_zero_extended() {
        let context = Context::create();
        let module = context.create_module("runtime");
        let kind = Attribute::get_named_enum_kind_id("zeroext");
        for function in RuntimeFunction::ALL {
            let declaration = runtime_function(&context, &module, function);
            for (index, parameter) in (0_u32..).zip(declaration.get_param_iter()) {
                if parameter.get_type() == context.bool_type().into() {
                    assert!(
                        declaration
                            .get_enum_attribute(AttributeLoc::Param(index), kind)
                            .is_some(),
                        "{} parameter {index}",
                        function.name()
                    );
                }
            }
        }
    }
}
