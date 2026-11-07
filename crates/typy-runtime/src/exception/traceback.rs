//! The frames an exception has passed through, innermost first.

use crate::allocator::{free, reallocate};

/// One frame: the function name, the file name and the line.
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub(crate) struct TracebackEntry {
    pub(crate) function: *const u8,
    pub(crate) file: *const u8,
    pub(crate) line: u32,
}

/// A growable list of entries in memory from `malloc`.
#[repr(C)]
#[derive(Debug)]
pub(crate) struct Traceback {
    entries: *mut TracebackEntry,
    length: u32,
    capacity: u32,
}

impl Traceback {
    pub(crate) const fn new() -> Self {
        Self {
            entries: core::ptr::null_mut(),
            length: 0,
            capacity: 0,
        }
    }

    pub(crate) fn len(&self) -> usize {
        self.length as usize
    }

    pub(crate) fn push(&mut self, function: *const u8, file: *const u8, line: u32) {
        if self.length == self.capacity {
            let capacity = (self.capacity * 2).max(4);
            let size = crate::allocator::array_size(
                capacity as usize,
                core::mem::size_of::<TracebackEntry>(),
                0,
            );
            self.entries = reallocate(self.entries.cast(), size).cast();
            self.capacity = capacity;
        }
        // SAFETY: the list has room for one more entry.
        unsafe {
            self.entries.add(self.len()).write(TracebackEntry {
                function,
                file,
                line,
            });
        };
        self.length += 1;
    }

    /// Returns the entry at `index`, innermost first.
    pub(crate) fn get(&self, index: usize) -> TracebackEntry {
        // SAFETY: callers keep `index` below the length.
        unsafe { *self.entries.add(index) }
    }

    pub(crate) fn release(&mut self) {
        if !self.entries.is_null() {
            free(self.entries.cast());
        }
        *self = Self::new();
    }
}
