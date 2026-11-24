//! Tests of `self_type`.

use crate::types::Type;

#[test]
fn self_takes_the_type_of_what_a_call_is_made_through() {
    let own = Type::self_of(Type::Int);
    assert!(own.has_self());
    assert_eq!(own.with_self(Some(&Type::Str)), Type::Str);
    assert_eq!(own.with_self(None), Type::Int);
    assert_eq!(
        Type::optional(own.clone()).with_self(Some(&Type::Str)),
        Type::optional(Type::Str)
    );
    let listed = Type::List(Box::new(own.clone()));
    assert_eq!(listed.with_self(None), Type::List(Box::new(Type::Int)));
    assert!(listed.has_nested_self());
    assert!(!Type::optional(own.clone()).has_nested_self());
    assert!(!own.has_nested_self());
    assert_eq!(Type::Float.with_self(Some(&Type::Str)), Type::Float);
    assert_eq!(own.value_type(), Type::Int);
}
