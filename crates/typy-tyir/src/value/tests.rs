//! Tests of `value`.

use super::{ValueInfo, ValueRepresentation};
use crate::types::TyirType;

#[test]
fn objects_are_heap_values_and_the_rest_are_inline() {
    assert_eq!(
        ValueInfo::of(TyirType::STR).representation,
        ValueRepresentation::Heap
    );
    for ty in [
        TyirType::Int,
        TyirType::Float,
        TyirType::Bool,
        TyirType::Unit,
    ] {
        assert_eq!(
            ValueInfo::of(ty).representation,
            ValueRepresentation::Inline
        );
    }
}
