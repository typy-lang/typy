//! Tests of stepping an instance of a class with `__next__` and `__iter__` (TYP-20).

use crate::exception::{ExceptionKind, ExceptionObject, raise, raise_bare};
use crate::header::ObjectHeader;
use crate::instance::{InstanceObject, typy_rt_instance_new};
use crate::iterator::{
    typy_rt_iter, typy_rt_iterable_iterator, typy_rt_iterator_advance, typy_rt_iterator_item,
    typy_rt_iterator_stepper,
};
use crate::list::ItemKind;
use crate::refcount::{typy_rt_decrement, typy_rt_increment};
use crate::string::{StrObject, from_str};
use crate::tuple::{Field, TupleType};
use crate::type_object::{IterEntry, NextEntry, OBJECT, TypeObject};
use crate::word::{inline, inline_value};

fn word(value: i64) -> i64 {
    inline(value).unwrap()
}

/// Returns the first attribute of an instance, an `int` word.
///
/// # Safety
///
/// `object` is an instance of a class made by `class`.
#[allow(
    clippy::cast_ptr_alignment,
    reason = "an instance comes from malloc, aligned for every type"
)]
unsafe fn counter(object: *mut ObjectHeader) -> *mut i64 {
    // SAFETY: the attributes follow the fixed part of the instance.
    unsafe { object.cast::<InstanceObject>().add(1).cast::<i64>() }
}

/// Counts down the instance's first attribute and gives each number, then raises
/// `StopIteration`.
unsafe extern "C" fn count_next(object: *mut ObjectHeader, slot: *mut u64) -> *mut ExceptionObject {
    // SAFETY: the instance holds an `int` word first, and the slot has room for one.
    unsafe {
        let left = inline_value(*counter(object));
        if left == 0 {
            return raise_bare(ExceptionKind::StopIteration);
        }
        *counter(object) = word(left - 1);
        *slot = word(left).cast_unsigned();
    }
    core::ptr::null_mut()
}

/// Gives the instance itself, as an iterator's `__iter__` does.
unsafe extern "C" fn own_iter(object: *mut ObjectHeader, slot: *mut u64) -> *mut ExceptionObject {
    // SAFETY: the entry gives a reference of its own.
    unsafe {
        typy_rt_increment(object);
        *slot = object.expose_provenance() as u64;
    }
    core::ptr::null_mut()
}

/// Gives two `int` words, a pair of the number and its double, while the count lasts.
unsafe extern "C" fn pair_next(object: *mut ObjectHeader, slot: *mut u64) -> *mut ExceptionObject {
    // SAFETY: the slot has room for the pair.
    unsafe {
        let left = inline_value(*counter(object));
        if left == 0 {
            return raise_bare(ExceptionKind::StopIteration);
        }
        *counter(object) = word(left - 1);
        *slot = word(left).cast_unsigned();
        *slot.add(1) = word(left * 2).cast_unsigned();
    }
    core::ptr::null_mut()
}

/// Gives a new string each step.
unsafe extern "C" fn text_next(object: *mut ObjectHeader, slot: *mut u64) -> *mut ExceptionObject {
    // SAFETY: as for `count_next`; the string's reference passes to the slot.
    unsafe {
        let left = inline_value(*counter(object));
        if left == 0 {
            return raise_bare(ExceptionKind::StopIteration);
        }
        *counter(object) = word(left - 1);
        *slot = from_str("word").expose_provenance() as u64;
    }
    core::ptr::null_mut()
}

/// Raises `ValueError`.
unsafe extern "C" fn broken(_: *mut ObjectHeader, _: *mut u64) -> *mut ExceptionObject {
    raise(ExceptionKind::ValueError, "broken")
}

/// Gives a runtime iterator over a new list of one `int`, as `__iter__` returning `iter([7])`.
unsafe extern "C" fn list_iter(_: *mut ObjectHeader, slot: *mut u64) -> *mut ExceptionObject {
    let list = crate::list::typy_rt_list_new(word(ItemKind::Int as i64), word(1), 0);
    // SAFETY: the list is live; the iterator takes its own reference to it.
    unsafe {
        crate::list::typy_rt_list_append(list, word(7).cast_unsigned());
        let made = typy_rt_iterable_iterator(list.cast(), core::ptr::null()).value;
        typy_rt_decrement(list.cast());
        *slot = made.expose_provenance() as u64;
    }
    core::ptr::null_mut()
}

/// Returns a class whose instances hold one `int` word, with the entries given.
fn class(next: Option<NextEntry>, iter: Option<IterEntry>, kind: ItemKind) -> &'static TypeObject {
    let fields = Box::leak(Box::new([Field {
        kind: ItemKind::Int as u32,
        offset: 0,
        tuple: core::ptr::null(),
    }]));
    let layout = Box::leak(Box::new(TupleType {
        size: 8,
        length: 1,
        fields: fields.as_ptr(),
    }));
    let mut class = TypeObject::new("Steps", &raw const OBJECT);
    class.fields = layout;
    class.next = next;
    class.iter = iter;
    class.next_kind = kind as u64;
    if kind == ItemKind::Tuple {
        let pair = Box::leak(Box::new([
            Field {
                kind: ItemKind::Int as u32,
                offset: 0,
                tuple: core::ptr::null(),
            },
            Field {
                kind: ItemKind::Int as u32,
                offset: 8,
                tuple: core::ptr::null(),
            },
        ]));
        class.next_tuple = Box::leak(Box::new(TupleType {
            size: 16,
            length: 2,
            fields: pair.as_ptr(),
        }));
    }
    Box::leak(Box::new(class))
}

/// Returns a new instance of `class` whose count is `count`.
fn instance(class: &'static TypeObject, count: i64) -> *mut ObjectHeader {
    // SAFETY: the class lays out one `int` word, written here before any use.
    unsafe {
        let object = typy_rt_instance_new(class).cast::<ObjectHeader>();
        *counter(object) = word(count);
        object
    }
}

/// Steps `iterator` to its end and returns the items, as `int` values.
fn drain(iterator: *mut crate::iterator::IteratorObject) -> Vec<i64> {
    let mut items = Vec::new();
    // SAFETY: the iterator is live, and each item is read before the next step.
    unsafe {
        while typy_rt_iterator_advance(iterator, word(0)).value {
            items.push(inline_value(typy_rt_iterator_item(iterator).cast_signed()));
        }
    }
    items
}

#[test]
fn a_loop_asks_iter_then_steps_the_instance_it_gives() {
    let steps = instance(class(Some(count_next), Some(own_iter), ItemKind::Int), 3);
    // SAFETY: the instance is live and owned here.
    unsafe {
        let made = typy_rt_iterable_iterator(steps, core::ptr::null());
        assert!(made.exception.is_null());
        assert_eq!(drain(made.value), [3, 2, 1]);
        // Once the instance ends, the wrapper ends too.
        assert!(!typy_rt_iterator_advance(made.value, word(0)).value);
        let again = typy_rt_iterator_advance(made.value, word(1));
        assert_eq!((*again.exception).kind(), ExceptionKind::StopIteration);
        typy_rt_decrement(again.exception.cast());
        typy_rt_decrement(made.value.cast());
        typy_rt_decrement(steps);
    }
}

#[test]
fn next_steps_the_instance_without_asking_iter_and_raises_its_stop_iteration() {
    let steps = instance(class(Some(count_next), Some(broken), ItemKind::Int), 1);
    // SAFETY: the instance is live and owned here.
    unsafe {
        let stepper = typy_rt_iterator_stepper(steps);
        assert!(typy_rt_iterator_advance(stepper, word(1)).value);
        assert_eq!(
            inline_value(typy_rt_iterator_item(stepper).cast_signed()),
            1
        );
        let end = typy_rt_iterator_advance(stepper, word(1));
        assert_eq!((*end.exception).kind(), ExceptionKind::StopIteration);
        typy_rt_decrement(end.exception.cast());
        typy_rt_decrement(stepper.cast());
        // A runtime iterator steps itself.
        let list = crate::list::typy_rt_list_new(word(ItemKind::Int as i64), word(0), 0);
        let iterator = typy_rt_iterable_iterator(list.cast(), core::ptr::null()).value;
        let same = typy_rt_iterator_stepper(iterator.cast());
        assert_eq!(same, iterator);
        for object in [
            same.cast::<ObjectHeader>(),
            iterator.cast(),
            list.cast(),
            steps,
        ] {
            typy_rt_decrement(object);
        }
    }
}

#[test]
fn an_error_of_next_or_iter_comes_back_to_the_caller() {
    let failing = instance(class(Some(broken), None, ItemKind::Int), 1);
    let refusing = instance(class(Some(count_next), Some(broken), ItemKind::Int), 1);
    // SAFETY: the instances are live and owned here.
    unsafe {
        // Without `__iter__`, the instance is stepped as it is.
        let made = typy_rt_iterable_iterator(failing, core::ptr::null());
        let step = typy_rt_iterator_advance(made.value, word(0));
        assert_eq!((*step.exception).kind(), ExceptionKind::ValueError);
        typy_rt_decrement(step.exception.cast());
        typy_rt_decrement(made.value.cast());
        for raised in [
            typy_rt_iterable_iterator(refusing, core::ptr::null()).exception,
            typy_rt_iter(refusing, core::ptr::null()).exception,
        ] {
            assert_eq!((*raised).kind(), ExceptionKind::ValueError);
            typy_rt_decrement(raised.cast());
        }
        typy_rt_decrement(failing);
        typy_rt_decrement(refusing);
    }
}

#[test]
fn iter_gives_what_the_instance_s_iter_returns_as_it_is() {
    let steps = instance(class(Some(count_next), Some(own_iter), ItemKind::Int), 2);
    let listed = instance(class(None, Some(list_iter), ItemKind::Int), 0);
    // SAFETY: the instances are live and owned here.
    unsafe {
        let itself = typy_rt_iter(steps, core::ptr::null());
        assert_eq!(itself.value, steps);
        typy_rt_decrement(itself.value);
        // An iterable whose `__iter__` gives a runtime iterator is stepped through it.
        let made = typy_rt_iterable_iterator(listed, core::ptr::null()).value;
        assert_eq!(drain(made), [7]);
        typy_rt_decrement(made.cast());
        let list = crate::list::typy_rt_list_new(word(ItemKind::Int as i64), word(0), 0);
        let built = typy_rt_iter(list.cast(), core::ptr::null());
        assert!(built.exception.is_null());
        for object in [built.value, list.cast(), steps, listed] {
            typy_rt_decrement(object);
        }
    }
}

#[test]
fn a_wrapper_holds_each_item_until_the_next_step_and_frees_it() {
    let pairs = instance(class(Some(pair_next), None, ItemKind::Tuple), 2);
    let words = instance(class(Some(text_next), None, ItemKind::Object), 2);
    // SAFETY: the instances are live and owned here; each item is read while it is held.
    unsafe {
        let made = typy_rt_iterable_iterator(pairs, core::ptr::null()).value;
        assert!(typy_rt_iterator_advance(made, word(0)).value);
        let pair = core::ptr::with_exposed_provenance::<i64>(
            usize::try_from(typy_rt_iterator_item(made)).unwrap(),
        );
        assert_eq!([inline_value(*pair), inline_value(*pair.add(1))], [2, 4]);
        typy_rt_decrement(made.cast());
        let made = typy_rt_iterable_iterator(words, core::ptr::null()).value;
        assert!(typy_rt_iterator_advance(made, word(0)).value);
        let text = core::ptr::with_exposed_provenance::<StrObject>(
            usize::try_from(typy_rt_iterator_item(made)).unwrap(),
        );
        assert_eq!((*text).header.reference_count, 1);
        // The next step releases the item it held; freeing the wrapper releases the last.
        assert!(typy_rt_iterator_advance(made, word(0)).value);
        typy_rt_decrement(made.cast());
        typy_rt_decrement(pairs);
        typy_rt_decrement(words);
    }
}
