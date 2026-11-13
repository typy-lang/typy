//! The set functions that take another collection: a set, a dict of the set's items as keys,
//! a view of a dict's keys or items, or a list or `tuple[T, ...]` of the set's items, which is
//! what generated code makes of any other iterable (DAT-15, DAT-16). Each takes
//! the steps CPython's `setobject.c` takes for that kind of operand, so the result holds its
//! items in CPython's order.

use super::algebra::{self, copy, is_disjoint, is_subset, purge};
use super::{SetObject, add, discard, lookup, new_set, resize};
use crate::dict::DictObject;
use crate::dict::view::ViewObject;
use crate::header::{DICT_TYPE_ID, DICT_VIEW_TYPE_ID, SET_TYPE_ID};
use crate::list::ListObject;

/// The other operand of a set function.
pub(super) enum Other<'a> {
    /// A set, whose table the set algorithms read.
    Set(&'a SetObject),
    /// A dict, whose keys CPython reads in order with the hashes it stored.
    Dict(&'a DictObject),
    /// A list or `tuple[T, ...]` of the set's items, read in order.
    Items(&'a ListObject),
    /// A view of a dict's keys, read in order as CPython reads any iterable.
    Keys(&'a ViewObject),
    /// A view of a dict's items, read in order as `(key, value)` pairs.
    Pairs(&'a ViewObject),
}

/// Returns the operand at `address`.
///
/// # Safety
///
/// `address` is the address of a live set, dict, view, list or `tuple[T, ...]`.
#[allow(
    clippy::cast_ptr_alignment,
    reason = "every object comes from malloc, aligned for every type, and starts with its header"
)]
pub(super) unsafe fn other<'a>(address: u64) -> Other<'a> {
    let header = core::ptr::with_exposed_provenance::<crate::header::ObjectHeader>(
        usize::try_from(address).unwrap_or(0),
    );
    // SAFETY: the caller passes a live object; its type ID says which it is.
    unsafe {
        match (*header).type_id {
            SET_TYPE_ID => Other::Set(&*header.cast::<SetObject>()),
            DICT_TYPE_ID => Other::Dict(&*header.cast::<DictObject>()),
            DICT_VIEW_TYPE_ID => {
                let view = &*header.cast::<ViewObject>();
                if view.kind == 0 {
                    Other::Keys(view)
                } else {
                    Other::Pairs(view)
                }
            }
            _ => Other::Items(&*header.cast::<ListObject>()),
        }
    }
}

/// Calls `visit` with each item of an operand and its hash, in order, until it returns false:
/// the items of a list, the keys of a dict or a keys view with their stored hashes, each
/// `(key, value)` pair of an items view, laid out as the set's tuple items, or the items of a
/// set in table order. Returns whether every call returned true.
///
/// # Safety
///
/// `other` holds items of the set's type.
pub(super) unsafe fn each_item(
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
            Other::Keys(view) => {
                let dict = &*view.dict;
                dict.live_entries()
                    .all(|index| visit(dict.key(index), dict.hash(index)))
            }
            Other::Pairs(view) => each_pair(set, &*view.dict, true, visit),
            Other::Set(source) => source
                .active()
                .all(|slot| visit(source.item(slot), source.hash(slot))),
        }
    }
}

/// Calls `visit` with each `(key, value)` pair of `dict` and its hash, or 0 when `hashed` is
/// false, in order, built in room laid out as the tuple items of `set`, until it returns
/// false; returns whether every call returned true. A pair borrows its key and value, so
/// `visit` keeps it only by a reference of its own. A pair whose value has no hash is read
/// only unhashed, as CPython compares two items views by looking up each key.
///
/// # Safety
///
/// `set` holds tuples of the dict's key and value types, and `dict` is live.
pub(super) unsafe fn each_pair(
    set: *const SetObject,
    dict: &DictObject,
    hashed: bool,
    mut visit: impl FnMut(u64, i64) -> bool,
) -> bool {
    // SAFETY: the caller's guarantee; the room is as large as one pair.
    unsafe {
        let item_type = (*set).item_type();
        let pair_type = (*set).tuple;
        let room = crate::allocator::allocate(usize::try_from((*pair_type).size).unwrap_or(8));
        let address = room.expose_provenance() as u64;
        let all = dict.live_entries().all(|index| {
            crate::tuple::store_field(pair_type, address, 0, dict.key(index));
            crate::tuple::store_field(pair_type, address, 1, dict.value(index));
            visit(address, pair_hash(item_type, address, hashed))
        });
        crate::allocator::free(room);
        all
    }
}

/// Returns the hash of the pair at `address`, or 0 when it is read `hashed` false.
///
/// # Safety
///
/// `address` holds a live pair of `item_type`.
unsafe fn pair_hash(item_type: crate::list::ItemType, address: u64, hashed: bool) -> i64 {
    if !hashed {
        return 0;
    }
    // SAFETY: the caller's guarantee.
    unsafe { item_type.hash(address) }
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
            Other::Items(_) | Other::Keys(_) | Other::Pairs(_) => add_each(set, &operand),
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
            Other::Items(_) | Other::Keys(_) | Other::Pairs(_) => {
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
