//! The calling convention of generated code (detailed design section 10.2).

use inkwell::AddressSpace;
use inkwell::context::Context;
use inkwell::types::{BasicMetadataTypeEnum, BasicType, BasicTypeEnum, FunctionType};
use typy_tyir::TyirType;

/// Returns the LLVM type of a TyIR value; `unit` has none.
pub(crate) fn value_type(context: &Context, ty: TyirType) -> Option<BasicTypeEnum<'_>> {
    match ty {
        TyirType::Int => Some(context.i64_type().into()),
        TyirType::Float => Some(context.f64_type().into()),
        TyirType::Bool => Some(context.bool_type().into()),
        TyirType::Unit => None,
        TyirType::Object(_) => Some(context.ptr_type(AddressSpace::default()).into()),
    }
}

/// Returns the type a function returns. A function that may raise returns `{result, exception}`, or only the exception when its result is `unit`; a null exception means none was raised.
pub(crate) fn function_type<'ctx>(
    context: &'ctx Context,
    parameters: &[TyirType],
    result: TyirType,
    raising: bool,
    boundary: bool,
) -> FunctionType<'ctx> {
    let parameter_types: Vec<BasicMetadataTypeEnum<'ctx>> = parameters
        .iter()
        .filter_map(|ty| value_type(context, *ty))
        .map(Into::into)
        .collect();
    let pointer = context.ptr_type(AddressSpace::default());
    // A `bool` crosses into the runtime as one byte, as C's `_Bool` does.
    let result_type = match result {
        TyirType::Bool if boundary => Some(context.i8_type().into()),
        other => value_type(context, other),
    };
    match (raising, result_type) {
        (false, None) => context.void_type().fn_type(&parameter_types, false),
        (false, Some(ty)) => ty.fn_type(&parameter_types, false),
        (true, None) => pointer.fn_type(&parameter_types, false),
        (true, Some(ty)) => context
            .struct_type(&[ty, pointer.into()], false)
            .fn_type(&parameter_types, false),
    }
}

#[cfg(test)]
mod tests {
    use super::{function_type, value_type};
    use inkwell::context::Context;
    use typy_tyir::TyirType;

    #[test]
    fn values_have_their_llvm_types() {
        let context = Context::create();
        assert_eq!(
            value_type(&context, TyirType::Int).unwrap().to_string(),
            "\"i64\""
        );
        assert_eq!(
            value_type(&context, TyirType::Float).unwrap().to_string(),
            "\"double\""
        );
        assert_eq!(
            value_type(&context, TyirType::Bool).unwrap().to_string(),
            "\"i1\""
        );
        assert_eq!(
            value_type(&context, TyirType::STR).unwrap().to_string(),
            "\"ptr\""
        );
        assert!(value_type(&context, TyirType::Unit).is_none());
    }

    #[test]
    fn raising_functions_return_the_exception_beside_the_result() {
        let context = Context::create();
        let cases = [
            (TyirType::Unit, false, false, "void (i64)"),
            (TyirType::Int, false, false, "i64 (i64)"),
            (TyirType::Unit, true, false, "ptr (i64)"),
            (TyirType::Int, true, false, "{ i64, ptr } (i64)"),
            (TyirType::Bool, false, true, "i8 (i64)"),
        ];
        for (result, raising, boundary, expected) in cases {
            let ty = function_type(
                &context,
                &[TyirType::Int, TyirType::Unit],
                result,
                raising,
                boundary,
            );
            assert_eq!(ty.to_string(), format!("\"{expected}\""));
        }
    }
}
