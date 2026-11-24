//! Tests of `callable_type`.

use std::sync::Arc;

use crate::class_type::InstanceClass;
use crate::typed::ClassId;
use crate::types::Type;

fn callable(parameters: Vec<Type>, returns: Type) -> Type {
    Type::Callable {
        parameters,
        returns: Box::new(returns),
    }
}

fn class(id: u32, name: &str, base: Option<&Arc<InstanceClass>>) -> Arc<InstanceClass> {
    let mut class = InstanceClass::root(ClassId(id), name);
    if let Some(base) = base {
        class.ancestors = std::iter::once((base.id, base.name.clone()))
            .chain(base.ancestors.clone())
            .collect();
    }
    Arc::new(class)
}

#[test]
fn nfr_19_a_function_fits_a_callable_of_the_same_types() {
    let own = callable(vec![Type::Int, Type::Str], Type::Bool);
    assert!(own.is_assignable_to(&own.clone()));
    assert!(!own.is_assignable_to(&callable(vec![Type::Int], Type::Bool)));
    assert!(!own.is_assignable_to(&callable(vec![Type::Int, Type::Str], Type::Int)));
}

#[test]
fn nfr_19_no_parameter_or_result_is_converted_through_a_callable() {
    // Each pair is held differently, so the code would read a value as another type; a
    // call by name converts or refuses each, which a call through a value cannot do.
    let takes_float = callable(vec![Type::Float], Type::NoneType);
    assert!(
        !callable(vec![Type::Float], Type::NoneType)
            .is_assignable_to(&callable(vec![Type::Int], Type::NoneType))
    );
    let takes_optional = callable(vec![Type::optional(Type::Int)], Type::NoneType);
    assert!(!takes_optional.is_assignable_to(&callable(vec![Type::Int], Type::NoneType)));
    let returns_int = callable(vec![], Type::Int);
    assert!(!returns_int.is_assignable_to(&callable(vec![], Type::optional(Type::Int))));
    let union = Type::union(vec![Type::Int, Type::Str]);
    assert!(
        !callable(vec![union], Type::NoneType)
            .is_assignable_to(&callable(vec![Type::Int], Type::NoneType))
    );
    assert!(takes_float.is_assignable_to(&takes_float.clone()));
}

#[test]
fn nfr_19_parameters_take_subclasses_and_results_give_them() {
    let shape = class(1, "Shape", None);
    let circle = class(2, "Circle", Some(&shape));
    let (shape, circle) = (Type::Instance(shape), Type::Instance(circle));
    // A function of any shape may be called with a circle, never the other way.
    let of_shape = callable(vec![shape.clone()], Type::NoneType);
    let of_circle = callable(vec![circle.clone()], Type::NoneType);
    assert!(of_shape.is_assignable_to(&of_circle));
    assert!(!of_circle.is_assignable_to(&of_shape));
    // A function that makes a circle makes a shape, never the other way.
    let makes_circle = callable(vec![], circle.clone());
    let makes_shape = callable(vec![], shape.clone());
    assert!(makes_circle.is_assignable_to(&makes_shape));
    assert!(!makes_shape.is_assignable_to(&makes_circle));
    let optional = callable(vec![], Type::optional(circle));
    assert!(optional.is_assignable_to(&callable(vec![], Type::optional(shape))));
}

#[test]
fn nfr_19_a_callable_parameter_is_matched_by_the_same_rule() {
    let of_int = callable(vec![Type::Int], Type::Int);
    let apply = callable(vec![of_int.clone(), Type::Int], Type::Int);
    assert!(apply.is_assignable_to(&apply.clone()));
    let of_str = callable(vec![Type::Str], Type::Int);
    assert!(!apply.is_assignable_to(&callable(vec![of_str, Type::Int], Type::Int)));
    assert_eq!(
        apply.to_string(),
        "Callable[[Callable[[int], int], int], int]"
    );
    assert_eq!(
        apply
            .callable_parts()
            .map(|(parameters, _)| parameters.len()),
        Some(2)
    );
    assert!(Type::Int.callable_parts().is_none());
    assert!(!Type::Int.fits_callable(&of_int));
}

#[test]
fn fun_05_function_values_compare_by_identity_and_never_order() {
    let first = callable(vec![Type::Int], Type::Int);
    let second = callable(vec![], Type::Str);
    assert!(first.can_test_equality(&second));
    assert!(!first.can_order(&first.clone()));
    assert!(first.is_hashable());
    assert!(!callable(vec![Type::Error], Type::Int).fits_callable(&Type::Int));
    assert!(callable(vec![Type::Error], Type::Int).has_error());
}
