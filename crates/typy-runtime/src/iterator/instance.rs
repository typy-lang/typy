//! Steps an instance of a class of the program through the entries of its type object
//! (TYP-20, ADR-0065): a runtime iterator wraps the instance inside a loop or a call of
//! `next()`, holds the item its `__next__` gives until the next step, and reads a
//! `StopIteration` as the end.

#[cfg(test)]
mod tests;

use super::{IteratorKind, IteratorObject, drop_source};
use crate::Raising;
use crate::exception::ExceptionKind;
use crate::header::{ITERATOR_TYPE_ID, ObjectHeader};
use crate::list::ItemType;
use crate::refcount::typy_rt_decrement;
use crate::type_object::TypeObject;

/// Returns the class of the instance a wrapper steps.
///
/// # Safety
///
/// `iterator` wraps a live instance.
unsafe fn class(iterator: &IteratorObject) -> &'static TypeObject {
    // SAFETY: the caller's guarantee.
    unsafe { crate::instance::class_of(iterator.source.cast()) }
}

/// Returns how the items of `class`'s `__next__` are held.
fn item_type(class: &TypeObject) -> ItemType {
    ItemType::of(
        u32::try_from(class.next_kind).unwrap_or(0),
        class.next_tuple,
    )
}

/// Returns a new iterator that steps `instance`, taking over the reference the caller holds.
///
/// # Safety
///
/// `instance` points to a live instance of a class with a `next` entry.
pub(super) unsafe fn wrap(instance: *mut ObjectHeader) -> *mut IteratorObject {
    // SAFETY: the caller's guarantee.
    let class = unsafe { crate::instance::class_of(instance.cast()) };
    let words = item_type(class).words();
    #[allow(
        clippy::cast_ptr_alignment,
        reason = "malloc returns memory aligned for every type"
    )]
    let iterator =
        crate::allocator::allocate(core::mem::size_of::<IteratorObject>()).cast::<IteratorObject>();
    let pair = if class.next_tuple.is_null() {
        core::ptr::null_mut()
    } else {
        crate::allocator::allocate(words * 8)
    };
    // SAFETY: the allocation holds an iterator.
    unsafe {
        iterator.write(IteratorObject {
            header: ObjectHeader::new(ITERATOR_TYPE_ID),
            kind: IteratorKind::Instance,
            source: instance,
            position: 0,
            length: 0,
            visited: 0,
            current: 0,
            held: core::ptr::null_mut(),
            number: 0,
            pair_type: class.next_tuple,
            pair,
            owned: core::ptr::null_mut(),
        });
    }
    iterator
}

/// Returns true when `object` is an instance of a class with a `next` entry.
///
/// # Safety
///
/// `object` points to a live object.
pub(super) unsafe fn steps(object: *const ObjectHeader) -> bool {
    // SAFETY: the caller's guarantee; the header says what the object is.
    unsafe {
        (*object).type_id == crate::header::INSTANCE_TYPE_ID
            && crate::instance::class_of(object.cast()).next.is_some()
    }
}

/// Returns the iterator `__iter__` of `instance` gives, with a reference of its own, or the
/// exception it raised; `None` when its class has no `iter` entry.
///
/// # Safety
///
/// `instance` points to a live instance.
pub(super) unsafe fn iter(instance: *mut ObjectHeader) -> Option<Raising<*mut ObjectHeader>> {
    // SAFETY: the caller's guarantee.
    let entry = unsafe { crate::instance::class_of(instance.cast()) }.iter?;
    let mut slot = 0;
    // SAFETY: the entry writes the iterator into the slot.
    let error = unsafe { entry(instance, &raw mut slot) };
    if !error.is_null() {
        return Some(Raising::error(core::ptr::null_mut(), error));
    }
    Some(Raising::ok(core::ptr::with_exposed_provenance_mut(
        usize::try_from(slot).unwrap_or(0),
    )))
}

/// Returns where the item the wrapper holds is stored.
fn slot(iterator: &mut IteratorObject) -> *mut u64 {
    if iterator.pair.is_null() {
        &raw mut iterator.current
    } else {
        iterator.pair.cast()
    }
}

/// Releases the item the wrapper holds, if any.
///
/// # Safety
///
/// `iterator` wraps a live instance.
pub(super) unsafe fn release_item(iterator: &mut IteratorObject) {
    if iterator.visited == 0 {
        return;
    }
    iterator.visited = 0;
    // SAFETY: the caller's guarantee; the slot holds an item of the class's item type.
    unsafe {
        let ty = item_type(class(iterator));
        ty.release(ty.load(slot(iterator)));
    }
}

/// Calls `__next__` of the instance the wrapper steps: true with the item held, false at the
/// end, when the wrapper drops the instance, or the exception `__next__` raised. A
/// `StopIteration` is the end, which `next()` without a default raises as it is, `stop` the
/// `int` word of 1.
///
/// # Safety
///
/// `iterator` wraps an instance, or has ended.
pub(super) unsafe fn advance(iterator: &mut IteratorObject, stop: bool) -> Raising<bool> {
    if iterator.source.is_null() {
        let error = if stop {
            crate::exception::raise_bare(ExceptionKind::StopIteration)
        } else {
            return Raising::ok(false);
        };
        return Raising::error(false, error);
    }
    // SAFETY: the caller's guarantee; the entry writes an item into the slot.
    unsafe {
        release_item(iterator);
        let Some(next) = class(iterator).next else {
            crate::libc::abort_with(b"typy runtime: an instance iterated has no next entry\n")
        };
        let error = next(iterator.source, slot(iterator));
        if error.is_null() {
            iterator.visited = 1;
            return Raising::ok(true);
        }
        // A `StopIteration`, or an exception of a class derived from it, is the end.
        let class = (*error).class();
        if !crate::exception::classes::derives_from(class, ExceptionKind::StopIteration.class()) {
            return Raising::error(false, error);
        }
        drop_source(iterator);
        if stop {
            return Raising::error(false, error);
        }
        typy_rt_decrement(error.cast());
        Raising::ok(false)
    }
}

/// Returns the item the wrapper's last step found: its word, or for a tuple its address.
///
/// # Safety
///
/// The wrapper's last step found an item.
pub(super) unsafe fn item(iterator: &mut IteratorObject) -> u64 {
    // SAFETY: the caller's guarantee.
    unsafe { item_type(class(iterator)).load(slot(iterator)) }
}
