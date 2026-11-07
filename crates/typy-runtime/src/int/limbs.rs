//! A growable array of 64-bit limbs in memory from `malloc`, since the runtime has no `alloc` crate.

use crate::allocator::{array_size, free, reallocate};

/// The magnitude of an integer, least significant limb first.
pub(crate) struct Limbs {
    pointer: *mut u64,
    length: usize,
    capacity: usize,
}

impl Limbs {
    /// Returns an empty array, which is the magnitude zero.
    pub(crate) const fn new() -> Self {
        Self {
            pointer: core::ptr::null_mut(),
            length: 0,
            capacity: 0,
        }
    }

    /// Returns an array holding `values`, without leading zero limbs.
    pub(crate) fn from_slice(values: &[u64]) -> Self {
        let mut limbs = Self::new();
        for value in values {
            limbs.push(*value);
        }
        limbs.trim();
        limbs
    }

    /// Returns an array of `length` zero limbs.
    pub(crate) fn zeros(length: usize) -> Self {
        let mut limbs = Self::new();
        limbs.reserve(length);
        for _ in 0..length {
            limbs.push(0);
        }
        limbs
    }

    fn reserve(&mut self, wanted: usize) {
        if wanted <= self.capacity {
            return;
        }
        let capacity = wanted.max(self.capacity * 2).max(4);
        let size = array_size(capacity, core::mem::size_of::<u64>(), 0);
        #[allow(
            clippy::cast_ptr_alignment,
            reason = "malloc returns memory aligned for every type"
        )]
        let pointer = reallocate(self.pointer.cast(), size).cast::<u64>();
        self.pointer = pointer;
        self.capacity = capacity;
    }

    /// Appends a limb.
    pub(crate) fn push(&mut self, value: u64) {
        self.reserve(self.length + 1);
        // SAFETY: the array has room for one more limb.
        unsafe { self.pointer.add(self.length).write(value) };
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
        if self.pointer.is_null() {
            return &[];
        }
        // SAFETY: the pointer holds `length` initialised limbs.
        unsafe { core::slice::from_raw_parts(self.pointer, self.length) }
    }

    /// Returns the limbs for writing.
    pub(crate) fn as_mut_slice(&mut self) -> &mut [u64] {
        if self.pointer.is_null() {
            return &mut [];
        }
        // SAFETY: the pointer holds `length` initialised limbs, borrowed mutably once.
        unsafe { core::slice::from_raw_parts_mut(self.pointer, self.length) }
    }

    /// Returns true for the magnitude zero.
    pub(crate) fn is_zero(&self) -> bool {
        self.as_slice().iter().all(|limb| *limb == 0)
    }
}

impl Drop for Limbs {
    fn drop(&mut self) {
        if !self.pointer.is_null() {
            free(self.pointer.cast());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Limbs;

    #[test]
    fn limbs_grow_and_trim_leading_zeros() {
        let mut limbs = Limbs::new();
        assert!(limbs.is_zero());
        for value in 0..10 {
            limbs.push(value);
        }
        limbs.push(0);
        limbs.trim();
        assert_eq!(limbs.as_slice().len(), 10);
        limbs.as_mut_slice()[0] = 7;
        assert_eq!(Limbs::from_slice(&[7, 0, 0]).as_slice(), [7]);
        assert_eq!(Limbs::zeros(3).as_slice(), [0, 0, 0]);
        assert!(Limbs::zeros(2).is_zero());
    }
}
