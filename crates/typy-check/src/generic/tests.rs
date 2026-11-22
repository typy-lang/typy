//! Tests of `generic`.

use super::{TypeParameter, infer};
use crate::types::Type;
use typy_diagnostics::{FileId, Span, TextOffset};

fn parameter(index: usize, bound: Option<Type>) -> Type {
    Type::parameter(TypeParameter {
        owner: Span::empty(FileId(0), TextOffset(4)),
        index,
        name: format!("T{index}"),
        bound,
        constraints: Vec::new(),
    })
}

fn owner() -> Span {
    Span::empty(FileId(0), TextOffset(4))
}

#[test]
fn arguments_solve_the_parameters_they_meet() {
    let (first, second) = (parameter(0, None), parameter(1, None));
    let mut solved = vec![None, None];
    let pairs = Type::Dict(
        Box::new(first.clone()),
        Box::new(Type::List(Box::new(second.clone()))),
    );
    let given = Type::Dict(
        Box::new(Type::Str),
        Box::new(Type::List(Box::new(Type::Int))),
    );
    infer(&pairs, &given, owner(), &mut solved);
    assert_eq!(solved, vec![Some(Type::Str), Some(Type::Int)]);
    let mut solved = vec![None, None];
    infer(&first, &Type::Int, owner(), &mut solved);
    infer(&first, &Type::Str, owner(), &mut solved);
    infer(&first, &Type::Int, owner(), &mut solved);
    assert_eq!(solved[0], Some(Type::union(vec![Type::Int, Type::Str])));
    let optional = Type::optional(second.clone());
    infer(&optional, &Type::NoneType, owner(), &mut solved);
    assert_eq!(solved[1], None);
    infer(&optional, &Type::Bytes, owner(), &mut solved);
    assert_eq!(solved[1], Some(Type::Bytes));
}

#[test]
fn a_parameter_is_held_as_itself_or_where_its_bound_is() {
    let free = parameter(0, None);
    let bounded = parameter(1, Some(Type::Str));
    assert!(free.is_assignable_to(&free));
    assert!(!free.is_assignable_to(&Type::Str));
    assert!(bounded.is_assignable_to(&Type::Str));
    assert!(bounded.is_upcast_to(&Type::Str));
    assert!(!bounded.is_upcast_to(&bounded));
    assert!(!Type::Str.is_assignable_to(&bounded));
    assert_eq!(bounded.with_article(), "a value of type T1");
    // A parameter of another definition is neither solved nor replaced here.
    let other = Type::parameter(TypeParameter {
        owner: Span::empty(FileId(0), TextOffset(9)),
        index: 0,
        name: "U".to_owned(),
        bound: None,
        constraints: Vec::new(),
    });
    let mut solved = vec![None];
    infer(&other, &Type::Int, owner(), &mut solved);
    assert_eq!(solved, vec![None]);
    assert_eq!(other.substituted(owner(), &[Type::Int]), other);
    // A wider second value widens the solution.
    let mut solved = vec![Some(Type::Bool)];
    infer(
        &free,
        &Type::union(vec![Type::Bool, Type::Str]),
        owner(),
        &mut solved,
    );
    assert_eq!(solved, vec![Some(Type::union(vec![Type::Bool, Type::Str]))]);
}

#[test]
fn every_shape_of_parameter_solves_from_its_argument() {
    let first = parameter(0, None);
    let pair = Type::Tuple(vec![first.clone(), Type::Str]);
    let mut solved = vec![None];
    infer(&pair, &Type::Tuple(vec![Type::Int]), owner(), &mut solved);
    assert_eq!(solved, vec![None]);
    infer(
        &pair,
        &Type::Tuple(vec![Type::Int, Type::Str]),
        owner(),
        &mut solved,
    );
    assert_eq!(solved, vec![Some(Type::Int)]);
    let mut solved = vec![None];
    let items = Type::Iterable(Box::new(first.clone()));
    infer(&items, &Type::Int, owner(), &mut solved);
    assert_eq!(solved, vec![None]);
    infer(
        &items,
        &Type::List(Box::new(Type::Bytes)),
        owner(),
        &mut solved,
    );
    assert_eq!(solved, vec![Some(Type::Bytes)]);
    let mut solved = vec![None, None];
    let either = Type::union(vec![first.clone(), Type::Str]);
    infer(&either, &Type::Str, owner(), &mut solved);
    assert_eq!(solved, vec![None, None]);
    infer(&either, &Type::Int, owner(), &mut solved);
    assert_eq!(solved[0], Some(Type::Int));
    let two = Type::union(vec![first.clone(), parameter(1, None)]);
    let mut solved = vec![None, None];
    infer(&two, &Type::Int, owner(), &mut solved);
    assert_eq!(solved, vec![None, None]);
    let of_tuple = Type::TupleOf(Box::new(first.clone()));
    let mut solved = vec![None];
    infer(
        &of_tuple,
        &Type::Tuple(vec![Type::Int, Type::Int]),
        owner(),
        &mut solved,
    );
    assert_eq!(solved, vec![Some(Type::Int)]);
    let copy = |root: u32, argument: Type| {
        let mut class = crate::class_type::InstanceClass::root(crate::typed::ClassId(9), "Box");
        class.generic = Some((crate::typed::ClassId(root), vec![argument]));
        Type::instance(class)
    };
    let mut solved = vec![None];
    infer(
        &copy(1, first.clone()),
        &copy(2, Type::Int),
        owner(),
        &mut solved,
    );
    assert_eq!(solved, vec![None]);
    infer(
        &copy(1, first.clone()),
        &copy(1, Type::Int),
        owner(),
        &mut solved,
    );
    assert_eq!(solved, vec![Some(Type::Int)]);
    assert!(copy(1, first).has_parameter());
}

#[test]
fn a_substitution_replaces_each_parameter_in_place() {
    let first = parameter(0, None);
    let shape = Type::Tuple(vec![
        Type::List(Box::new(first.clone())),
        Type::optional(first.clone()),
        Type::Str,
    ]);
    let made = shape.substituted(owner(), &[Type::Int]);
    assert_eq!(
        made,
        Type::Tuple(vec![
            Type::List(Box::new(Type::Int)),
            Type::optional(Type::Int),
            Type::Str
        ])
    );
    assert!(shape.has_parameter());
    assert!(!made.has_parameter());
    assert_eq!(first.substituted(owner(), &[]), first);
}
