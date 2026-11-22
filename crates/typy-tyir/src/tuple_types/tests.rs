//! Tests of `tuple_types`.

use super::TupleTypes;
use crate::types::{TupleTypeId, TyirType};

/// Returns the number of a tuple type that interning gave.
fn tuple_id(ty: TyirType) -> TupleTypeId {
    match ty {
        TyirType::Tuple(id) => id,
        other => panic!("interning gave {other:?}"),
    }
}

#[test]
fn a_union_is_a_tuple_of_its_own() {
    let mut types = TupleTypes::default();
    let tuple = types.intern(vec![TyirType::Int, TyirType::Int, TyirType::STR]);
    let union = types.intern_union(&[TyirType::Int, TyirType::STR]);
    assert_ne!(tuple, union);
    assert_eq!(types.intern_union(&[TyirType::Int, TyirType::STR]), union);
    let (tuple, union) = (tuple_id(tuple), tuple_id(union));
    assert!(types.is_union(union));
    assert!(!types.is_union(tuple));
    assert!(!types.is_union(TupleTypeId(9)));
    assert_eq!(types.items(union), types.items(tuple));
}

#[test]
fn equal_item_lists_share_one_number() {
    let mut types = TupleTypes::default();
    let first = types.intern(vec![TyirType::Int, TyirType::STR]);
    let second = types.intern(vec![TyirType::Float]);
    assert_eq!(types.intern(vec![TyirType::Int, TyirType::STR]), first);
    assert_ne!(first, second);
    assert_eq!(types.items(TupleTypeId(1)), Some(&[TyirType::Float][..]));
    assert_eq!(types.items(TupleTypeId(9)), None);
    assert_eq!(types.iter().count(), 2);
}

#[test]
fn a_tuple_is_counted_when_an_item_is() {
    let mut types = TupleTypes::default();
    let floats = types.intern(vec![TyirType::Float, TyirType::Bool]);
    let strings = types.intern(vec![TyirType::STR]);
    let nested = types.intern(vec![TyirType::Float, strings]);
    assert!(!types.is_counted(floats));
    assert!(types.is_counted(nested));
    assert!(types.is_counted(TyirType::Int));
    assert!(types.is_counted(TyirType::STR));
    assert!(!types.is_counted(TyirType::Unit));
    assert!(!types.is_counted(TyirType::Tuple(TupleTypeId(9))));
}
