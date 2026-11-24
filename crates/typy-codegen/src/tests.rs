//! Tests of the crate root of `typy-codegen`.

use super::llvm_version;

#[test]
fn the_linked_llvm_is_major_version_23() {
    assert_eq!(llvm_version().0, 23);
}
