//! Allocates the runtime's objects through the program's allocator; running out of memory
//! ends the program (detailed design section 14.2).

use crate::libc;

unsafe extern "C" {
    // The program object defines these hooks; each calls the function its project names in
    // `typy.toml`, or `malloc`, `realloc` or `free` (MEM-08, ADR-0043).
    fn typy_rt_heap_allocate(size: usize) -> *mut u8;
    fn typy_rt_heap_reallocate(pointer: *mut u8, size: usize) -> *mut u8;
    fn typy_rt_heap_free(pointer: *mut u8);
}

/// Returns `size` bytes, or ends the program with `MemoryError` when they are not available.
pub(crate) fn allocate(size: usize) -> *mut u8 {
    // SAFETY: the hook takes any size; the runtime never asks for zero bytes, which an
    // allocator may answer with null.
    let pointer = unsafe { typy_rt_heap_allocate(size.max(1)) };
    if pointer.is_null() {
        out_of_memory();
    }
    pointer
}

/// Resizes an allocation, or ends the program with `MemoryError`.
pub(crate) fn reallocate(pointer: *mut u8, size: usize) -> *mut u8 {
    // SAFETY: `pointer` came from `allocate` or `reallocate`, which callers keep to.
    let resized = unsafe { typy_rt_heap_reallocate(pointer, size.max(1)) };
    if resized.is_null() {
        out_of_memory();
    }
    resized
}

/// Frees an allocation.
pub(crate) fn free(pointer: *mut u8) {
    // SAFETY: callers pass a pointer from `allocate` or `reallocate` that is freed once.
    unsafe { typy_rt_heap_free(pointer) }
}

/// The hooks a program object defines, over the C library, for the unit tests, which link
/// no program object.
#[cfg(test)]
mod hooks {
    use crate::libc;

    #[unsafe(no_mangle)]
    extern "C" fn typy_rt_heap_allocate(size: usize) -> *mut u8 {
        libc::allocate(size)
    }

    #[unsafe(no_mangle)]
    extern "C" fn typy_rt_heap_reallocate(pointer: *mut u8, size: usize) -> *mut u8 {
        libc::reallocate(pointer, size)
    }

    #[unsafe(no_mangle)]
    extern "C" fn typy_rt_heap_free(pointer: *mut u8) {
        libc::release(pointer);
    }
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
