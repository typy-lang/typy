//! The set functions that take another collection: a set, a dict of the set's items as keys,
//! or a list or `tuple[T, ...]` of the set's items, which is what generated code makes of any
//! other iterable (DAT-15). Each takes
//! the steps CPython's `setobject.c` takes for that kind of operand, so the result holds its
//! items in CPython's order.

use super::algebra::{self, copy, is_disjoint, is_subset, purge};
use super::{SetObject, add, discard, lookup, new_set, resize};
use crate::dict::DictObject;
use crate::header::{DICT_TYPE_ID, SET_TYPE_ID};
use crate::list::ListObject;

/// The other operand of a set function.
enum Other<'a> {
    /// A set, whose table the set algorithms read.
    Set(&'a SetObject),
    /// A dict, whose keys CPython reads in order with the hashes it stored.
    Dict(&'a DictObject),
    /// A list or `tuple[T, ...]` of the set's items, read in order.
    Items(&'a ListObject),
}

/// Returns the operand at `address`.
///
/// # Safety
///
/// `address` is the address of a live set, list or `tuple[T, ...]`.
#[allow(
    clippy::cast_ptr_alignment,
    reason = "every object comes from malloc, aligned for every type, and starts with its header"
)]
unsafe fn other<'a>(address: u64) -> Other<'a> {
    let header = core::ptr::with_exposed_provenance::<crate::header::ObjectHeader>(
        usize::try_from(address).unwrap_or(0),
    );
    // SAFETY: the caller passes a live object; its type ID says which it is.
    unsafe {
        match (*header).type_id {
            SET_TYPE_ID => Other::Set(&*header.cast::<SetObject>()),
            DICT_TYPE_ID => Other::Dict(&*header.cast::<DictObject>()),
            _ => Other::Items(&*header.cast::<ListObject>()),
        }
    }
}

/// Calls `visit` with each item of an operand that is not a set and its hash, in order: the
/// items of a list, or the keys of a dict with their stored hashes, until it returns false;
/// returns whether every call returned true.
///
/// # Safety
///
/// `other` holds items of the set's type.
unsafe fn each_item(
    set: *const SetObject,
    other: &Other<'_>,
    mut visit: impl FnMut(u64, i64) -> bool,
) -> bool {
    // SAFETY: the caller's guarantee; every index is inside the operand.
    unsafe {
        match other {
            Other::Items(items) => {
                let item_type = (*set).item_type();
                (0..items.length).all(|index| {
                    let item = items.item(index);
                    visit(item, item_type.hash(item))
                })
            }
            Other::Dict(dict) => dict
                .live_entries()
                .all(|index| visit(dict.key(index), dict.hash(index))),
            Other::Set(_) => unreachable!("a set operand is read by the set algorithms"),
        }
    }
}

/// Adds each item of `other` to `set`, CPython's `set_update_internal`: nothing when `other`
/// is `skip`, a merge of a set, and each item of a list in order (`update`, `|=`, and `union`,
/// whose `skip` is the receiver).
///
/// # Safety
///
/// `set` points to a live set, and `other` and `skip` are addresses of live collections of its
/// items, or 0.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn typy_rt_set_extend(set: *mut SetObject, other: u64, skip: u64) {
    if other == skip {
        return;
    }
    // SAFETY: the caller's guarantee.
    unsafe {
        let operand = self::other(other);
        match operand {
            Other::Set(source) => algebra::merge(set, source),
            Other::Dict(dict) => {
                // CPython resizes once for every key first, expecting few to be there already.
                let target = &mut *set;
                if (target.fill + dict.used) * 5 >= target.mask * 3 {
                    resize(target, (target.used + dict.used) * 2);
                }
                add_each(set, &operand);
            }
            Other::Items(_) => add_each(set, &operand),
        }
    }
}

/// Adds each item of an operand that is not a set, in order.
///
/// # Safety
///
/// `set` points to a live set, and `other` holds items of its type.
unsafe fn add_each(set: *mut SetObject, other: &Other<'_>) {
    // SAFETY: the caller's guarantee.
    unsafe {
        each_item(set, other, |item, hash| {
            add(set, item, hash);
            true
        });
    }
}

/// Returns a new set of the items of `set` that `other` holds, CPython's `set_intersection`:
/// for a list, each item of it that the set holds, in order, until the result is as large as
/// the set.
///
/// # Safety
///
/// `set` points to a live set, and `other` is the address of a live collection of its items.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn typy_rt_set_intersect(
    set: *const SetObject,
    other: u64,
) -> *mut SetObject {
    // SAFETY: the caller's guarantee.
    unsafe {
        match self::other(other) {
            Other::Set(source) => algebra::intersection(set, source),
            operand => common_items(set, &operand),
        }
    }
}

/// Returns a new set of each item of an operand that is not a set that `set` holds, in the
/// operand's order, stopping when the result is as large as the set.
///
/// # Safety
///
/// `set` points to a live set, and `items` holds items of its type.
unsafe fn common_items(set: *const SetObject, items: &Other<'_>) -> *mut SetObject {
    // SAFETY: the caller's guarantee.
    unsafe {
        let result = new_set((*set).item_type());
        each_item(set, items, |item, hash| {
            if lookup(set, item, hash).is_none() {
                return true;
            }
            add(result, item, hash);
            (*result).used < (*set).used
        });
        result
    }
}

/// Returns a new set of the items of `set` that `other` does not hold, CPython's
/// `set_difference`: for a list, a copy of the set without each item of the list.
///
/// # Safety
///
/// `set` points to a live set, and `other` is the address of a live collection of its items.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn typy_rt_set_subtract(set: *const SetObject, other: u64) -> *mut SetObject {
    // SAFETY: the caller's guarantee.
    unsafe {
        match self::other(other) {
            Other::Set(source) => algebra::difference(set, source),
            // CPython tests each item of a set four times larger than the dict at most.
            Other::Dict(dict) if (*set).used >> 2 <= dict.used => {
                let result = new_set((*set).item_type());
                let source = &*set;
                for slot in source.active() {
                    let (item, hash) = (source.item(slot), source.hash(slot));
                    if crate::dict::find(dict, item, hash).is_none() {
                        add(result, item, hash);
                    }
                }
                result
            }
            operand => {
                let result = copy(set);
                discard_items(result, &operand);
                result
            }
        }
    }
}

/// Removes each item of an operand that is not a set from a set, then rebuilds a table more
/// than a quarter dummy.
///
/// # Safety
///
/// `set` points to a live set, and `items` holds items of its type.
unsafe fn discard_items(set: *mut SetObject, items: &Other<'_>) {
    // SAFETY: the caller's guarantee.
    unsafe {
        each_item(set, items, |item, hash| {
            discard(set, item, hash);
            true
        });
        purge(&mut *set);
    }
}

/// Removes the items of `other` from `set`, CPython's `set_difference_update_internal`
/// (`difference_update`, `-=`).
///
/// # Safety
///
/// `set` points to a live set, and `other` is the address of a live collection of its items.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn typy_rt_set_remove_all(set: *mut SetObject, other: u64) {
    // SAFETY: the caller's guarantee.
    unsafe {
        match self::other(other) {
            Other::Set(source) => algebra::difference_update(set, source),
            operand => discard_items(set, &operand),
        }
    }
}

/// Returns a new set of the items in exactly one of `set` and `other`: a new set updated with
/// `other`, then with `set`, as CPython's `set.symmetric_difference` builds it.
///
/// # Safety
///
/// `set` points to a live set, and `other` is the address of a live collection of its items.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn typy_rt_set_symmetric(
    set: *const SetObject,
    other: u64,
) -> *mut SetObject {
    // SAFETY: the caller's guarantee.
    unsafe {
        let result = new_set((*set).item_type());
        typy_rt_set_extend(result, other, 0);
        algebra::symmetric_difference_update(result, set);
        result
    }
}

/// Keeps the items in exactly one of `set` and `other`, CPython's
/// `set.symmetric_difference_update`: a list is first made a set of its items in order.
///
/// # Safety
///
/// `set` points to a live set, and `other` is the address of a live collection of its items.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn typy_rt_set_toggle(set: *mut SetObject, other: u64) {
    // SAFETY: the caller's guarantee; the set made from a list is released here.
    unsafe {
        match self::other(other) {
            Other::Set(source) => algebra::symmetric_difference_update(set, source),
            operand @ Other::Dict(_) => {
                each_item(set, &operand, |item, hash| {
                    if !discard(set, item, hash) {
                        add(set, item, hash);
                    }
                    true
                });
            }
            Other::Items(_) => {
                let source = new_set((*set).item_type());
                typy_rt_set_extend(source, other, 0);
                algebra::symmetric_difference_update(set, source);
                crate::refcount::typy_rt_decrement(source.cast());
            }
        }
    }
}

/// Gives `set` the table of `other`, which takes the set's table in exchange; the set keeps
/// its finger, as CPython's `set_swap_bodies` does (`intersection_update`, `&=`).
///
/// # Safety
///
/// Both point to live sets of one item type.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn typy_rt_set_replace(set: *mut SetObject, other: *mut SetObject) {
    // SAFETY: the caller passes two live sets; each table stays owned once.
    unsafe {
        let (target, source) = (&mut *set, &mut *other);
        core::mem::swap(&mut target.fill, &mut source.fill);
        core::mem::swap(&mut target.used, &mut source.used);
        core::mem::swap(&mut target.mask, &mut source.mask);
        core::mem::swap(&mut target.table, &mut source.table);
        core::mem::swap(&mut target.states, &mut source.states);
    }
}

/// Returns whether `set` stands in the relation whose code is the `int` word `relation` to
/// `other`: 0 for `issubset`, 1 for `issuperset` and 2 for `isdisjoint`. For a list,
/// `issubset` intersects the set with it, and the others test each item in order.
///
/// # Safety
///
/// `set` points to a live set, and `other` is the address of a live collection of its items.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn typy_rt_set_relation(
    set: *const SetObject,
    other: u64,
    relation: i64,
) -> bool {
    // SAFETY: the caller's guarantee; the intersection is released here.
    unsafe {
        match (self::other(other), crate::word::inline_value(relation)) {
            (Other::Set(source), 0) => is_subset(set, source),
            (Other::Set(source), 1) => is_subset(source, set),
            (Other::Set(source), _) => is_disjoint(set, source),
            (operand, 0) => {
                let common = common_items(set, &operand);
                let subset = (*common).used == (*set).used;
                crate::refcount::typy_rt_decrement(common.cast());
                subset
            }
            (operand, 1) => each_item(set, &operand, |item, hash| {
                lookup(set, item, hash).is_some()
            }),
            (operand, _) => each_item(set, &operand, |item, hash| {
                lookup(set, item, hash).is_none()
            }),
        }
    }
}
