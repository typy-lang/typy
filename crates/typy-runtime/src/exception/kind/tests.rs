//! Tests of the exception classes the runtime raises itself.

use super::ExceptionKind;

#[test]
fn kinds_carry_their_python_names() {
    assert_eq!(ExceptionKind::ZeroDivisionError.name(), "ZeroDivisionError");
    assert_eq!(ExceptionKind::RecursionError.name(), "RecursionError");
    assert_eq!(ExceptionKind::StopIteration.name(), "StopIteration");
}

#[test]
fn each_kind_has_its_own_class() {
    for (index, kind) in ExceptionKind::ALL.iter().enumerate() {
        for other in &ExceptionKind::ALL[index + 1..] {
            assert!(!core::ptr::eq(kind.class(), other.class()), "{kind:?}");
        }
    }
}
