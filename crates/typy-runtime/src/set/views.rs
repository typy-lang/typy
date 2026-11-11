//! The set operators and comparisons of `dict.keys()` and `dict.items()`, with the steps
//! CPython's `dictobject.c` takes, so each result holds its items in CPython's order (DAT-16).
//! `|`, `-` and `^` make a set of the left operand and update it with the right, through the
//! functions of `others.rs`; `&` and `^` of two items views take steps of their own.

use super::others::{Other, each_item, each_pair, other};
use super::{SetObject, add, lookup};
use crate::dict::DictObject;
use crate::dict::view::{ViewObject, typy_rt_dict_view_contains};
use crate::word::inline_value;

/// Returns the number of items of a set or a view operand.
fn size(operand: &Other<'_>) -> usize {
    match operand {
        Other::Set(set) => set.used,
        // SAFETY: a view holds a reference to its live dict.
        Other::View(view) => unsafe { (*view.dict).used },
        Other::Dict(_) | Other::Items(_) => {
            unreachable!("a comparison has a set or a view on each side")
        }
    }
}

/// Returns whether `operand`, a set or a view, holds `item`, whose hash is `hash`.
///
/// # Safety
///
/// `operand` is live, `item` is a live item of its type, and `result` has the item type.
unsafe fn holds(operand: &Other<'_>, result: *const SetObject, item: u64, hash: i64) -> bool {
    // SAFETY: the caller's guarantee.
    unsafe {
        match operand {
            Other::Set(set) => lookup(*set, item, hash).is_some(),
            Other::View(view) => {
                typy_rt_dict_view_contains(*view, item, (*result).tuple.expose_provenance() as u64)
            }
            Other::Dict(_) | Other::Items(_) => {
                unreachable!("a comparison has a set or a view on each side")
            }
        }
    }
}

/// The code of each operator `typy_rt_dict_view_combine` takes.
const UNION: i64 = 0;
const INTERSECTION: i64 = 1;
const DIFFERENCE: i64 = 2;
const SYMMETRIC_DIFFERENCE: i64 = 3;

/// Returns the view at `address`, or `None` when the object there is not a view.
///
/// # Safety
///
/// `address` is the address of a live object.
#[allow(
    clippy::cast_ptr_alignment,
    reason = "every object comes from malloc, aligned for every type, and starts with its header"
)]
unsafe fn view_at(address: u64) -> Option<*const ViewObject> {
    let header = core::ptr::with_exposed_provenance::<crate::header::ObjectHeader>(
        usize::try_from(address).unwrap_or(0),
    );
    // SAFETY: the caller's guarantee.
    let is_view = unsafe { (*header).type_id } == crate::header::DICT_VIEW_TYPE_ID;
    is_view.then(|| header.cast::<ViewObject>())
}

/// Fills the empty set `result` with `left op right`, where `op` is the `int` word of 0 for
/// `|`, 1 for `&`, 2 for `-` and 3 for `^`, and at least one operand is a view of keys or
/// items. Each other operand is a set, a view, a list or a `tuple[T, ...]` of the same items.
///
/// # Safety
///
/// Both are addresses of live collections of the items of `result`, a live empty set.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn typy_rt_dict_view_combine(
    left: u64,
    right: u64,
    operator: i64,
    result: *mut SetObject,
) {
    // SAFETY: the caller's guarantee.
    unsafe {
        let operator = inline_value(operator);
        let views = (view_at(left), view_at(right));
        if operator == INTERSECTION {
            // CPython swaps the operands when the view is on the right.
            return match views {
                (Some(view), _) => intersect(view, right, result),
                (None, Some(view)) => intersect(view, left, result),
                (None, None) => unreachable!("one operand is a view"),
            };
        }
        if let (Some(first), Some(second)) = views
            && (*first).kind == 2
            && (*second).kind == 2
            && operator == SYMMETRIC_DIFFERENCE
        {
            return items_xor(first, second, result);
        }
        // CPython's `dictviews_to_set` makes a set of a keys view from its dict.
        let source = match views.0 {
            Some(view) if (*view).kind == 0 => (*view).dict.expose_provenance() as u64,
            _ => left,
        };
        super::others::typy_rt_set_extend(result, source, 0);
        match operator {
            UNION => super::others::typy_rt_set_extend(result, right, 0),
            DIFFERENCE => super::others::typy_rt_set_remove_all(result, right),
            _ => super::others::typy_rt_set_toggle(result, right),
        }
    }
}

/// Fills the empty set `result` with the items of `view & other`, CPython's
/// `_PyDictView_Intersect`: a set at least as large as the view is intersected with the view,
/// a larger view takes the place of the first, and otherwise each item of `other` that the
/// view holds is added, in `other`'s order.
///
/// # Safety
///
/// `view` points to a live view of keys or items, `other` is the address of a live set, view,
/// list or `tuple[T, ...]` of its items, and `result` points to a live empty set of them.
unsafe fn intersect(view: *const ViewObject, other: u64, result: *mut SetObject) {
    // SAFETY: the caller's guarantee; the set made by the intersection is released here.
    unsafe {
        let length = (*(*view).dict).used;
        let (view, operand) = match self::other(other) {
            Other::Set(set) if length <= set.used => {
                let common =
                    super::others::typy_rt_set_intersect(set, view.expose_provenance() as u64);
                super::others::typy_rt_set_replace(result, common);
                crate::refcount::typy_rt_decrement(common.cast());
                return;
            }
            Other::View(larger) if (*larger.dict).used > length => {
                (core::ptr::from_ref(larger), Other::View(&*view))
            }
            operand => (view, operand),
        };
        let pair = (*result).tuple.expose_provenance() as u64;
        each_item(result, &operand, |item, hash| {
            if typy_rt_dict_view_contains(view, item, pair) {
                add(result, item, hash);
            }
            true
        });
    }
}

/// Fills the empty set `result` with the pairs in exactly one of two items views, CPython's
/// `dictitems_xor`: each pair of the right dict whose key the left maps to another value, or
/// not at all, then the pairs of the left whose key the right does not map to an equal value.
///
/// # Safety
///
/// `left` and `right` point to live views of the items of dicts of one key type and one value
/// type, and `result` points to a live empty set of their pairs.
unsafe fn items_xor(left: *const ViewObject, right: *const ViewObject, result: *mut SetObject) {
    // SAFETY: the caller's guarantee; the copy is released here, and each value taken out of
    // it with the reference it held.
    unsafe {
        let remaining = crate::dict::typy_rt_dict_copy((*left).dict);
        let other: &DictObject = &*(*right).dict;
        let value_type = other.value_type();
        let pair_type = (*result).tuple;
        let item_type = (*result).item_type();
        let room = crate::allocator::allocate(usize::try_from((*pair_type).size).unwrap_or(8));
        let address = room.expose_provenance() as u64;
        for index in other.live_entries() {
            let (key, value) = (other.key(index), other.value(index));
            let same = crate::dict::find(remaining, key, other.hash(index))
                .is_some_and(|position| value_type.equal((*remaining).value(position), value));
            if same {
                let taken = crate::dict::typy_rt_dict_pop(remaining, key);
                value_type.release(taken.value);
            } else {
                crate::tuple::store_field(pair_type, address, 0, key);
                crate::tuple::store_field(pair_type, address, 1, value);
                add(result, address, item_type.hash(address));
            }
        }
        crate::allocator::free(room);
        each_pair(result, &*remaining, true, |pair, hash| {
            add(result, pair, hash);
            true
        });
        crate::refcount::typy_rt_decrement(remaining.cast());
    }
}

/// Returns whether `left` and `right`, each a set or a view of keys or items with at least one
/// view, stand in the comparison whose code is the `int` word `comparison`, CPython's
/// `dictview_richcompare`: the sizes decide first, then whether every item of the smaller side
/// is in the other. The item kind, an `int` word, and the address of the descriptor of tuple
/// items, or 0, are the item type's, as `typy_rt_set_new` takes them.
///
/// # Safety
///
/// Both are addresses of live sets or views of one item type, which `kind` and `tuple` give.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn typy_rt_dict_view_compare(
    left: u64,
    right: u64,
    comparison: i64,
    kind: i64,
    tuple: u64,
) -> bool {
    // An empty set of the item type tells the loops below how the items are laid out.
    let pair = super::operations::typy_rt_set_new(kind, tuple);
    // SAFETY: the caller's guarantee; the empty set is released here.
    unsafe {
        let (left, right) = (other(left), other(right));
        let (left_size, right_size) = (size(&left), size(&right));
        // An item tested against a view is looked up by its key, so it needs no hash.
        let contained = |inner: &Other<'_>, outer: &Other<'_>| match (inner, outer) {
            (Other::View(view), Other::View(_)) if view.kind == 2 => {
                each_pair(pair, &*view.dict, false, |item, hash| {
                    holds(outer, pair, item, hash)
                })
            }
            _ => each_item(pair, inner, |item, hash| holds(outer, pair, item, hash)),
        };
        let holds = match inline_value(comparison) {
            0 => left_size == right_size && contained(&left, &right),
            1 => !(left_size == right_size && contained(&left, &right)),
            2 => left_size < right_size && contained(&left, &right),
            3 => left_size <= right_size && contained(&left, &right),
            4 => left_size > right_size && contained(&right, &left),
            _ => left_size >= right_size && contained(&right, &left),
        };
        crate::refcount::typy_rt_decrement(pair.cast());
        holds
    }
}
