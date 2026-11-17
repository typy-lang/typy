//! The objects a `ctypes` object keeps alive for the memory it owns or points into, each
//! under the address it was written at, as CPython's `_objects` does: a pointer's target,
//! and the `bytes` whose buffer a `c_char_p` holds, written through the object itself or
//! through an item of it.

use crate::header::ObjectHeader;

/// One object kept, with the address of the C value that refers to it.
#[derive(Clone, Copy)]
struct Entry {
    address: usize,
    object: *mut ObjectHeader,
}

/// A growing table of entries, one for each address written; made, with room for a few
/// entries, the first time an object keeps something.
pub(crate) struct Kept {
    length: usize,
    capacity: usize,
    entries: *mut Entry,
}

/// The entries a new table has room for.
const FIRST_CAPACITY: usize = 4;

impl Kept {
    /// Returns the table's entries.
    fn entries(&mut self) -> &mut [Entry] {
        // SAFETY: `entries` holds `length` initialised entries.
        unsafe { core::slice::from_raw_parts_mut(self.entries, self.length) }
    }
}

fn entries_size(capacity: usize) -> usize {
    crate::allocator::array_size(capacity, core::mem::size_of::<Entry>(), 0)
}

/// Keeps `object` alive in `*table` for the C value at `address`, releasing the object kept
/// there before; a null `object` keeps nothing there.
///
/// # Safety
///
/// `*table` is null or a table this module made, and `object` null or a live object.
pub(crate) unsafe fn keep(table: &mut *mut Kept, address: *mut u8, object: *mut ObjectHeader) {
    if !object.is_null() {
        // SAFETY: the caller's guarantee.
        unsafe { crate::refcount::typy_rt_increment(object) };
    }
    if table.is_null() {
        if object.is_null() {
            return;
        }
        let made = allocated::<Kept>(crate::allocator::allocate(core::mem::size_of::<Kept>()));
        let entries = allocated::<Entry>(crate::allocator::allocate(entries_size(FIRST_CAPACITY)));
        // SAFETY: the allocation holds a table, aligned for it.
        unsafe {
            made.write(Kept {
                length: 0,
                capacity: FIRST_CAPACITY,
                entries,
            });
        }
        *table = made;
    }
    // SAFETY: the table is live and only this object reaches it.
    let kept = unsafe { &mut **table };
    let address = address as usize;
    if let Some(entry) = kept
        .entries()
        .iter_mut()
        .find(|entry| entry.address == address)
    {
        let before = core::mem::replace(&mut entry.object, object);
        release(before);
        return;
    }
    if object.is_null() {
        return;
    }
    if kept.length == kept.capacity {
        kept.capacity *= 2;
        let size = entries_size(kept.capacity);
        kept.entries = allocated::<Entry>(crate::allocator::reallocate(kept.entries.cast(), size));
    }
    // SAFETY: the table has room for one more entry.
    unsafe {
        kept.entries
            .add(kept.length)
            .write(Entry { address, object });
    }
    kept.length += 1;
}

/// Returns the object `table` keeps for the C value at `address`, or null.
///
/// # Safety
///
/// `table` is null or a table this module made.
pub(crate) unsafe fn kept_at(table: *mut Kept, address: *mut u8) -> *mut ObjectHeader {
    if table.is_null() {
        return core::ptr::null_mut();
    }
    // SAFETY: the caller's guarantee.
    let kept = unsafe { &mut *table };
    kept.entries()
        .iter()
        .find(|entry| entry.address == address as usize)
        .map_or(core::ptr::null_mut(), |entry| entry.object)
}

/// Releases every object a table keeps, and frees it.
///
/// # Safety
///
/// `table` is null or a table this module made, which nothing reads after.
pub(crate) unsafe fn release_all(table: *mut Kept) {
    if table.is_null() {
        return;
    }
    // SAFETY: the caller's guarantee.
    let kept = unsafe { &mut *table };
    for entry in kept.entries() {
        release(entry.object);
    }
    crate::allocator::free(kept.entries.cast());
    crate::allocator::free(table.cast());
}

/// Returns memory from the allocator as a pointer to `T`.
#[allow(
    clippy::cast_ptr_alignment,
    reason = "the allocator returns memory aligned for every type"
)]
const fn allocated<T>(memory: *mut u8) -> *mut T {
    memory.cast::<T>()
}

fn release(object: *mut ObjectHeader) {
    if !object.is_null() {
        // SAFETY: a kept object holds the reference this table took.
        unsafe { crate::refcount::typy_rt_decrement(object) };
    }
}
