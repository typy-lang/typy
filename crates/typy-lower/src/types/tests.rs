//! Tests of `types`.

use super::tyir_type;
use typy_check::Type;
use typy_tyir::{TupleTypes, TyirType};

#[test]
fn each_checked_type_has_its_tyir_type() {
    let mut tuples = TupleTypes::default();
    assert_eq!(tyir_type(&Type::Int, &mut tuples), TyirType::Int);
    assert_eq!(tyir_type(&Type::Float, &mut tuples), TyirType::Float);
    assert_eq!(tyir_type(&Type::Bool, &mut tuples), TyirType::Bool);
    assert_eq!(tyir_type(&Type::Str, &mut tuples), TyirType::STR);
    assert_eq!(tyir_type(&Type::NoneType, &mut tuples), TyirType::Unit);
    let pair = tyir_type(&Type::Tuple(vec![Type::Int, Type::Str]), &mut tuples);
    assert_eq!(pair, tuples.intern(vec![TyirType::Int, TyirType::STR]));
}

#[test]
fn a_union_is_held_as_its_members_share_or_as_any_object() {
    use typy_check::{ClassId, InstanceClass};
    use typy_tyir::TypeId;
    let mut tuples = TupleTypes::default();
    let mixed = Type::union(vec![Type::Str, Type::List(Box::new(Type::Int))]);
    assert_eq!(tyir_type(&mixed, &mut tuples), TyirType::UNION);
    let text = Type::union(vec![
        Type::List(Box::new(Type::Str)),
        Type::TupleOf(Box::new(Type::Int)),
    ]);
    assert_eq!(tyir_type(&text, &mut tuples), TyirType::LIST);
    let derived = |id: u32, name: &str| {
        let mut class = InstanceClass::root(ClassId(id), name);
        class.ancestors = vec![(ClassId(0), "Shape".to_owned())];
        Type::instance(class)
    };
    let shapes = Type::union(vec![derived(1, "Circle"), derived(2, "Square")]);
    assert_eq!(
        tyir_type(&shapes, &mut tuples),
        TyirType::Object(TypeId::class(0))
    );
}
