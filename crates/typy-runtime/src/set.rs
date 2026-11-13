//! `set`: a hash table of items, laid out and probed as CPython's `setobject.c` does, so a set
//! iterates and prints in CPython's order (DAT-10, ADR-0010). Items are stored by their type,
//! as a list stores them (ADR-0009): each slot holds an item's hash and its words.

mod algebra;
mod operations;
#[cfg(test)]
pub(crate) use operations::typy_rt_set_add_moved;
mod others;
mod views;

pub(crate) use operations::{
    key_error, typy_rt_set_compare, typy_rt_set_item, typy_rt_set_next, typy_rt_set_repr,
};
#[cfg(test)]
pub(crate) use operations::{typy_rt_set_add, typy_rt_set_new};
#[cfg(test)]
mod tests;

use crate::allocator::{allocate, array_size, free};
use crate::header::{ObjectHeader, SET_TYPE_ID};
use crate::list::ItemType;
use crate::tuple::TupleType;

/// The number of slots of a new table, CPython's `PySet_MINSIZE`.
const MINIMUM_SIZE: usize = 8;
/// The slots probed one after another before the probe jumps, CPython's `LINEAR_PROBES`.
const LINEAR_PROBES: usize = 9;
/// How far the probe's perturbation shifts each jump, CPython's `PERTURB_SHIFT`.
const PERTURB_SHIFT: u32 = 5;
/// Above this many items a set grows to twice its items, below it to four times.
const LARGE_SET: usize = 50_000;

/// A slot that has never held an item; a probe stops at it.
const EMPTY: u8 = 0;
/// A slot that holds an item.
const ACTIVE: u8 = 1;
/// A slot whose item was removed, CPython's `dummy`; a probe passes over it.
const DUMMY: u8 = 2;

/// A set. Its slots live in a table of their own, so the set keeps its address as it grows.
#[repr(C)]
pub struct SetObject {
    /// The object header.
    pub header: ObjectHeader,
    /// The slots that are active or dummy.
    pub fill: usize,
    /// The slots that are active: the number of items.
    pub used: usize,
    /// The number of slots, less one; the number of slots is a power of two.
    pub mask: usize,
    /// Where `pop` starts to look, CPython's `finger`.
    pub finger: usize,
    /// How the items are stored, as an `ItemKind`.
    pub kind: u32,
    /// The eight-byte words an item takes.
    pub words: u32,
    /// The descriptor of tuple items, or null.
    pub tuple: *const TupleType,
    /// The slots, each a hash word followed by the item's words.
    pub table: *mut u64,
    /// The state of each slot: `EMPTY`, `ACTIVE` or `DUMMY`.
    pub states: *mut u8,
}

impl SetObject {
    /// Returns the type of the set's items.
    pub(crate) fn item_type(&self) -> ItemType {
        ItemType::of(self.kind, self.tuple)
    }

    /// Returns the words a slot takes: the hash, then the item.
    fn stride(&self) -> usize {
        1 + usize::try_from(self.words).unwrap_or(1)
    }

    /// Returns the state of `slot`.
    ///
    /// # Safety
    ///
    /// `slot` is at most the mask.
    pub(crate) unsafe fn state(&self, slot: usize) -> u8 {
        // SAFETY: the caller keeps the slot inside the table.
        unsafe { *self.states.add(slot) }
    }

    /// Returns where `slot` begins: its hash word.
    ///
    /// # Safety
    ///
    /// `slot` is at most the mask.
    unsafe fn entry(&self, slot: usize) -> *mut u64 {
        // SAFETY: the caller keeps the slot inside the table.
        unsafe { self.table.add(slot * self.stride()) }
    }

    /// Returns the hash stored in `slot`.
    ///
    /// # Safety
    ///
    /// `slot` is at most the mask and active.
    pub(crate) unsafe fn hash(&self, slot: usize) -> i64 {
        // SAFETY: the caller's guarantee.
        unsafe { (*self.entry(slot)).cast_signed() }
    }

    /// Returns the item in `slot`: its eight bytes, or the address of a tuple.
    ///
    /// # Safety
    ///
    /// `slot` is at most the mask and active.
    pub(crate) unsafe fn item(&self, slot: usize) -> u64 {
        // SAFETY: the caller's guarantee; the item follows the hash word.
        unsafe { self.item_type().load(self.entry(slot).add(1)) }
    }

    /// Returns the active slots in table order, the order CPython iterates a set in.
    pub(crate) fn active(&self) -> impl Iterator<Item = usize> + '_ {
        // SAFETY: every slot up to the mask is inside the table.
        (0..=self.mask).filter(move |slot| unsafe { self.state(*slot) } == ACTIVE)
    }
}

/// An empty set that is never freed: the first value the compiler gives a set variable, so
/// every path out of a function can release it. Definite assignment guarantees no program reads
/// it, so it has no table and is never changed.
#[repr(transparent)]
pub struct Placeholder(SetObject);

// SAFETY: the placeholder is immortal and never written, so sharing it cannot race.
unsafe impl Sync for Placeholder {}

/// The placeholder set generated code refers to.
#[unsafe(no_mangle)]
#[allow(
    non_upper_case_globals,
    reason = "the runtime's symbols are `typy_rt_` and a snake-case name"
)]
pub static typy_rt_placeholder_set: Placeholder = Placeholder(SetObject {
    header: ObjectHeader {
        reference_count: crate::header::IMMORTAL,
        type_id: SET_TYPE_ID,
    },
    fill: 0,
    used: 0,
    mask: 0,
    finger: 0,
    kind: 0,
    words: 1,
    tuple: core::ptr::null(),
    table: core::ptr::null_mut(),
    states: core::ptr::null_mut(),
});

/// Returns a new table of `size` slots of `stride` words, every slot empty.
fn new_table(size: usize, stride: usize) -> (*mut u64, *mut u8) {
    let words = array_size(size, 8 * stride, 0);
    let block = allocate(array_size(size, 1, words));
    // SAFETY: the block holds the slots, then one state byte per slot.
    unsafe {
        let states = block.add(words);
        states.write_bytes(EMPTY, size);
        #[allow(
            clippy::cast_ptr_alignment,
            reason = "malloc returns memory aligned for every type"
        )]
        (block.cast::<u64>(), states)
    }
}

/// Returns a new empty set of items of `item_type`.
pub(crate) fn new_set(item_type: ItemType) -> *mut SetObject {
    let words = u32::try_from(item_type.words()).unwrap_or(1);
    let (table, states) = new_table(MINIMUM_SIZE, 1 + item_type.words());
    #[allow(
        clippy::cast_ptr_alignment,
        reason = "malloc returns memory aligned for every type"
    )]
    let set = allocate(core::mem::size_of::<SetObject>()).cast::<SetObject>();
    // SAFETY: `set` points to enough writable bytes for a `SetObject`.
    unsafe {
        set.write(SetObject {
            header: ObjectHeader::new(SET_TYPE_ID),
            fill: 0,
            used: 0,
            mask: MINIMUM_SIZE - 1,
            finger: 0,
            kind: item_type.kind() as u32,
            words,
            tuple: item_type.descriptor(),
            table,
            states,
        });
    }
    set
}

/// Calls `visit` on the slots a probe for `hash` passes, in CPython's order, until it returns
/// `Some`: runs of up to ten slots, each run begun by a jump the hash perturbs.
fn probe<T>(mask: usize, hash: i64, mut visit: impl FnMut(usize) -> Option<T>) -> T {
    #[allow(
        clippy::cast_possible_truncation,
        reason = "CPython takes the hash as a `size_t`, which is 64 bits on every target TyPy supports"
    )]
    let mut perturb = hash.cast_unsigned() as usize;
    let mut slot = perturb & mask;
    loop {
        let run = if slot + LINEAR_PROBES <= mask {
            LINEAR_PROBES
        } else {
            0
        };
        for next in slot..=slot + run {
            if let Some(found) = visit(next) {
                return found;
            }
        }
        perturb >>= PERTURB_SHIFT;
        slot = (slot.wrapping_mul(5).wrapping_add(1).wrapping_add(perturb)) & mask;
    }
}

/// Returns the slot that holds an item equal to `item`, or `None`.
///
/// # Safety
///
/// `set` points to a live set, and `item` is a live item of its type whose hash is `hash`.
pub(crate) unsafe fn lookup(set: *const SetObject, item: u64, hash: i64) -> Option<usize> {
    // SAFETY: the caller's guarantee; the probe stays inside the table.
    unsafe {
        let set = &*set;
        let item_type = set.item_type();
        probe(set.mask, hash, |slot| match set.state(slot) {
            EMPTY => Some(None),
            ACTIVE if set.hash(slot) == hash && item_type.equal(set.item(slot), item) => {
                Some(Some(slot))
            }
            _ => None,
        })
    }
}

/// Writes `item` and its hash into `slot`, which takes over the item's references.
///
/// # Safety
///
/// `slot` is a slot of the set that is not active.
unsafe fn occupy(set: &mut SetObject, slot: usize, item: u64, hash: i64) {
    // SAFETY: the caller's guarantee; the item follows the hash word.
    unsafe {
        let entry = set.entry(slot);
        *entry = hash.cast_unsigned();
        set.item_type().store(entry.add(1), item);
        *set.states.add(slot) = ACTIVE;
    }
}

/// Adds `item`, whose hash is `hash`, unless an equal item is there; an added item gets a
/// reference of its own. Returns whether it was added. CPython's `set_add_entry`: the item goes
/// to the first empty slot the probe meets, or to the last dummy slot before it.
///
/// # Safety
///
/// `set` points to a live set, and `item` is a live item of its type whose hash is `hash`.
pub(crate) unsafe fn add(set: *mut SetObject, item: u64, hash: i64) -> bool {
    // SAFETY: the caller's guarantee.
    unsafe { add_item(set, item, hash, true) }
}

/// Adds `item` as `add` does, with a reference of its own only when `retain`; otherwise the set
/// takes over the caller's reference, which is released when an equal item is there already.
///
/// # Safety
///
/// As for `add`.
pub(crate) unsafe fn add_item(set: *mut SetObject, item: u64, hash: i64, retain: bool) -> bool {
    // SAFETY: the caller's guarantee; the probe stays inside the table.
    unsafe {
        let item_type = (*set).item_type();
        let mut free_slot = None;
        let found = {
            let set = &*set;
            probe(set.mask, hash, |slot| match set.state(slot) {
                EMPTY => Some(None),
                ACTIVE if set.hash(slot) == hash && item_type.equal(set.item(slot), item) => {
                    Some(Some(slot))
                }
                DUMMY => {
                    free_slot = Some(slot);
                    None
                }
                _ => None,
            })
        };
        if found.is_some() {
            if !retain {
                item_type.release(item);
            }
            return false;
        }
        if retain {
            item_type.retain(item);
        }
        let set = &mut *set;
        set.used += 1;
        if let Some(slot) = free_slot {
            occupy(set, slot, item, hash);
            return true;
        }
        let slot = first_empty(set, hash);
        occupy(set, slot, item, hash);
        set.fill += 1;
        if set.fill * 5 >= set.mask * 3 {
            resize(set, grown_size(set.used));
        }
        true
    }
}

/// Returns the number of items a full set is resized for: four times its items, or twice them
/// for a set of more than 50000.
pub(crate) const fn grown_size(used: usize) -> usize {
    if used > LARGE_SET { used * 2 } else { used * 4 }
}

/// Returns the first empty slot a probe for `hash` meets, CPython's `set_insert_clean`.
///
/// # Safety
///
/// `set` is a live set with an empty slot.
unsafe fn first_empty(set: &SetObject, hash: i64) -> usize {
    probe(set.mask, hash, |slot| {
        // SAFETY: the caller's guarantee; the probe stays inside the table.
        (unsafe { set.state(slot) } == EMPTY).then_some(slot)
    })
}

/// Moves the items to a new table of the smallest power-of-two size above `minimum`, at least
/// eight slots, which drops the dummy slots. CPython keeps a table of eight slots that has no
/// dummies, but no caller asks for one: a table is resized for at most eight slots only when
/// fewer than four items fill at least five slots, so some of those slots are dummies.
///
/// # Safety
///
/// `set` is a live set with fewer than `minimum` items, or a table of at least `minimum` slots.
pub(crate) unsafe fn resize(set: &mut SetObject, minimum: usize) {
    let mut size = MINIMUM_SIZE;
    while size <= minimum {
        size <<= 1;
    }
    let (old_table, old_states, old_mask) = (set.table, set.states, set.mask);
    let stride = set.stride();
    let (table, states) = new_table(size, stride);
    set.table = table;
    set.states = states;
    set.mask = size - 1;
    set.fill = set.used;
    // SAFETY: each old active slot moves, hash and item words together, to an empty new slot.
    unsafe {
        for slot in 0..=old_mask {
            if *old_states.add(slot) != ACTIVE {
                continue;
            }
            let entry = old_table.add(slot * stride);
            let target = first_empty(set, (*entry).cast_signed());
            core::ptr::copy_nonoverlapping(entry, set.entry(target), stride);
            *set.states.add(target) = ACTIVE;
        }
    }
    free(old_table.cast());
}

/// Removes the item in `slot` and returns it, with its references, which pass to the caller;
/// the slot becomes a dummy. The item's words stay in the slot until the set changes.
///
/// # Safety
///
/// `slot` is an active slot of a live set.
pub(crate) unsafe fn take(set: *mut SetObject, slot: usize) -> u64 {
    // SAFETY: the caller's guarantee.
    unsafe {
        let item = (*set).item(slot);
        *(*set).states.add(slot) = DUMMY;
        (*set).used -= 1;
        item
    }
}

/// Removes an item equal to `item`, and returns whether there was one.
///
/// # Safety
///
/// `set` points to a live set, and `item` is a live item of its type whose hash is `hash`.
pub(crate) unsafe fn discard(set: *mut SetObject, item: u64, hash: i64) -> bool {
    // SAFETY: the caller's guarantee; `lookup` returns an active slot.
    unsafe {
        let Some(slot) = lookup(set, item, hash) else {
            return false;
        };
        let removed = take(set, slot);
        (*set).item_type().release(removed);
        true
    }
}

/// Releases every item and frees the table.
///
/// # Safety
///
/// `set` points to a live set; its table is not used again until a new one replaces it.
unsafe fn release_table(set: &SetObject) {
    let item_type = set.item_type();
    // SAFETY: each active slot holds a live item the set holds references for.
    unsafe {
        for slot in set.active() {
            item_type.release(set.item(slot));
        }
    }
    free(set.table.cast());
}

/// Removes every item, leaving a table of eight slots; `pop` keeps its finger, as in CPython.
///
/// # Safety
///
/// `set` points to a live set.
pub(crate) unsafe fn clear(set: *mut SetObject) {
    // SAFETY: the caller passes a live set; the new table replaces the freed one.
    unsafe {
        let set = &mut *set;
        if set.fill == 0 {
            return;
        }
        release_table(set);
        let (table, states) = new_table(MINIMUM_SIZE, set.stride());
        set.table = table;
        set.states = states;
        set.mask = MINIMUM_SIZE - 1;
        set.fill = 0;
        set.used = 0;
    }
}

/// Frees a set whose last reference is gone, with its items.
///
/// # Safety
///
/// `set` points to a live set that nothing else refers to.
pub(crate) unsafe fn destroy(set: *mut SetObject) {
    // SAFETY: the caller's guarantee.
    unsafe { release_table(&*set) };
    free(set.cast());
}
