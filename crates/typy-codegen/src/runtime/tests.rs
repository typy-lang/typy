//! Tests of `runtime`.

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
