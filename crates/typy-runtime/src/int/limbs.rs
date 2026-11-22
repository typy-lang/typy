//! A growable array of 64-bit limbs, held in place up to 1024 bits and in memory from `malloc`
//! past that, since the runtime has no `alloc` crate (ADR-0103).

use core::mem::MaybeUninit;

use crate::allocator::{allocate, array_size, free, reallocate};

/// The limbs an array holds in place: 1024 bits, the largest size class (ADR-0103).
pub(crate) const INLINE_LIMBS: usize = 16;

/// The magnitude of an integer, least significant limb first.
pub(crate) struct Limbs {
    /// The limbs while they fit, which is while `heap` is null; only the first `length` are
    /// written, so a new array costs no clearing.
    inline: [MaybeUninit<u64>; INLINE_LIMBS],
    /// The limbs once they outgrow `inline`, in memory from `malloc`.
    heap: *mut u64,
    length: usize,
    capacity: usize,
}

impl Limbs {
    /// Returns an empty array, which is the magnitude zero.
    pub(crate) const fn new() -> Self {
        Self {
            inline: [const { MaybeUninit::uninit() }; INLINE_LIMBS],
            heap: core::ptr::null_mut(),
            length: 0,
            capacity: INLINE_LIMBS,
        }
    }

    /// Returns an array holding `values`, without leading zero limbs.
    pub(crate) fn from_slice(values: &[u64]) -> Self {
        let mut limbs = Self::new();
        limbs.reserve(values.len());
        // SAFETY: the array has room for every value, and the two do not overlap.
        unsafe {
            core::ptr::copy_nonoverlapping(values.as_ptr(), limbs.data_mut(), values.len());
        }
        limbs.length = values.len();
        limbs.trim();
        limbs
    }

    /// Returns an array of `length` zero limbs.
    pub(crate) fn zeros(length: usize) -> Self {
        let mut limbs = Self::new();
        limbs.reserve(length);
        // SAFETY: the array has room for `length` limbs.
        unsafe { core::ptr::write_bytes(limbs.data_mut(), 0, length) };
        limbs.length = length;
        limbs
    }

    /// Returns where the limbs start.
    fn data(&self) -> *const u64 {
        if self.heap.is_null() {
            self.inline.as_ptr().cast()
        } else {
            self.heap
        }
    }

    /// Returns where the limbs start, for writing.
    fn data_mut(&mut self) -> *mut u64 {
        if self.heap.is_null() {
            self.inline.as_mut_ptr().cast()
        } else {
            self.heap
        }
    }

    fn reserve(&mut self, wanted: usize) {
        if wanted <= self.capacity {
            return;
        }
        let capacity = wanted.max(self.capacity * 2);
        let size = array_size(capacity, core::mem::size_of::<u64>(), 0);
        #[allow(
            clippy::cast_ptr_alignment,
            reason = "malloc returns memory aligned for every type"
        )]
        if self.heap.is_null() {
            let heap = allocate(size).cast::<u64>();
            // SAFETY: the new memory has room for the limbs held in place, the first `length`
            // of which are written.
            unsafe {
                core::ptr::copy_nonoverlapping(self.inline.as_ptr().cast(), heap, self.length);
            }
            self.heap = heap;
        } else {
            self.heap = reallocate(self.heap.cast(), size).cast::<u64>();
        }
        self.capacity = capacity;
    }

    /// Appends a limb.
    #[inline]
    pub(crate) fn push(&mut self, value: u64) {
        if self.length == self.capacity {
            self.reserve(self.length + 1);
        }
        // SAFETY: the array has room for one more limb.
        unsafe { self.data_mut().add(self.length).write(value) };
        self.length += 1;
    }

    /// Removes leading zero limbs, so zero is the empty array.
    pub(crate) fn trim(&mut self) {
        while self.length > 0 && self.as_slice()[self.length - 1] == 0 {
            self.length -= 1;
        }
    }

    /// Returns the limbs.
    pub(crate) fn as_slice(&self) -> &[u64] {
        // SAFETY: the array holds `length` initialised limbs where `data` points.
        unsafe { core::slice::from_raw_parts(self.data(), self.length) }
    }

    /// Returns the limbs for writing.
    pub(crate) fn as_mut_slice(&mut self) -> &mut [u64] {
        let length = self.length;
        // SAFETY: the array holds `length` initialised limbs, borrowed mutably once.
        unsafe { core::slice::from_raw_parts_mut(self.data_mut(), length) }
    }

    /// Returns true for the magnitude zero.
    pub(crate) fn is_zero(&self) -> bool {
        self.as_slice().iter().all(|limb| *limb == 0)
    }
}

impl Drop for Limbs {
    fn drop(&mut self) {
        if !self.heap.is_null() {
            free(self.heap.cast());
        }
    }
}

#[cfg(test)]
mod tests;
