//! Tests of `abi`.

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
