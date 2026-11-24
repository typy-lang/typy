//! Tests of allocation through the program's allocator.

use super::{allocate, array_size, free, reallocate};

#[test]
fn memory_can_be_allocated_resized_and_freed() {
    let pointer = allocate(16);
    let pointer = reallocate(pointer, 64);
    free(pointer);
    assert_eq!(array_size(3, 8, 24), 48);
}
