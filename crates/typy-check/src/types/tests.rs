//! Tests of `types`.

use super::Type;

#[test]
fn an_int_is_not_assignable_to_a_float() {
    assert!(!Type::Int.is_assignable_to(&Type::Float));
    assert!(!Type::Float.is_assignable_to(&Type::Int));
    assert!(!Type::Bool.is_assignable_to(&Type::Int));
}

#[test]
fn equal_types_and_error_are_assignable() {
    for ty in [
        Type::Int,
        Type::Float,
        Type::Bool,
        Type::Str,
        Type::NoneType,
    ] {
        assert!(ty.is_assignable_to(&ty));
        assert!(Type::Error.is_assignable_to(&ty));
        assert!(ty.is_assignable_to(&Type::Error));
        assert!(ty.is_truth_testable());
        assert!(ty.is_printable());
    }
}

#[test]
fn instances_compare_by_identity_and_have_no_order() {
    use crate::class_type::InstanceClass;
    use crate::typed::ClassId;
    let point = Type::instance(InstanceClass::root(ClassId(0), "Point"));
    let item = Type::instance(InstanceClass::root(ClassId(1), "Item"));
    assert!(point.can_test_equality(&point));
    assert!(!point.can_test_equality(&item));
    assert!(!point.can_order(&point));
    assert!(!Type::Int.can_order(&point));
    assert!(point.is_hashable());
    assert!(point.is_assignable_to(&Type::optional(point.clone())));
    assert_eq!(
        (point.with_article(), item.with_article()),
        ("a Point".to_owned(), "an Item".to_owned())
    );
    assert_eq!(Type::optional(item).to_string(), "Item | None");
}

#[test]
fn names_take_the_right_article() {
    assert_eq!(Type::Int.with_article(), "an int");
    assert_eq!(Type::Str.with_article(), "a str");
    assert_eq!(Type::NoneType.with_article(), "None");
}

#[test]
fn only_int_and_float_are_numeric() {
    let numeric: Vec<bool> = [
        Type::Int,
        Type::Float,
        Type::Bool,
        Type::Str,
        Type::NoneType,
    ]
    .iter()
    .map(Type::is_numeric)
    .collect::<Vec<_>>();
    assert_eq!(numeric, [true, true, false, false, false]);
}

#[test]
fn types_are_written_with_their_python_names() {
    let names: Vec<String> = [
        Type::Int,
        Type::Float,
        Type::Bool,
        Type::Str,
        Type::NoneType,
    ]
    .iter()
    .map(ToString::to_string)
    .collect();
    assert_eq!(names, ["int", "float", "bool", "str", "None"]);
}

#[test]
fn lists_sets_and_tuples_that_hold_them_have_no_hash() {
    let list = Type::List(Box::new(Type::Int));
    let set = Type::Set(Box::new(Type::Int));
    let cases = [
        (Type::Int, true),
        (Type::Float, true),
        (Type::Bytes, true),
        (Type::NoneType, true),
        (Type::Tuple(vec![Type::Int, Type::Str]), true),
        (Type::TupleOf(Box::new(Type::Float)), true),
        (list.clone(), false),
        (set.clone(), false),
        (Type::Tuple(vec![Type::Int, list]), false),
        (Type::TupleOf(Box::new(set)), false),
    ];
    for (ty, hashable) in cases {
        assert_eq!(ty.is_hashable(), hashable, "{ty}");
    }
}

#[test]
fn sets_compare_and_order_only_with_sets_of_their_item_type() {
    let ints = Type::Set(Box::new(Type::Int));
    let floats = Type::Set(Box::new(Type::Float));
    assert_eq!(ints.to_string(), "set[int]");
    assert!(ints.can_test_equality(&ints));
    assert!(ints.can_order(&ints));
    assert!(!ints.can_test_equality(&floats));
    assert!(!ints.can_order(&floats));
    assert!(ints.is_assignable_to(&ints));
    assert!(!ints.is_assignable_to(&floats));
    assert!(Type::Set(Box::new(Type::Error)).has_error());
}

#[test]
fn dicts_and_views_follow_their_key_and_value_types() {
    use super::View;
    let dict = |key: Type, value: Type| Type::Dict(Box::new(key), Box::new(value));
    let ints = dict(Type::Int, Type::Int);
    let view = |kind| Type::DictView(kind, Box::new(Type::Int), Box::new(Type::Str));
    assert_eq!(ints.to_string(), "dict[int, int]");
    assert_eq!(view(View::Items).to_string(), "dict_items[int, str]");
    assert_eq!(view(View::Keys).to_string(), "dict_keys[int, str]");
    assert_eq!(view(View::Values).to_string(), "dict_values[int, str]");
    assert!(ints.is_assignable_to(&ints));
    assert!(!ints.is_assignable_to(&dict(Type::Int, Type::Float)));
    assert!(!ints.is_assignable_to(&dict(Type::Str, Type::Int)));
    assert!(dict(Type::Error, Type::Int).has_error());
    assert!(dict(Type::Int, Type::Error).has_error());
    assert!(ints.can_test_equality(&ints));
    assert!(!ints.can_test_equality(&dict(Type::Int, Type::Str)));
    assert!(!view(View::Keys).can_test_equality(&view(View::Keys)));
    assert!(!Type::Int.can_test_equality(&view(View::Keys)));
    assert!(!ints.can_order(&ints));
    assert!(!Type::Int.can_order(&view(View::Values)));
    assert!(!ints.is_hashable());
    assert!(!view(View::Keys).is_hashable());
    let lists = |item: Type| Type::List(Box::new(item));
    assert!(lists(Type::Int).can_order(&lists(Type::Int)));
    assert!(!lists(ints.clone()).can_order(&lists(ints)));
    assert_eq!(
        [View::Keys, View::Values, View::Items].map(View::code),
        [0, 1, 2]
    );
    assert_eq!(
        View::Items.item(&Type::Int, &Type::Str),
        Type::Tuple(vec![Type::Int, Type::Str])
    );
    assert_eq!(View::Values.item(&Type::Int, &Type::Str), Type::Str);
}

#[test]
fn tuples_are_written_compared_and_ordered_item_by_item() {
    let pair = Type::Tuple(vec![Type::Int, Type::Str]);
    let nones = Type::Tuple(vec![Type::NoneType, Type::Int]);
    assert_eq!(Type::Tuple(Vec::new()).to_string(), "tuple[()]");
    assert_eq!(pair.to_string(), "tuple[int, str]");
    assert!(!pair.is_assignable_to(&Type::Tuple(vec![Type::Int])));
    assert!(!pair.can_test_equality(&Type::Tuple(vec![Type::Int])));
    assert!(pair.can_test_equality(&Type::Tuple(vec![Type::Float, Type::Str])));
    assert!(nones.can_order(&nones));
    assert!(!pair.can_order(&Type::Tuple(vec![Type::Int])));
    assert!(Type::Int.can_order(&Type::Float));
    assert!(!Type::Str.can_order(&Type::Int));
    assert!(Type::Str.can_test_equality(&Type::Str));
    assert!(!Type::Str.can_test_equality(&Type::Int));
}

#[test]
fn optional_types_join_none_and_keep_containers_exact() {
    let optional = |inner: Type| Type::optional(inner);
    let int = optional(Type::Int);
    assert_eq!(int.to_string(), "int | None");
    assert_eq!(optional(Type::NoneType), Type::NoneType);
    assert_eq!(optional(int.clone()), int);
    assert_eq!(optional(Type::Error), Type::Error);
    assert!(Type::NoneType.is_assignable_to(&int));
    assert!(Type::Int.is_assignable_to(&int));
    assert!(!Type::Str.is_assignable_to(&int));
    assert!(int.is_assignable_to(&int));
    assert!(!int.is_assignable_to(&Type::Int));
    assert!(!int.is_assignable_to(&optional(Type::Float)));
    let list = |item: Type| Type::List(Box::new(item));
    assert!(!list(Type::Int).is_assignable_to(&list(int.clone())));
    assert!(list(int.clone()).matches(&list(int.clone())));
    let view = |view, key: Type| Type::DictView(view, Box::new(key), Box::new(Type::Int));
    assert!(view(super::View::Keys, Type::Str).matches(&view(super::View::Keys, Type::Str)));
    assert!(!view(super::View::Keys, Type::Str).matches(&view(super::View::Values, Type::Str)));
    assert!(!view(super::View::Keys, Type::Str).matches(&view(super::View::Keys, Type::Int)));
    let pair = Type::Tuple(vec![Type::Int, Type::Str]);
    assert!(!pair.matches(&Type::Tuple(vec![Type::Int])));
    assert!(pair.is_assignable_to(&optional(pair.clone())));
    assert!(optional(Type::Error).has_error());
    assert!(Type::Optional(Box::new(Type::Error)).has_error());
    assert!(int.is_hashable());
    assert!(!int.can_order(&int));
    // `None` equals only `None`, and two values compare as their base does (EXP-06).
    assert!(int.can_test_equality(&int));
    assert!(Type::Int.can_test_equality(&int));
    assert!(int.can_test_equality(&Type::NoneType));
    assert!(!int.can_test_equality(&Type::Str));
    assert!(!int.can_test_equality(&optional(Type::Str)));
}
