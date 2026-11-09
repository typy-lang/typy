//! `list`: a growable array whose items are stored by their type, eight bytes each for an
//! `int`, a `float`, a `bool`, `None` or an object, and inline for a tuple (DAT-07, MEM-02,
//! NFR-13, ADR-0001, ADR-0009).

mod aggregate;
mod item;
mod methods;
mod operations;
mod order;
#[cfg(test)]
mod tests;

#[cfg(test)]
pub(crate) use aggregate::{typy_rt_list_any, typy_rt_list_extreme, typy_rt_list_sort};
pub(crate) use item::{ItemKind, ItemType};
pub(crate) use methods::typy_rt_list_reverse;
#[cfg(test)]
pub(crate) use methods::{
    typy_rt_list_clear, typy_rt_list_count, typy_rt_list_extend, typy_rt_list_index,
    typy_rt_list_insert, typy_rt_list_pop, typy_rt_list_remove,
};
pub(crate) use operations::join_reprs;
#[cfg(test)]
pub(crate) use operations::typy_rt_list_contains;
pub(crate) use operations::{typy_rt_list_append, typy_rt_list_equal, typy_rt_list_repr};
pub(crate) use order::typy_rt_list_compare;
#[cfg(test)]
pub(crate) use order::{typy_rt_list_concat, typy_rt_list_repeat};

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
    /// The eight-byte words an item takes: 1, or more for a tuple.
    pub words: u32,
    /// The items, `words` words each.
    pub items: *mut u64,
    /// The descriptor of tuple items, or null.
    pub tuple: *const crate::tuple::TupleType,
}

impl ListObject {
    /// Returns whether this is a `tuple[T, ...]`, a list object sealed by `typy_rt_tuple_seal`.
    pub(crate) fn is_tuple(&self) -> bool {
        self.header.type_id == crate::header::TUPLE_TYPE_ID
    }

    /// Returns the type of the list's items.
    pub(crate) fn item_type(&self) -> ItemType {
        ItemType::of(self.kind, self.tuple)
    }

    /// Returns the eight-byte words an item takes.
    pub(crate) fn words(&self) -> usize {
        usize::try_from(self.words).unwrap_or(1)
    }

    /// Returns where item `index` is stored.
    ///
    /// # Safety
    ///
    /// `index` is at most the capacity, and the buffer exists when it is below it.
    pub(crate) unsafe fn slot(&self, index: usize) -> *mut u64 {
        // SAFETY: the caller keeps the index inside the buffer, or just past it.
        unsafe { self.items.add(index * self.words()) }
    }

    /// Returns item `index`: its eight bytes, or the address of a tuple.
    ///
    /// # Safety
    ///
    /// `index` is below the length.
    pub(crate) unsafe fn item(&self, index: usize) -> u64 {
        // SAFETY: the caller keeps the index below the length.
        unsafe { self.item_type().load(self.slot(index)) }
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
    words: 1,
    items: core::ptr::null_mut(),
    tuple: core::ptr::null(),
});

/// Returns a new empty list of items of `item_type` with room for `capacity` items.
#[allow(
    clippy::cast_ptr_alignment,
    reason = "malloc returns memory aligned for every type"
)]
pub(crate) fn new_list(item_type: impl Into<ItemType>, capacity: usize) -> *mut ListObject {
    let item_type = item_type.into();
    let words = item_type.words();
    let object = allocate(core::mem::size_of::<ListObject>()).cast::<ListObject>();
    let items = if capacity == 0 {
        core::ptr::null_mut()
    } else {
        allocate(array_size(capacity, 8 * words, 0)).cast::<u64>()
    };
    // SAFETY: `object` points to enough writable bytes for a `ListObject`.
    unsafe {
        object.write(ListObject {
            header: ObjectHeader::new(LIST_TYPE_ID),
            length: 0,
            capacity,
            kind: item_type.kind() as u32,
            words: u32::try_from(words).unwrap_or(u32::MAX),
            items,
            tuple: item_type.descriptor(),
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
        let size = array_size(capacity, 8 * (*list).words(), 0);
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

/// Makes room for `additional` more items at once.
///
/// # Safety
///
/// `list` points to a live list.
pub(crate) unsafe fn reserve(list: *mut ListObject, additional: usize) {
    // SAFETY: the caller passes a live list.
    unsafe {
        while (*list).capacity < (*list).length + additional {
            let wanted = (*list).length + additional;
            let grown = grown_capacity((*list).capacity).max(wanted);
            let size = array_size(grown, 8 * (*list).words(), 0);
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
            (*list).capacity = grown;
        }
    }
}

/// Appends `item` with a reference of its own, into room the caller has reserved.
///
/// # Safety
///
/// `list` points to a live list with room for one more item, and `item` is a live item of its
/// type that does not lie in the slot it is written to.
pub(crate) unsafe fn push_retained(list: *mut ListObject, item: u64) {
    // SAFETY: the caller's guarantee.
    unsafe {
        let item_type = (*list).item_type();
        item_type.retain(item);
        item_type.store((*list).slot((*list).length), item);
        (*list).length += 1;
    }
}

/// Removes item `position` by rotating it to the slot just past the new end, where it stays
/// until the list changes again, and returns it there, with the references it holds.
///
/// # Safety
///
/// `list` points to a live list, and `position` is below its length.
pub(crate) unsafe fn take_out(list: *mut ListObject, position: usize) -> u64 {
    // SAFETY: the caller keeps the position inside the list, so the rotated words exist.
    unsafe {
        let words = (*list).words();
        let length = (*list).length;
        core::slice::from_raw_parts_mut((*list).slot(position), (length - position) * words)
            .rotate_left(words);
        (*list).length = length - 1;
        (*list).item(length - 1)
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
        let item_type = (*list).item_type();
        for index in 0..(*list).length {
            item_type.release((*list).item(index));
        }
        if !(*list).items.is_null() {
            free((*list).items.cast::<u8>());
        }
    }
    free(list.cast::<u8>());
}

/// Makes `list` a `tuple[T, ...]`: it keeps its items and is never changed again (DAT-14).
///
/// # Safety
///
/// `list` points to a live list that nothing else refers to yet.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn typy_rt_tuple_seal(list: *mut ListObject) {
    // SAFETY: the caller passes a live list.
    unsafe { (*list).header.type_id = crate::header::TUPLE_TYPE_ID };
}

/// Gives `result`, a new list made from `source`, the type of `source`, so a slice, a
/// concatenation or a repetition of a tuple is a tuple.
///
/// # Safety
///
/// Both point to live lists.
pub(crate) unsafe fn same_type(
    result: *mut ListObject,
    source: *const ListObject,
) -> *mut ListObject {
    // SAFETY: the caller passes live lists.
    unsafe { (*result).header.type_id = (*source).header.type_id };
    result
}
