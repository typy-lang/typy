//! `zip(*iterables)`, a zip of as many iterables as the list `*` gives holds (EXP-22,
//! ADR-0083): it makes an iterator of each when it is made, as CPython's `zip` does, and each
//! step takes one item of each into a new `tuple[T, ...]`. With `strict`, iterables of
//! different lengths raise CPython's `ValueError`.

use super::{IteratorKind, IteratorObject, made, typy_rt_iterable_iterator};
use crate::Raising;
use crate::exception::ExceptionObject;
use crate::header::ObjectHeader;
use crate::list::{ItemKind, ItemType, ListObject};
use crate::refcount::typy_rt_decrement;
use crate::tuple::TupleType;

/// Returns a new zip of the iterables `iterables` holds, whose items are of the kind the
/// `int` word `kind` names, with the descriptor at `tuple` for tuple items, or the exception
/// making an iterator of one of them raised. `strict` makes iterables of different lengths
/// raise `ValueError`.
///
/// # Safety
///
/// `iterables` points to a live list of iterable objects, and `tuple` is 0 or the address
/// of a live tuple descriptor.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn typy_rt_zip_many(
    iterables: *const ListObject,
    kind: i64,
    tuple: u64,
    strict: bool,
) -> Raising<*mut IteratorObject> {
    // SAFETY: the caller's guarantee.
    let iterables = unsafe { &*iterables };
    let iterators = crate::list::new_list(ItemKind::Object, iterables.length);
    let descriptor: *const TupleType =
        core::ptr::with_exposed_provenance(usize::try_from(tuple).unwrap_or(0));
    for index in 0..iterables.length {
        // SAFETY: each item of the list is a live iterable object.
        let made = unsafe {
            let iterable = core::ptr::with_exposed_provenance_mut::<ObjectHeader>(
                usize::try_from(iterables.item(index)).unwrap_or(0),
            );
            typy_rt_iterable_iterator(iterable, descriptor)
        };
        if !made.exception.is_null() {
            // SAFETY: the list of iterators is live and owned here.
            unsafe { typy_rt_decrement(iterators.cast()) };
            return Raising::error(core::ptr::null_mut(), made.exception);
        }
        // SAFETY: the list has room for every iterator; it takes a reference of its own, and
        // the new one is given up.
        unsafe {
            crate::list::push_retained(iterators, made.value.expose_provenance() as u64);
            typy_rt_decrement(made.value.cast());
        }
    }
    let code = usize::try_from(crate::word::inline_value(kind)).unwrap_or(0);
    // SAFETY: the zip takes over the list's reference.
    let zip = unsafe {
        made(
            iterators.cast(),
            IteratorKind::ZipMany,
            (code, usize::from(strict)),
            core::ptr::null(),
        )
    };
    // SAFETY: the zip is new.
    unsafe { (*zip).pair_type = descriptor };
    Raising::ok(zip)
}

/// Steps a zip: one item of each iterator into a new tuple, which the zip keeps as its
/// current item; false when an iterator has no item left, or `ValueError` for `strict`
/// iterators of different lengths.
///
/// # Safety
///
/// `zip` is a live zip whose source is its live list of iterators.
#[allow(
    clippy::cast_ptr_alignment,
    reason = "the source of a zip is a list, which malloc aligned for every type"
)]
pub(super) unsafe fn step(zip: &mut IteratorObject) -> Result<bool, *mut ExceptionObject> {
    // SAFETY: the caller's guarantee.
    let iterators = unsafe { &*zip.source.cast::<ListObject>() };
    // A zip of no iterables has no item.
    if iterators.length == 0 {
        return Ok(false);
    }
    let ty = ItemType::of(
        u32::try_from(zip.position).unwrap_or(u32::MAX),
        zip.pair_type,
    );
    let tuple = crate::list::new_list(ty, iterators.length);
    for index in 0..iterators.length {
        // SAFETY: each item is a live iterator the list owns.
        let found = unsafe { advance(iterators, index) };
        let found = found.inspect_err(|_| {
            // SAFETY: the tuple is new and owned here.
            unsafe { typy_rt_decrement(tuple.cast()) };
        })?;
        if !found {
            // SAFETY: the tuple is new and owned here.
            unsafe { typy_rt_decrement(tuple.cast()) };
            return if zip.length == 0 {
                Ok(false)
            } else {
                // SAFETY: the caller's guarantee.
                unsafe { unequal(iterators, index) }
            };
        }
        // SAFETY: the iterator's item is live, and the tuple has room for it.
        unsafe { crate::list::push_retained(tuple, iterator_item(iterators, index)) };
    }
    // SAFETY: the tuple is new; the zip owns it from now on, and releases the one before.
    unsafe {
        crate::list::typy_rt_tuple_seal(tuple);
        let before = core::mem::replace(&mut zip.owned, tuple.cast());
        if !before.is_null() {
            typy_rt_decrement(before);
        }
    }
    zip.current = tuple.expose_provenance() as u64;
    Ok(true)
}

/// Returns the error of `strict` iterators of different lengths once iterator `ended` has
/// no item left: shorter than those before it, or, for the first, longer for the first that
/// still has one; false when they all ended together.
///
/// # Safety
///
/// `iterators` is a live list of live iterators.
unsafe fn unequal(iterators: &ListObject, ended: usize) -> Result<bool, *mut ExceptionObject> {
    let word = |value: usize| crate::word::inline(i64::try_from(value).unwrap_or(0)).unwrap_or(1);
    if ended > 0 {
        return Err(crate::zip::typy_rt_zip_unequal(word(ended), word(0)));
    }
    for index in 1..iterators.length {
        // SAFETY: the caller's guarantee.
        if unsafe { advance(iterators, index) }? {
            return Err(crate::zip::typy_rt_zip_unequal(word(index), word(1)));
        }
    }
    Ok(false)
}

/// Steps iterator `index` of `iterators`.
///
/// # Safety
///
/// `iterators` is a live list of live iterators, with more than `index` of them.
unsafe fn advance(iterators: &ListObject, index: usize) -> Result<bool, *mut ExceptionObject> {
    // SAFETY: the caller's guarantee.
    let stepped = unsafe {
        super::typy_rt_iterator_advance(
            iterator_at(iterators, index),
            crate::word::inline(0).unwrap_or(1),
        )
    };
    if stepped.exception.is_null() {
        Ok(stepped.value)
    } else {
        Err(stepped.exception)
    }
}

/// Returns the item iterator `index` of `iterators` moved to.
///
/// # Safety
///
/// As for `advance`, and the iterator's last step found an item.
unsafe fn iterator_item(iterators: &ListObject, index: usize) -> u64 {
    // SAFETY: the caller's guarantee.
    unsafe { super::typy_rt_iterator_item(iterator_at(iterators, index)) }
}

/// Returns iterator `index` of `iterators`.
///
/// # Safety
///
/// As for `advance`.
unsafe fn iterator_at(iterators: &ListObject, index: usize) -> *mut IteratorObject {
    // SAFETY: the caller's guarantee.
    core::ptr::with_exposed_provenance_mut(
        usize::try_from(unsafe { iterators.item(index) }).unwrap_or(0),
    )
}
