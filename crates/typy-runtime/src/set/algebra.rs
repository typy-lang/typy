//! Combines sets as CPython's `setobject.c` does: union, intersection, difference and symmetric
//! difference, and the copies they start from. Each step is CPython's, in CPython's order, so
//! the result's table, and the order it iterates in, is CPython's.

use super::{SetObject, add, discard, first_empty, grown_size, lookup, new_set, occupy, resize};

/// Adds every item of `other` to `set`, CPython's `set_merge`. An empty set takes the items
/// slot by slot when the tables have one size and `other` has no dummies, or by their first
/// empty slot otherwise.
///
/// # Safety
///
/// Both point to live sets of one item type.
pub(crate) unsafe fn merge(set: *mut SetObject, other: *const SetObject) {
    if core::ptr::eq(set, other) {
        return;
    }
    // SAFETY: the caller passes two live sets; every slot read is inside its table.
    unsafe {
        let (set, other) = (&mut *set, &*other);
        if other.used == 0 {
            return;
        }
        if (set.fill + other.used) * 5 >= set.mask * 3 {
            resize(set, (set.used + other.used) * 2);
        }
        let item_type = set.item_type();
        if set.fill == 0 && set.mask == other.mask && other.fill == other.used {
            for slot in other.active() {
                item_type.retain(other.item(slot));
                occupy(set, slot, other.item(slot), other.hash(slot));
            }
            set.fill = other.used;
            set.used = other.used;
            return;
        }
        if set.fill == 0 {
            for slot in other.active() {
                item_type.retain(other.item(slot));
                let target = first_empty(set, other.hash(slot));
                occupy(set, target, other.item(slot), other.hash(slot));
            }
            set.fill = other.used;
            set.used = other.used;
            return;
        }
        for slot in other.active() {
            add(set, other.item(slot), other.hash(slot));
        }
    }
}

/// Returns a new set with the items of `set`, CPython's `set.copy`.
///
/// # Safety
///
/// `set` points to a live set.
pub(crate) unsafe fn copy(set: *const SetObject) -> *mut SetObject {
    // SAFETY: the caller passes a live set.
    unsafe {
        let result = new_set((*set).item_type());
        merge(result, set);
        result
    }
}

/// Returns a new set of the items in both: the items of the smaller set, in its order, that
/// the larger one holds.
///
/// # Safety
///
/// Both point to live sets of one item type.
pub(crate) unsafe fn intersection(
    set: *const SetObject,
    other: *const SetObject,
) -> *mut SetObject {
    // SAFETY: the caller passes two live sets.
    unsafe {
        if core::ptr::eq(set, other) {
            return copy(set);
        }
        let (larger, smaller) = if (*other).used > (*set).used {
            (other, set)
        } else {
            (set, other)
        };
        let result = new_set((*set).item_type());
        let smaller = &*smaller;
        for slot in smaller.active() {
            let (item, hash) = (smaller.item(slot), smaller.hash(slot));
            if lookup(larger, item, hash).is_some() {
                add(result, item, hash);
            }
        }
        result
    }
}

/// Removes the items of `other` from `set`, CPython's `set_difference_update`; a table left
/// with more than a quarter of its slots dummy is resized.
///
/// # Safety
///
/// Both point to live sets of one item type.
pub(crate) unsafe fn difference_update(set: *mut SetObject, other: *const SetObject) {
    // SAFETY: the caller passes two live sets.
    unsafe {
        if core::ptr::eq(set, other) {
            super::clear(set);
            return;
        }
        // When `other` is more than eight times larger, its items in `set` are fewer to visit.
        let common = ((*other).used >> 3 > (*set).used).then(|| intersection(set, other));
        let source = common.map_or(other, <*mut SetObject>::cast_const);
        let source_set = &*source;
        for slot in source_set.active() {
            discard(set, source_set.item(slot), source_set.hash(slot));
        }
        if let Some(common) = common {
            crate::refcount::typy_rt_decrement(common.cast());
        }
        purge(&mut *set);
    }
}

/// Resizes a set when more than a quarter of its slots are dummy.
///
/// # Safety
///
/// `set` is a live set.
pub(crate) unsafe fn purge(set: &mut SetObject) {
    if set.fill - set.used > set.mask / 4 {
        // SAFETY: the caller passes a live set.
        unsafe { resize(set, grown_size(set.used)) };
    }
}

/// Returns a new set of the items of `set` that `other` does not hold. A set four times larger
/// than `other` is copied and the items of `other` removed; otherwise each item of `set` is
/// tested.
///
/// # Safety
///
/// Both point to live sets of one item type.
pub(crate) unsafe fn difference(set: *const SetObject, other: *const SetObject) -> *mut SetObject {
    // SAFETY: the caller passes two live sets.
    unsafe {
        if (*set).used >> 2 > (*other).used {
            let result = copy(set);
            difference_update(result, other);
            return result;
        }
        let result = new_set((*set).item_type());
        let source = &*set;
        for slot in source.active() {
            let (item, hash) = (source.item(slot), source.hash(slot));
            if lookup(other, item, hash).is_none() {
                add(result, item, hash);
            }
        }
        result
    }
}

/// Removes from `set` each item of `other` it holds, and adds each it does not, in the order of
/// `other`'s table.
///
/// # Safety
///
/// Both point to live sets of one item type.
pub(crate) unsafe fn symmetric_difference_update(set: *mut SetObject, other: *const SetObject) {
    // SAFETY: the caller passes two live sets.
    unsafe {
        if core::ptr::eq(set, other) {
            super::clear(set);
            return;
        }
        let other = &*other;
        for slot in other.active() {
            let (item, hash) = (other.item(slot), other.hash(slot));
            if !discard(set, item, hash) {
                add(set, item, hash);
            }
        }
    }
}

/// Returns `left ^ right`: a new set merged from `right`, then updated with `left`, as CPython
/// builds it.
///
/// # Safety
///
/// Both point to live sets of one item type.
pub(crate) unsafe fn symmetric_difference(
    left: *const SetObject,
    right: *const SetObject,
) -> *mut SetObject {
    // SAFETY: the caller passes two live sets.
    unsafe {
        let result = new_set((*left).item_type());
        merge(result, right);
        symmetric_difference_update(result, left);
        result
    }
}

/// Returns `left | right`: a copy of `left` merged with `right`.
///
/// # Safety
///
/// Both point to live sets of one item type.
pub(crate) unsafe fn union(left: *const SetObject, right: *const SetObject) -> *mut SetObject {
    // SAFETY: the caller passes two live sets.
    unsafe {
        let result = copy(left);
        merge(result, right);
        result
    }
}

/// Returns whether every item of `set` is in `other`.
///
/// # Safety
///
/// Both point to live sets of one item type.
pub(crate) unsafe fn is_subset(set: *const SetObject, other: *const SetObject) -> bool {
    // SAFETY: the caller passes two live sets.
    unsafe {
        let source = &*set;
        source.used <= (*other).used
            && source
                .active()
                .all(|slot| lookup(other, source.item(slot), source.hash(slot)).is_some())
    }
}

/// Returns whether the sets have no item in common, testing the items of the smaller.
///
/// # Safety
///
/// Both point to live sets of one item type.
pub(crate) unsafe fn is_disjoint(set: *const SetObject, other: *const SetObject) -> bool {
    // SAFETY: the caller passes two live sets.
    unsafe {
        let (larger, smaller) = if (*other).used > (*set).used {
            (other, &*set)
        } else {
            (set, &*other)
        };
        smaller
            .active()
            .all(|slot| lookup(larger, smaller.item(slot), smaller.hash(slot)).is_none())
    }
}
