//! `list`: a growable array whose items are stored in eight bytes each, by their kind (DAT-07,
//! MEM-02, NFR-13, ADR-0009).

mod item;
mod operations;
#[cfg(test)]
mod tests;

pub(crate) use item::ItemKind;
pub(crate) use operations::{typy_rt_list_equal, typy_rt_list_repr};

use crate::allocator::{allocate, array_size, free, reallocate};
use crate::header::{LIST_TYPE_ID, ObjectHeader};

/// A list. Its items live in a buffer of their own, so the list keeps its address as it grows.
#[repr(C)]
pub struct ListObject {
    /// The object header.
    pub header: ObjectHeader,
    /// The number of items.
    pub length: usize,
    /// The number of items the buffer holds.
    pub capacity: usize,
    /// How the items are stored, as an `ItemKind`.
    pub kind: u32,
    /// Unused; keeps `items` aligned.
    pub padding: u32,
    /// The items, eight bytes each.
    pub items: *mut u64,
}

impl ListObject {
    /// Returns the kind of the list's items.
    pub(crate) fn item_kind(&self) -> ItemKind {
        ItemKind::from_code(self.kind)
    }
}

/// An empty list that is never freed: the first value the compiler gives a list variable, so
/// every path out of a function can release it. Definite assignment guarantees no program reads
/// it, so it is never changed.
#[repr(transparent)]
pub struct Placeholder(ListObject);

// SAFETY: the placeholder is immortal and never written, so sharing it cannot race.
unsafe impl Sync for Placeholder {}

/// The placeholder list generated code refers to.
#[unsafe(no_mangle)]
#[allow(
    non_upper_case_globals,
    reason = "the runtime's symbols are `typy_rt_` and a snake-case name"
)]
pub static typy_rt_placeholder_list: Placeholder = Placeholder(ListObject {
    header: ObjectHeader {
        reference_count: crate::header::IMMORTAL,
        type_id: LIST_TYPE_ID,
    },
    length: 0,
    capacity: 0,
    kind: ItemKind::Int as u32,
    padding: 0,
    items: core::ptr::null_mut(),
});

/// Returns a new empty list of `kind` with room for `capacity` items.
#[allow(
    clippy::cast_ptr_alignment,
    reason = "malloc returns memory aligned for every type"
)]
pub(crate) fn new_list(kind: ItemKind, capacity: usize) -> *mut ListObject {
    let object = allocate(core::mem::size_of::<ListObject>()).cast::<ListObject>();
    let items = if capacity == 0 {
        core::ptr::null_mut()
    } else {
        allocate(array_size(capacity, 8, 0)).cast::<u64>()
    };
    // SAFETY: `object` points to enough writable bytes for a `ListObject`.
    unsafe {
        object.write(ListObject {
            header: ObjectHeader::new(LIST_TYPE_ID),
            length: 0,
            capacity,
            kind: kind as u32,
            padding: 0,
            items,
        });
    }
    object
}

/// Returns the capacity to grow to when a list of `length` items needs one more. Small lists
/// grow by an eighth and large ones by a thirty-second, so a list built one `append` at a time
/// wastes at most about three percent of its memory once it is large (NFR-13).
pub(crate) const fn grown_capacity(length: usize) -> usize {
    let extra = if length < 65_536 {
        length >> 3
    } else {
        length >> 5
    };
    length + extra + 4
}

/// Makes room for one more item.
///
/// # Safety
///
/// `list` points to a live list.
pub(crate) unsafe fn reserve_one(list: *mut ListObject) {
    // SAFETY: the caller passes a live list, whose buffer holds `capacity` items.
    unsafe {
        if (*list).length < (*list).capacity {
            return;
        }
        let capacity = grown_capacity((*list).length);
        let size = array_size(capacity, 8, 0);
        let items = if (*list).items.is_null() {
            allocate(size)
        } else {
            reallocate((*list).items.cast::<u8>(), size)
        };
        #[allow(
            clippy::cast_ptr_alignment,
            reason = "malloc returns memory aligned for every type"
        )]
        let items = items.cast::<u64>();
        (*list).items = items;
        (*list).capacity = capacity;
    }
}

/// Frees a list whose last reference is gone, after releasing each item.
///
/// # Safety
///
/// `list` points to a live list that nothing else refers to.
pub(crate) unsafe fn destroy(list: *mut ListObject) {
    // SAFETY: the list is live, and its first `length` items are initialised.
    unsafe {
        let kind = (*list).item_kind();
        for index in 0..(*list).length {
            kind.release(*(*list).items.add(index));
        }
        if !(*list).items.is_null() {
            free((*list).items.cast::<u8>());
        }
    }
    free(list.cast::<u8>());
}
