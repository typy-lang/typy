//! Bytes in memory of their own that grow as they are added to: a line `input` reads, or the
//! rest of a file `read()` reads.

/// A growable run of bytes; `discard` frees it.
pub(crate) struct ByteBuffer {
    bytes: *mut u8,
    length: usize,
    capacity: usize,
}

impl ByteBuffer {
    /// Returns an empty buffer with room for 64 bytes.
    pub(crate) fn new() -> Self {
        Self {
            bytes: crate::allocator::allocate(64),
            length: 0,
            capacity: 64,
        }
    }

    /// Adds `bytes` at the end.
    pub(crate) fn extend(&mut self, bytes: &[u8]) {
        let needed = self.length + bytes.len();
        if needed > self.capacity {
            self.capacity = needed.next_power_of_two();
            self.bytes = crate::allocator::reallocate(self.bytes, self.capacity);
        }
        // SAFETY: the allocation holds `capacity` bytes, at least `length` more than the
        // bytes copied, and the source is a different allocation.
        unsafe {
            core::ptr::copy_nonoverlapping(
                bytes.as_ptr(),
                self.bytes.add(self.length),
                bytes.len(),
            );
        }
        self.length = needed;
    }

    /// Returns the bytes added so far.
    pub(crate) fn bytes(&self) -> &[u8] {
        // SAFETY: the first `length` bytes are written.
        unsafe { core::slice::from_raw_parts(self.bytes, self.length) }
    }

    /// Frees the buffer.
    pub(crate) fn discard(self) {
        crate::allocator::free(self.bytes);
    }
}
