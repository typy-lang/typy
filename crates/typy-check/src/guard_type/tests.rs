//! Tests of `guard_type`.

use super::GuardKind;
use crate::types::Type;

#[test]
fn a_guard_is_written_as_its_form_and_held_as_a_bool() {
    let guard = Type::guard(GuardKind::TypeIs, Type::Int);
    assert_eq!(guard.to_string(), "TypeIs[int]");
    assert_eq!(
        Type::guard(GuardKind::TypeGuard, Type::Str).to_string(),
        "TypeGuard[str]"
    );
    assert_eq!(guard.without_guard(), Type::Bool);
    assert_eq!(guard.value_type(), Type::Bool);
    assert_eq!(guard.without_declared_forms(), Type::Bool);
    assert!(Type::Bool.is_assignable_to(&guard));
    assert!(guard.is_assignable_to(&Type::Bool));
    assert_eq!(Type::Int.without_guard(), Type::Int);
}
