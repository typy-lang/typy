//! Tests of `union_type`.

use crate::class_type::InstanceClass;
use crate::typed::ClassId;
use crate::types::Type;

fn class(id: u32, name: &str, ancestors: &[(u32, &str)]) -> Type {
    let mut class = InstanceClass::root(ClassId(id), name);
    class.ancestors = ancestors
        .iter()
        .map(|(id, name)| (ClassId(*id), (*name).to_owned()))
        .collect();
    Type::instance(class)
}

#[test]
fn a_union_has_one_normal_form() {
    let list = Type::List(Box::new(Type::Int));
    let forward = Type::union(vec![Type::Str, list.clone()]);
    let backward = Type::union(vec![list.clone(), Type::Str, Type::Str]);
    assert_eq!(forward, backward);
    assert_eq!(forward.members().len(), 2);
    assert_eq!(Type::union(vec![Type::Str]), Type::Str);
    assert_eq!(Type::union(vec![Type::Str, Type::Error]), Type::Error);
    assert_eq!(Type::union(vec![Type::NoneType]), Type::NoneType);
    let nested = Type::union(vec![forward.clone(), Type::NoneType]);
    assert_eq!(nested, Type::optional(forward.clone()));
    let again = Type::union(vec![nested, Type::Str]);
    assert_eq!(again, Type::optional(forward));
}

#[test]
fn a_member_another_member_accepts_is_dropped() {
    let shape = class(0, "Shape", &[]);
    let circle = class(1, "Circle", &[(0, "Shape")]);
    assert_eq!(Type::union(vec![circle, shape.clone()]), shape);
}

#[test]
fn an_optional_value_is_held_only_where_none_is() {
    let either = Type::union(vec![Type::Int, Type::Str]);
    let maybe = Type::optional(Type::Int);
    assert!(!maybe.is_assignable_to(&either));
    assert!(!maybe.is_upcast_to(&either));
    assert!(maybe.is_assignable_to(&Type::optional(either.clone())));
    assert!(Type::Int.is_assignable_to(&either));
}

#[test]
fn instances_of_one_base_share_it() {
    let circle = class(1, "Circle", &[(0, "Shape")]);
    let square = class(2, "Square", &[(0, "Shape")]);
    let both = Type::union(vec![circle, square.clone()]);
    assert_eq!(both.union_base().map(|base| base.id), Some(ClassId(0)));
    let unrelated = Type::union(vec![class(3, "Other", &[]), square]);
    assert_eq!(unrelated.union_base(), None);
    assert_eq!(Type::union(vec![Type::Str, both]).union_base(), None);
    assert_eq!(Type::Str.union_base(), None);
    assert!(Type::union(vec![Type::Str, Type::Bytes]).is_object());
    assert!(!Type::Int.is_object());
}
