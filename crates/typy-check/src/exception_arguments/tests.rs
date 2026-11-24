//! Tests of `exception_arguments`.

use super::Creations;
use crate::exception::{BuiltinException, ExceptionClass};
use crate::types::Type;

fn builtin(class: BuiltinException) -> ExceptionClass {
    ExceptionClass::Builtin(class)
}

fn tuple(items: &[Type]) -> Type {
    Type::Tuple(items.to_vec())
}

/// Returns the members of a union.
fn members(ty: Type) -> Vec<Type> {
    match ty {
        Type::Union(members) => members,
        other => panic!("a union, not {other:?}"),
    }
}

#[test]
fn a_base_class_has_the_unicode_errors_five_arguments_when_the_program_can_cause_them() {
    let value_error = builtin(BuiltinException::ValueError);
    let mut creations = Creations::default();
    assert_eq!(creations.arguments_of(&value_error), tuple(&[Type::Str]));
    creations.codec(false, true);
    assert_eq!(
        creations.arguments_of(&value_error),
        Type::Union(vec![
            tuple(&[Type::Str, Type::Bytes, Type::Int, Type::Int, Type::Str]),
            tuple(&[Type::Str])
        ])
    );
    creations.codec(true, false);
    let members = members(creations.arguments_of(&value_error));
    let object = Type::union(vec![Type::Str, Type::Bytes]);
    assert_eq!(
        members,
        [
            tuple(&[Type::Str, object, Type::Int, Type::Int, Type::Str]),
            tuple(&[Type::Str])
        ]
    );
}

#[test]
fn a_class_the_program_never_creates_has_the_runtime_message() {
    let creations = Creations::default();
    assert_eq!(
        creations.arguments_of(&builtin(BuiltinException::TypeError)),
        tuple(&[Type::Str])
    );
    assert_eq!(
        creations.arguments_of(&builtin(BuiltinException::StopIteration)),
        tuple(&[])
    );
}

#[test]
fn calls_with_other_layouts_become_members_of_a_union() {
    let mut creations = Creations::default();
    let type_error = builtin(BuiltinException::TypeError);
    creations.make(&type_error, &[Type::Str, Type::Int]);
    creations.make(&type_error, &[Type::Int]);
    let members = members(creations.arguments_of(&type_error));
    assert_eq!(members.len(), 3, "{members:?}");
    assert!(members.contains(&tuple(&[Type::Str, Type::Int])));
    assert!(members.contains(&tuple(&[Type::Int])));
}

#[test]
fn objects_at_one_position_share_a_member() {
    let mut creations = Creations::default();
    let type_error = builtin(BuiltinException::TypeError);
    creations.make(&type_error, &[Type::List(Box::new(Type::Int))]);
    assert_eq!(
        creations.arguments_of(&type_error),
        tuple(&[Type::union(vec![
            Type::Str,
            Type::List(Box::new(Type::Int))
        ])])
    );
}

#[test]
fn an_int_and_a_float_stay_apart_since_their_layouts_differ() {
    let mut creations = Creations::default();
    let class = builtin(BuiltinException::AssertionError);
    creations.make(&class, &[Type::Int]);
    creations.make(&class, &[Type::Float]);
    assert_eq!(
        creations.arguments_of(&class),
        Type::Union(vec![tuple(&[Type::Float]), tuple(&[Type::Int])])
    );
}

#[test]
fn a_base_class_has_the_arguments_of_its_derived_classes() {
    let mut creations = Creations::default();
    creations.key(&Type::Int);
    let lookup = creations.arguments_of(&builtin(BuiltinException::LookupError));
    assert_eq!(
        lookup,
        Type::Union(vec![tuple(&[Type::Int]), tuple(&[Type::Str])])
    );
    assert_eq!(
        creations.arguments_of(&builtin(BuiltinException::IndexError)),
        tuple(&[Type::Str])
    );
}

#[test]
fn an_os_error_with_a_file_name_keeps_two_arguments() {
    let mut creations = Creations::default();
    let class = builtin(BuiltinException::FileNotFoundError);
    creations.make(&class, &[Type::Int, Type::Str, Type::Str]);
    creations.make(&class, &[Type::Int, Type::Str, Type::NoneType]);
    let members = members(creations.arguments_of(&class));
    assert!(members.contains(&tuple(&[Type::Int, Type::Str])));
    assert!(members.contains(&tuple(&[Type::Int, Type::Str, Type::NoneType])));
    assert_eq!(members.len(), 3, "{members:?}");
}

#[test]
fn an_initializer_applies_to_its_bases_and_derived_classes() {
    let mut creations = Creations::default();
    let base = ExceptionClass::program(
        crate::typed::ClassId(0),
        "Base".to_owned(),
        Vec::new(),
        BuiltinException::Exception,
    );
    let derived = ExceptionClass::program(
        crate::typed::ClassId(1),
        "Derived".to_owned(),
        vec![(crate::typed::ClassId(0), "Base".to_owned())],
        BuiltinException::Exception,
    );
    creations.initialize(&base, &[Type::Float]);
    creations.make(&derived, &[]);
    assert_eq!(
        creations.arguments_of(&derived),
        Type::Union(vec![tuple(&[Type::Float]), tuple(&[])])
    );
}

#[test]
fn an_os_error_that_becomes_a_subclass_applies_to_the_subclasses() {
    let mut creations = Creations::default();
    let os_error = builtin(BuiltinException::OSError);
    creations.make(&os_error, &[Type::Int, Type::Str, Type::NoneType]);
    creations.make(&os_error, &[Type::Float]);
    let members = members(creations.arguments_of(&builtin(BuiltinException::FileNotFoundError)));
    assert!(members.contains(&tuple(&[Type::Int, Type::Str, Type::NoneType])));
    assert!(!members.contains(&tuple(&[Type::Float])));
}

#[test]
fn covering_and_extending_compare_every_creation() {
    let mut first = Creations::default();
    let mut second = Creations::default();
    second.key(&Type::Str);
    second.codec(true, false);
    assert!(!first.covers(&second));
    first.extend(&second);
    assert!(first.covers(&second));
    assert_eq!(first, second);
}

#[test]
fn creations_cover_others_only_with_each_call_and_each_codec() {
    let mut made = Creations::default();
    made.make(&builtin(BuiltinException::TypeError), &[Type::Int]);
    let mut initialized = Creations::default();
    initialized.initialize(&builtin(BuiltinException::TypeError), &[Type::Int]);
    let mut encodes = Creations::default();
    encodes.codec(true, false);
    let mut decodes = Creations::default();
    decodes.codec(false, true);
    let none = Creations::default();
    for other in [&made, &initialized, &encodes, &decodes] {
        assert!(!none.covers(other), "{other:?}");
        assert!(other.covers(&none), "{other:?}");
    }
    assert!(!encodes.covers(&decodes));
    assert!(!decodes.covers(&encodes));
}

#[test]
fn a_key_of_an_error_type_is_not_recorded() {
    let mut creations = Creations::default();
    creations.key(&Type::Error);
    creations.key(&Type::List(Box::new(Type::Error)));
    assert_eq!(creations, Creations::default());
    assert_eq!(
        creations.arguments_of(&builtin(BuiltinException::KeyError)),
        tuple(&[Type::Str])
    );
}

#[test]
fn exp_22_a_variadic_tuple_takes_in_the_fixed_tuples_held_as_its_items() {
    let mut creations = Creations::default();
    let class = builtin(BuiltinException::ValueError);
    let numbers = Type::List(Box::new(Type::Int));
    creations.make_variadic(&class, &numbers);
    creations.make(&class, &[Type::Str, numbers.clone()]);
    creations.make(&class, &[]);
    assert_eq!(
        creations.arguments_of(&class),
        Type::TupleOf(Box::new(Type::union(vec![numbers, Type::Str])))
    );
}

#[test]
fn exp_22_a_variadic_tuple_leaves_apart_the_tuples_of_other_layouts() {
    let mut creations = Creations::default();
    let class = builtin(BuiltinException::ValueError);
    creations.make_variadic(&class, &Type::Int);
    creations.make_variadic(&class, &Type::Int);
    creations.make(&class, &[Type::Int, Type::Float]);
    let members = members(creations.arguments_of(&class));
    assert_eq!(members.len(), 3, "{members:?}");
    assert!(members.contains(&Type::TupleOf(Box::new(Type::Int))));
    assert!(members.contains(&tuple(&[Type::Int, Type::Float])));
    assert!(members.contains(&tuple(&[Type::Str])));
}

#[test]
fn exp_22_variadic_tuples_of_objects_join_and_others_stay_apart() {
    let mut creations = Creations::default();
    let class = builtin(BuiltinException::RuntimeError);
    let names = Type::List(Box::new(Type::Str));
    creations.make_variadic(&class, &Type::Bool);
    creations.make_variadic(&class, &names);
    creations.make_variadic(&class, &Type::Str);
    let members = members(creations.arguments_of(&class));
    assert_eq!(
        members,
        [
            Type::TupleOf(Box::new(Type::Bool)),
            Type::TupleOf(Box::new(Type::union(vec![names, Type::Str]))),
        ]
    );
}

#[test]
fn an_os_error_of_one_or_two_arguments_shows_them_all() {
    let mut creations = Creations::default();
    let class = builtin(BuiltinException::FileNotFoundError);
    creations.make(&class, &[Type::Float]);
    creations.make(&class, &[Type::Int, Type::Int]);
    let members = members(creations.arguments_of(&class));
    for shown in [
        tuple(&[Type::Float]),
        tuple(&[Type::Int, Type::Int]),
        tuple(&[Type::Int, Type::Str]),
    ] {
        assert!(members.contains(&shown), "{shown:?} in {members:?}");
    }
}
