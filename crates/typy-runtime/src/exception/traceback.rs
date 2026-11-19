//! The frames an exception has passed through, innermost first, and the variables of those
//! frames it keeps alive, as CPython's traceback keeps its frames (FUN-11, ADR-0070).

use crate::allocator::{free, reallocate};
use crate::header::ObjectHeader;

/// One frame: the function name, the file name and the line.
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub(crate) struct TracebackEntry {
    pub(crate) function: *const u8,
    pub(crate) file: *const u8,
    pub(crate) line: u32,
}

/// A growable list of entries in memory from `malloc`, and a growable list of the objects it
/// keeps, each with the reference the frame that held it gave up.
#[repr(C)]
#[derive(Debug)]
pub(crate) struct Traceback {
    entries: *mut TracebackEntry,
    length: u32,
    capacity: u32,
    kept: *mut *mut ObjectHeader,
    kept_length: u32,
    kept_capacity: u32,
}

impl Traceback {
    pub(crate) const fn new() -> Self {
        Self {
            entries: core::ptr::null_mut(),
            length: 0,
            capacity: 0,
            kept: core::ptr::null_mut(),
            kept_length: 0,
            kept_capacity: 0,
        }
    }

    /// Keeps `object`, taking over a reference to it, until the traceback is released.
    pub(crate) fn keep(&mut self, object: *mut ObjectHeader) {
        if self.kept_length == self.kept_capacity {
            let capacity = (self.kept_capacity * 2).max(4);
            let size = crate::allocator::array_size(
                capacity as usize,
                core::mem::size_of::<*mut ObjectHeader>(),
                0,
            );
            self.kept = reallocate(self.kept.cast(), size).cast();
            self.kept_capacity = capacity;
        }
        // SAFETY: the list has room for one more object.
        unsafe { self.kept.add(self.kept_length as usize).write(object) };
        self.kept_length += 1;
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

    /// Frees the entries, and releases the objects it keeps in the order they came, the
    /// innermost frame's first, as CPython frees a traceback's frames.
    pub(crate) fn release(&mut self) {
        if !self.entries.is_null() {
            free(self.entries.cast());
        }
        let (kept, length) = (self.kept, self.kept_length as usize);
        *self = Self::new();
        if kept.is_null() {
            return;
        }
        for index in 0..length {
            // SAFETY: each kept object holds the reference the traceback took over.
            unsafe { crate::refcount::typy_rt_decrement(kept.add(index).read()) };
        }
        free(kept.cast());
    }
}
