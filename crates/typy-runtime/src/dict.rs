//! `dict`: CPython's compact dict (DAT-09, ADR-0010). Entries are kept in insertion order in an
//! array of their own, and a table of indices finds them by hash. A removed entry leaves a hole
//! until the dict is resized, which compacts the entries, so the positions a loop reads are
//! CPython's. Keys and values are stored by their types, as a list stores items (ADR-0009).

mod operations;
#[cfg(test)]
mod tests;
mod text;
pub(crate) mod view;

pub(crate) use text::typy_rt_dict_view_text;
#[cfg(test)]
pub(crate) use view::typy_rt_dict_view;
pub(crate) use view::{ViewObject, destroy as destroy_view, length as view_length};

pub(crate) use operations::{
    typy_rt_dict_copy, typy_rt_dict_equal, typy_rt_dict_key, typy_rt_dict_next, typy_rt_dict_pop,
    typy_rt_dict_value,
};
#[cfg(test)]
pub(crate) use operations::{typy_rt_dict_new, typy_rt_dict_set};
pub(crate) use text::typy_rt_dict_repr;

use crate::allocator::{allocate, array_size, free};
use crate::header::{DICT_TYPE_ID, ObjectHeader};
use crate::list::ItemType;
use crate::tuple::TupleType;

/// The fewest index slots of a table, CPython's `PyDict_MINSIZE`, as a power of two.
const MINIMUM_LOG2: u32 = 3;
/// How far the probe's perturbation shifts each jump, CPython's `PERTURB_SHIFT`.
const PERTURB_SHIFT: u32 = 5;
/// An index slot that has never held an entry.
const EMPTY: i64 = -1;
/// An index slot whose entry was removed.
const DUMMY: i64 = -2;

/// A dict. Its entries and indices live in buffers of their own, so the dict keeps its
/// address as it grows.
#[repr(C)]
pub struct DictObject {
    /// The object header.
    pub header: ObjectHeader,
    /// The number of entries that hold an item, CPython's `ma_used`.
    pub used: usize,
    /// The entries written, holes included, CPython's `dk_nentries`.
    pub filled: usize,
    /// The entries that can still be written before the table is resized, `dk_usable`.
    pub usable: usize,
    /// The number of index slots as a power of two, or 0 before the first entry.
    pub log2: u32,
    /// How keys are stored, as an `ItemKind`.
    pub key_kind: u32,
    /// How values are stored, as an `ItemKind`.
    pub value_kind: u32,
    /// The eight-byte words of a key.
    pub key_words: u32,
    /// The descriptor of tuple keys, or null.
    pub key_tuple: *const TupleType,
    /// The descriptor of tuple values, or null.
    pub value_tuple: *const TupleType,
    /// The index slots: an entry's position, `EMPTY` or `DUMMY`.
    pub indices: *mut i64,
    /// The entries, each a hash word, the key's words and the value's words.
    pub entries: *mut u64,
    /// Whether each entry holds an item.
    pub live: *mut u8,
}

/// Returns the entries a table of `2**log2` index slots holds, CPython's `USABLE_FRACTION`.
const fn usable_entries(log2: u32) -> usize {
    ((1_usize << log2) << 1) / 3
}

/// Returns the smallest table, at least of eight slots, of at least `minimum` slots, CPython's
/// `calculate_log2_keysize`.
pub(crate) fn log2_size(minimum: usize) -> u32 {
    let minimum = minimum.max(1 << MINIMUM_LOG2);
    (minimum - 1).bit_width()
}

/// Returns the table that holds `count` entries without resizing, CPython's
/// `estimate_log2_keysize`.
pub(crate) fn log2_for(count: usize) -> u32 {
    log2_size((count * 3).div_ceil(2))
}

impl DictObject {
    /// Returns the type of the keys.
    pub(crate) fn key_type(&self) -> ItemType {
        ItemType::of(self.key_kind, self.key_tuple)
    }

    /// Returns the type of the values.
    pub(crate) fn value_type(&self) -> ItemType {
        ItemType::of(self.value_kind, self.value_tuple)
    }

    /// Returns the words an entry takes: the hash, the key and the value.
    fn stride(&self) -> usize {
        1 + usize::try_from(self.key_words).unwrap_or(1) + self.value_type().words()
    }

    /// Returns where entry `index` begins: its hash word.
    ///
    /// # Safety
    ///
    /// `index` is below the entries' capacity.
    unsafe fn entry(&self, index: usize) -> *mut u64 {
        // SAFETY: the caller keeps the index inside the entries.
        unsafe { self.entries.add(index * self.stride()) }
    }

    /// Returns whether entry `index` holds an item.
    ///
    /// # Safety
    ///
    /// `index` is below `filled`.
    pub(crate) unsafe fn is_live(&self, index: usize) -> bool {
        // SAFETY: the caller's guarantee.
        unsafe { *self.live.add(index) != 0 }
    }

    /// Returns the hash of entry `index`.
    ///
    /// # Safety
    ///
    /// `index` is below `filled`.
    pub(crate) unsafe fn hash(&self, index: usize) -> i64 {
        // SAFETY: the caller's guarantee.
        unsafe { (*self.entry(index)).cast_signed() }
    }

    /// Returns the key of entry `index`: its eight bytes, or the address of a tuple.
    ///
    /// # Safety
    ///
    /// `index` is below `filled`, and the entry holds an item or was just taken out.
    pub(crate) unsafe fn key(&self, index: usize) -> u64 {
        // SAFETY: the caller's guarantee; the key follows the hash word.
        unsafe { self.key_type().load(self.entry(index).add(1)) }
    }

    /// Returns where the value of entry `index` is stored.
    ///
    /// # Safety
    ///
    /// `index` is below the entries' capacity.
    unsafe fn value_slot(&self, index: usize) -> *mut u64 {
        let offset = 1 + usize::try_from(self.key_words).unwrap_or(1);
        // SAFETY: the caller's guarantee; the value follows the key.
        unsafe { self.entry(index).add(offset) }
    }

    /// Returns the value of entry `index`, as `key` returns the key.
    ///
    /// # Safety
    ///
    /// As for `key`.
    pub(crate) unsafe fn value(&self, index: usize) -> u64 {
        // SAFETY: the caller's guarantee.
        unsafe { self.value_type().load(self.value_slot(index)) }
    }

    /// Returns the positions of the entries that hold an item, in insertion order.
    pub(crate) fn live_entries(&self) -> impl Iterator<Item = usize> + '_ {
        // SAFETY: every position below `filled` is inside the entries.
        (0..self.filled).filter(move |index| unsafe { self.is_live(*index) })
    }

    /// Returns the index slot of the entry that holds a key equal to `key`, or of the first
    /// slot the probe finds empty, and the entry's position when there is one.
    ///
    /// # Safety
    ///
    /// The dict has a table, and `key` is a live key of its type whose hash is `hash`.
    unsafe fn probe(&self, key: u64, hash: i64) -> (usize, Option<usize>) {
        let mask = (1_usize << self.log2) - 1;
        #[allow(
            clippy::cast_possible_truncation,
            reason = "CPython takes the hash as a `size_t`, which is 64 bits on every target TyPy supports"
        )]
        let mut perturb = hash.cast_unsigned() as usize;
        let mut slot = perturb & mask;
        let key_type = self.key_type();
        // SAFETY: the caller's guarantee; every slot is masked into the table.
        unsafe {
            loop {
                let index = *self.indices.add(slot);
                if index == EMPTY {
                    return (slot, None);
                }
                if let Ok(position) = usize::try_from(index)
                    && self.hash(position) == hash
                    && key_type.equal(self.key(position), key)
                {
                    return (slot, Some(position));
                }
                perturb >>= PERTURB_SHIFT;
                slot = slot.wrapping_mul(5).wrapping_add(perturb).wrapping_add(1) & mask;
            }
        }
    }

    /// Returns the index slot that holds entry `position`, found by its hash and its position
    /// rather than by comparing keys, as CPython's `lookdict_index` does.
    ///
    /// # Safety
    ///
    /// `position` holds an item.
    unsafe fn slot_of(&self, position: usize) -> usize {
        let mask = (1_usize << self.log2) - 1;
        let wanted = i64::try_from(position).unwrap_or(-1);
        // SAFETY: the caller's guarantee; every slot is masked into the table.
        unsafe {
            #[allow(
                clippy::cast_possible_truncation,
                reason = "CPython takes the hash as a `size_t`, which is 64 bits on every target TyPy supports"
            )]
            let mut perturb = self.hash(position).cast_unsigned() as usize;
            let mut slot = perturb & mask;
            while *self.indices.add(slot) != wanted {
                perturb >>= PERTURB_SHIFT;
                slot = slot.wrapping_mul(5).wrapping_add(perturb).wrapping_add(1) & mask;
            }
            slot
        }
    }

    /// Returns the first index slot for `hash` that holds no entry, empty or dummy, CPython's
    /// `find_empty_slot`.
    ///
    /// # Safety
    ///
    /// The dict has a table with such a slot.
    unsafe fn free_slot(&self, hash: i64) -> usize {
        let mask = (1_usize << self.log2) - 1;
        #[allow(
            clippy::cast_possible_truncation,
            reason = "CPython takes the hash as a `size_t`, which is 64 bits on every target TyPy supports"
        )]
        let mut perturb = hash.cast_unsigned() as usize;
        let mut slot = perturb & mask;
        // SAFETY: the caller's guarantee; every slot is masked into the table.
        unsafe {
            while *self.indices.add(slot) >= 0 {
                perturb >>= PERTURB_SHIFT;
                slot = slot.wrapping_mul(5).wrapping_add(perturb).wrapping_add(1) & mask;
            }
        }
        slot
    }
}

/// Returns a new empty dict of keys of `key_type` and values of `value_type`, with a table
/// for `capacity` entries when it is not 0, as CPython presizes a display.
pub(crate) fn new_dict(
    key_type: ItemType,
    value_type: ItemType,
    capacity: usize,
) -> *mut DictObject {
    #[allow(
        clippy::cast_ptr_alignment,
        reason = "malloc returns memory aligned for every type"
    )]
    let dict = allocate(core::mem::size_of::<DictObject>()).cast::<DictObject>();
    // SAFETY: `dict` points to enough writable bytes for a `DictObject`.
    unsafe {
        dict.write(DictObject {
            header: ObjectHeader::new(DICT_TYPE_ID),
            used: 0,
            filled: 0,
            usable: 0,
            log2: 0,
            key_kind: key_type.kind() as u32,
            value_kind: value_type.kind() as u32,
            key_words: u32::try_from(key_type.words()).unwrap_or(1),
            key_tuple: key_type.descriptor(),
            value_tuple: value_type.descriptor(),
            indices: core::ptr::null_mut(),
            entries: core::ptr::null_mut(),
            live: core::ptr::null_mut(),
        });
        if capacity > 0 {
            resize(&mut *dict, log2_for(capacity));
        }
    }
    dict
}

/// Moves the entries that hold items, in order, to a new table of `2**log2` index slots, which
/// drops the holes, CPython's `dictresize`.
///
/// # Safety
///
/// `dict` is a live dict whose items fit the new table.
#[allow(
    clippy::cast_ptr_alignment,
    reason = "malloc returns memory aligned for every type"
)]
pub(crate) unsafe fn resize(dict: &mut DictObject, log2: u32) {
    let size = 1_usize << log2;
    let capacity = usable_entries(log2);
    let stride = dict.stride();
    let indices = allocate(array_size(size, 8, 0)).cast::<i64>();
    let entries = allocate(array_size(capacity, 8 * stride, 0)).cast::<u64>();
    let live = allocate(capacity);
    // SAFETY: the new buffers have room for every item; each old live entry moves once.
    unsafe {
        for slot in 0..size {
            *indices.add(slot) = EMPTY;
        }
        live.write_bytes(0, capacity);
        let mut written = 0;
        for index in 0..dict.filled {
            if !dict.is_live(index) {
                continue;
            }
            core::ptr::copy_nonoverlapping(
                dict.entry(index),
                entries.add(written * stride),
                stride,
            );
            *live.add(written) = 1;
            written += 1;
        }
        free(dict.indices.cast());
        free(dict.entries.cast());
        free(dict.live);
        dict.indices = indices;
        dict.entries = entries;
        dict.live = live;
        dict.log2 = log2;
        dict.filled = written;
        dict.usable = capacity - written;
        for index in 0..written {
            let slot = dict.free_slot(dict.hash(index));
            *dict.indices.add(slot) = i64::try_from(index).unwrap_or(0);
        }
    }
}

/// Returns the position of the entry whose key equals `key`, or `None`.
///
/// # Safety
///
/// `dict` points to a live dict, and `key` is a live key of its type whose hash is `hash`.
pub(crate) unsafe fn find(dict: *const DictObject, key: u64, hash: i64) -> Option<usize> {
    // SAFETY: the caller's guarantee.
    unsafe {
        if (*dict).log2 == 0 {
            return None;
        }
        (*dict).probe(key, hash).1
    }
}

/// Sets the value of `key`, whose hash is `hash`, to `value`, each with a reference of its own.
/// An existing entry keeps its key and its position and drops its old value; a new key is
/// added at the end, after the table grows to `used * 3` slots when no entry is usable, as
/// CPython's `insertdict` does.
///
/// # Safety
///
/// `dict` points to a live dict, and `key` and `value` are live items of its types.
pub(crate) unsafe fn insert(dict: *mut DictObject, key: u64, hash: i64, value: u64) {
    // SAFETY: the caller's guarantee; the table has a usable entry when one is written.
    unsafe {
        let dict = &mut *dict;
        let value_type = dict.value_type();
        if let Some(position) = find(dict, key, hash) {
            value_type.retain(value);
            let slot = dict.value_slot(position);
            let old = value_type.load(slot);
            // A tuple value is read from its slot, so it is released before it is overwritten.
            value_type.release(old);
            value_type.store(slot, value);
            return;
        }
        if dict.usable == 0 {
            resize(dict, log2_size(dict.used * 3));
        }
        let key_type = dict.key_type();
        key_type.retain(key);
        value_type.retain(value);
        let position = dict.filled;
        let entry = dict.entry(position);
        *entry = hash.cast_unsigned();
        key_type.store(entry.add(1), key);
        value_type.store(dict.value_slot(position), value);
        *dict.live.add(position) = 1;
        let slot = dict.free_slot(hash);
        *dict.indices.add(slot) = i64::try_from(position).unwrap_or(0);
        dict.filled += 1;
        dict.used += 1;
        dict.usable -= 1;
    }
}

/// Takes out the entry at `position`, whose index slot is `slot`; its key and value, with their
/// references, pass to the caller and stay readable until the dict next changes.
///
/// # Safety
///
/// `position` holds an item and `slot` is its index slot.
unsafe fn take_out(dict: &mut DictObject, slot: usize, position: usize) {
    // SAFETY: the caller's guarantee.
    unsafe {
        *dict.indices.add(slot) = DUMMY;
        *dict.live.add(position) = 0;
        dict.used -= 1;
    }
}

/// Takes out the entry whose key equals `key` and returns its position, or `None`; the key
/// and the value pass to the caller as `take_out` says.
///
/// # Safety
///
/// `dict` points to a live dict, and `key` is a live key of its type whose hash is `hash`.
pub(crate) unsafe fn remove(dict: *mut DictObject, key: u64, hash: i64) -> Option<usize> {
    // SAFETY: the caller's guarantee.
    unsafe {
        let dict = &mut *dict;
        if dict.log2 == 0 {
            return None;
        }
        let (slot, position) = dict.probe(key, hash);
        let position = position?;
        take_out(dict, slot, position);
        Some(position)
    }
}

/// Takes out the last entry that holds an item and returns its position, or `None` for an
/// empty dict; the entries after it are dropped, as CPython's `popitem` does.
///
/// # Safety
///
/// `dict` points to a live dict.
pub(crate) unsafe fn take_last(dict: *mut DictObject) -> Option<usize> {
    // SAFETY: the caller's guarantee; the last live entry has an index slot.
    unsafe {
        let dict = &mut *dict;
        if dict.used == 0 {
            return None;
        }
        let position = (0..dict.filled).rev().find(|index| dict.is_live(*index))?;
        let slot = dict.slot_of(position);
        take_out(dict, slot, position);
        dict.filled = position;
        Some(position)
    }
}

/// Releases every key and value and frees the table.
///
/// # Safety
///
/// `dict` points to a live dict; its table is not used again until a new one replaces it.
unsafe fn release_table(dict: &DictObject) {
    let (key_type, value_type) = (dict.key_type(), dict.value_type());
    // SAFETY: each live entry holds references the dict owns.
    unsafe {
        for index in dict.live_entries() {
            key_type.release(dict.key(index));
            value_type.release(dict.value(index));
        }
    }
    free(dict.indices.cast());
    free(dict.entries.cast());
    free(dict.live);
}

/// Removes every entry, leaving a dict with no table, as CPython's `dict.clear` does.
///
/// # Safety
///
/// `dict` points to a live dict.
pub(crate) unsafe fn clear(dict: *mut DictObject) {
    // SAFETY: the caller passes a live dict; its table is replaced by none.
    unsafe {
        let dict = &mut *dict;
        release_table(dict);
        dict.indices = core::ptr::null_mut();
        dict.entries = core::ptr::null_mut();
        dict.live = core::ptr::null_mut();
        dict.log2 = 0;
        dict.used = 0;
        dict.filled = 0;
        dict.usable = 0;
    }
}

/// Frees a dict whose last reference is gone, with its keys and values.
///
/// # Safety
///
/// `dict` points to a live dict that nothing else refers to.
pub(crate) unsafe fn destroy(dict: *mut DictObject) {
    // SAFETY: the caller's guarantee.
    unsafe { release_table(&*dict) };
    free(dict.cast());
}

/// An empty dict that is never freed: the first value the compiler gives a dict variable,
/// never read, as the placeholder list is.
#[repr(transparent)]
pub struct Placeholder(DictObject);

// SAFETY: the placeholder is immortal and never written, so sharing it cannot race.
unsafe impl Sync for Placeholder {}

/// The placeholder dict generated code refers to.
#[unsafe(no_mangle)]
#[allow(
    non_upper_case_globals,
    reason = "the runtime's symbols are `typy_rt_` and a snake-case name"
)]
pub static typy_rt_placeholder_dict: Placeholder = Placeholder(DictObject {
    header: ObjectHeader {
        reference_count: crate::header::IMMORTAL,
        type_id: DICT_TYPE_ID,
    },
    used: 0,
    filled: 0,
    usable: 0,
    log2: 0,
    key_kind: 0,
    value_kind: 0,
    key_words: 1,
    key_tuple: core::ptr::null(),
    value_tuple: core::ptr::null(),
    indices: core::ptr::null_mut(),
    entries: core::ptr::null_mut(),
    live: core::ptr::null_mut(),
});
