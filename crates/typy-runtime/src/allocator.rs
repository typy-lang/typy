//! Allocates the runtime's objects; running out of memory ends the program (detailed design section 14.2).

use crate::libc;

/// Returns `size` bytes, or ends the program with `MemoryError` when they are not available.
pub(crate) fn allocate(size: usize) -> *mut u8 {
    let pointer = libc::allocate(size.max(1));
    if pointer.is_null() {
        out_of_memory();
    }
    pointer
}

/// Resizes an allocation, or ends the program with `MemoryError`.
pub(crate) fn reallocate(pointer: *mut u8, size: usize) -> *mut u8 {
    let resized = libc::reallocate(pointer, size.max(1));
    if resized.is_null() {
        out_of_memory();
    }
    resized
}

/// Frees an allocation.
pub(crate) fn free(pointer: *mut u8) {
    libc::release(pointer);
}

/// Returns `count * size` bytes for an array, or ends the program when the size overflows.
pub(crate) fn array_size(count: usize, size: usize, header: usize) -> usize {
    count
        .checked_mul(size)
        .and_then(|bytes| bytes.checked_add(header))
        .unwrap_or_else(|| out_of_memory())
}

fn out_of_memory() -> ! {
    crate::io::flush_stdout();
    libc::abort_with(b"MemoryError\n")
}

#[cfg(test)]
mod tests {
    use super::{allocate, array_size, free, reallocate};

    #[test]
    fn memory_can_be_allocated_resized_and_freed() {
        let pointer = allocate(16);
        let pointer = reallocate(pointer, 64);
        free(pointer);
        assert_eq!(array_size(3, 8, 24), 48);
    }
}
