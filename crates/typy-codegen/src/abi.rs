//! The calling convention of generated code (detailed design section 10.2).

use inkwell::AddressSpace;
use inkwell::context::Context;
use inkwell::types::{BasicMetadataTypeEnum, BasicType, BasicTypeEnum, FunctionType};
use typy_tyir::{TupleTypes, TyirType};

/// Returns the LLVM type of a TyIR value; `unit` has none. A tuple is a struct of its items
/// that have a value, so an item of type `None` takes no field.
pub(crate) fn value_type<'ctx>(
    context: &'ctx Context,
    tuples: &TupleTypes,
    ty: TyirType,
) -> Option<BasicTypeEnum<'ctx>> {
    match ty {
        TyirType::Int | TyirType::Bits => Some(context.i64_type().into()),
        TyirType::Float => Some(context.f64_type().into()),
        TyirType::Bool => Some(context.bool_type().into()),
        TyirType::Unit => None,
        TyirType::Object(_) => Some(context.ptr_type(AddressSpace::default()).into()),
        TyirType::Tuple(id) => {
            let fields: Vec<BasicTypeEnum<'ctx>> = tuples
                .items(id)
                .unwrap_or_default()
                .iter()
                .filter_map(|item| value_type(context, tuples, *item))
                .collect();
            Some(context.struct_type(&fields, false).into())
        }
    }
}

/// Returns the struct field that holds item `index` of a tuple, or `None` for an item of type
/// `None`, which has no field.
pub(crate) fn tuple_field(items: &[TyirType], index: usize) -> Option<u32> {
    if items.get(index).is_none_or(|item| *item == TyirType::Unit) {
        return None;
    }
    let before = items[..index]
        .iter()
        .filter(|item| **item != TyirType::Unit)
        .count();
    u32::try_from(before).ok()
}

/// Returns the type a function returns. A function that may raise returns `{result, exception}`, or only the exception when its result is `unit`; a null exception means none was raised.
pub(crate) fn function_type<'ctx>(
    context: &'ctx Context,
    tuples: &TupleTypes,
    parameters: &[TyirType],
    result: TyirType,
    raising: bool,
    boundary: bool,
) -> FunctionType<'ctx> {
    let parameter_types: Vec<BasicMetadataTypeEnum<'ctx>> = parameters
        .iter()
        .filter_map(|ty| value_type(context, tuples, *ty))
        .map(Into::into)
        .collect();
    let pointer = context.ptr_type(AddressSpace::default());
    // A `bool` crosses into the runtime as one byte, as C's `_Bool` does.
    let result_type = match result {
        TyirType::Bool if boundary => Some(context.i8_type().into()),
        other => value_type(context, tuples, other),
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
    use typy_tyir::{TupleTypes, TyirType};

    #[test]
    fn values_have_their_llvm_types() {
        let context = Context::create();
        assert_eq!(
            value_type(&context, &TupleTypes::default(), TyirType::Int)
                .unwrap()
                .to_string(),
            "\"i64\""
        );
        assert_eq!(
            value_type(&context, &TupleTypes::default(), TyirType::Float)
                .unwrap()
                .to_string(),
            "\"double\""
        );
        assert_eq!(
            value_type(&context, &TupleTypes::default(), TyirType::Bool)
                .unwrap()
                .to_string(),
            "\"i1\""
        );
        assert_eq!(
            value_type(&context, &TupleTypes::default(), TyirType::STR)
                .unwrap()
                .to_string(),
            "\"ptr\""
        );
        assert!(value_type(&context, &TupleTypes::default(), TyirType::Unit).is_none());
    }

    #[test]
    fn a_tuple_is_a_struct_of_its_items_that_have_a_value() {
        let context = Context::create();
        let mut tuples = TupleTypes::default();
        let pair = tuples.intern(vec![TyirType::Int, TyirType::Unit, TyirType::Float]);
        let empty = tuples.intern(Vec::new());
        assert_eq!(
            value_type(&context, &tuples, pair).unwrap().to_string(),
            "\"{ i64, double }\""
        );
        assert_eq!(
            value_type(&context, &tuples, empty).unwrap().to_string(),
            "\"{}\""
        );
        let items = [TyirType::Int, TyirType::Unit, TyirType::Float];
        assert_eq!(super::tuple_field(&items, 0), Some(0));
        assert_eq!(super::tuple_field(&items, 1), None);
        assert_eq!(super::tuple_field(&items, 2), Some(1));
        assert_eq!(super::tuple_field(&items, 3), None);
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
                &TupleTypes::default(),
                &[TyirType::Int, TyirType::Unit],
                result,
                raising,
                boundary,
            );
            assert_eq!(ty.to_string(), format!("\"{expected}\""));
        }
    }
}
