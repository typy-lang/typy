//! The calling convention of generated code (detailed design section 10.2).

use inkwell::AddressSpace;
use inkwell::context::Context;
use inkwell::types::{BasicMetadataTypeEnum, BasicType, BasicTypeEnum, FunctionType};
use typy_tyir::{OptionalBase, TupleTypes, TyirType};

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
        TyirType::Optional(OptionalBase::Bool) => Some(context.i8_type().into()),
        TyirType::Optional(base) => value_type(context, tuples, storage(base)),
        TyirType::Tuple(id) => {
            let items = tuples.items(id).unwrap_or_default();
            let fields: Vec<BasicTypeEnum<'ctx>> = storage_order(tuples, items)
                .into_iter()
                .filter_map(|index| value_type(context, tuples, items[index]))
                .collect();
            Some(context.struct_type(&fields, false).into())
        }
    }
}

/// Returns the type whose layout an optional value of `base` shares, other than a `bool`'s,
/// which widens to a byte: an `int`, a `float`, an object, or the pair of a flag and a tuple.
pub(crate) const fn storage(base: OptionalBase) -> TyirType {
    match base {
        OptionalBase::Int => TyirType::Int,
        OptionalBase::Float => TyirType::Float,
        OptionalBase::Bool => TyirType::Bool,
        OptionalBase::Object(id) => TyirType::Object(id),
        OptionalBase::Tuple(pair) => TyirType::Tuple(pair),
    }
}

/// Returns the alignment in bytes of a value of type `ty` in memory: 8 for an `int`, a `float`,
/// bits and an object, 1 for a `bool` and an optional `bool`, a tuple's most aligned item's,
/// and 0 for `None`, which takes no room. Every supported target aligns these types alike.
pub(crate) fn alignment(tuples: &TupleTypes, ty: TyirType) -> u32 {
    match ty {
        TyirType::Unit => 0,
        TyirType::Bool | TyirType::Optional(OptionalBase::Bool) => 1,
        TyirType::Int | TyirType::Bits | TyirType::Float | TyirType::Object(_) => 8,
        TyirType::Optional(base) => alignment(tuples, storage(base)),
        TyirType::Tuple(id) => tuples
            .items(id)
            .unwrap_or_default()
            .iter()
            .map(|item| alignment(tuples, *item))
            .max()
            .unwrap_or(1)
            .max(1),
    }
}

/// Returns the position of each item of a tuple that takes a field, in the order of the
/// fields: by decreasing alignment, and in the declared order among items of one alignment,
/// so the padding is at the end and the order follows from the types alone (MEM-11,
/// ADR-0062). An item of type `None` takes no field.
pub(crate) fn storage_order(tuples: &TupleTypes, items: &[TyirType]) -> Vec<usize> {
    let mut order: Vec<usize> = (0..items.len())
        .filter(|index| items[*index] != TyirType::Unit)
        .collect();
    order.sort_by_key(|index| core::cmp::Reverse(alignment(tuples, items[*index])));
    order
}

/// Returns the struct field that holds item `index` of a tuple, or `None` for an item of type
/// `None`, which has no field.
pub(crate) fn tuple_field(tuples: &TupleTypes, items: &[TyirType], index: usize) -> Option<u32> {
    if items.get(index).is_none_or(|item| *item == TyirType::Unit) {
        return None;
    }
    let position = storage_order(tuples, items)
        .iter()
        .position(|each| *each == index)?;
    u32::try_from(position).ok()
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
    // A `bool` crosses into the runtime as one byte, as C's `_Bool` does, and a `float` the
    // runtime may raise beside crosses as its bits, which every target returns in an integer
    // register as the runtime's C structure is returned (ADR-0053).
    let result_type = match result {
        TyirType::Bool if boundary => Some(context.i8_type().into()),
        TyirType::Float if boundary && raising => Some(context.i64_type().into()),
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
        assert_eq!(super::tuple_field(&tuples, &items, 0), Some(0));
        assert_eq!(super::tuple_field(&tuples, &items, 1), None);
        assert_eq!(super::tuple_field(&tuples, &items, 2), Some(1));
        assert_eq!(super::tuple_field(&tuples, &items, 3), None);
    }

    #[test]
    fn mem_11_items_are_stored_by_decreasing_alignment_in_their_declared_order() {
        use typy_tyir::OptionalBase;
        let context = Context::create();
        let mut tuples = TupleTypes::default();
        let flagged = tuples.intern(vec![TyirType::Bool, TyirType::Int]);
        let items = vec![
            TyirType::Int,
            TyirType::Bool,
            TyirType::Float,
            TyirType::Optional(OptionalBase::Bool),
            TyirType::Unit,
            flagged,
            TyirType::STR,
        ];
        let node = tuples.intern(items.clone());
        assert_eq!(
            value_type(&context, &tuples, node).unwrap().to_string(),
            "\"{ i64, double, { i64, i1 }, ptr, i1, i8 }\""
        );
        let fields: Vec<Option<u32>> = (0..items.len())
            .map(|index| super::tuple_field(&tuples, &items, index))
            .collect();
        assert_eq!(
            fields,
            [Some(0), Some(4), Some(1), Some(5), None, Some(2), Some(3)]
        );
        assert_eq!(super::alignment(&tuples, flagged), 8);
        assert_eq!(super::alignment(&tuples, TyirType::Unit), 0);
        let empty = tuples.intern(Vec::new());
        assert_eq!(super::alignment(&tuples, empty), 1);
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
            (TyirType::Float, false, true, "double (i64)"),
            (TyirType::Float, true, false, "{ double, ptr } (i64)"),
            (TyirType::Float, true, true, "{ i64, ptr } (i64)"),
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
