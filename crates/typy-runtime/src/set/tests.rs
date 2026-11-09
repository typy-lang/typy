//! Tests of sets: CPython's order through every operation, items of each kind, errors and
//! freeing.

mod steps;

use super::SetObject;
use super::operations::{
    typy_rt_set_add, typy_rt_set_clear, typy_rt_set_combine, typy_rt_set_compare,
    typy_rt_set_contains, typy_rt_set_copy, typy_rt_set_discard, typy_rt_set_isdisjoint,
    typy_rt_set_item, typy_rt_set_length, typy_rt_set_new, typy_rt_set_next, typy_rt_set_pop,
    typy_rt_set_purge, typy_rt_set_remove, typy_rt_set_repr, typy_rt_set_update,
};
use crate::exception::ExceptionObject;
use crate::int::value::Integer;
use crate::list::ItemKind;
use crate::refcount::{typy_rt_decrement, typy_rt_int_decrement};
use crate::string::{StrObject, code_point, from_str, length};
use crate::tuple::{Field, TupleType};
use crate::word::{inline, inline_value};

/// One step on the two sets of the scripted test.
#[derive(Clone, Copy)]
enum Step {
    /// Adds an `int` to a set.
    Add(usize, i128),
    /// Discards an `int` from a set.
    Discard(usize, i128),
    /// Pops from a set, which gives this `int`.
    Pop(usize, i128),
    /// Replaces a set with it combined with the other by an operation code.
    Combine(usize, i64),
    /// Changes a set in place by an operation code with the other.
    Update(usize, i64),
    /// Empties a set.
    Clear(usize),
    /// Replaces a set with a copy of it.
    Copy(usize),
    /// Checks the text of a set.
    Check(usize, &'static str),
}

fn word(value: i64) -> i64 {
    inline(value).unwrap()
}

fn text(string: *const StrObject) -> String {
    // SAFETY: the string is live and every index is in bounds.
    (0..unsafe { length(string) })
        .map(|index| char::from_u32(unsafe { code_point(string, index) }).unwrap())
        .collect()
}

fn repr(set: *const SetObject) -> String {
    // SAFETY: the set is live; its text is a new string.
    unsafe {
        let result = typy_rt_set_repr(set);
        assert!(result.exception.is_null());
        let shown = text(result.value);
        typy_rt_decrement(result.value.cast());
        shown
    }
}

fn int_set() -> *mut SetObject {
    typy_rt_set_new(word(ItemKind::Int as i64), 0)
}

/// Calls `act` with the `int` word of `value`, then drops the word's own reference.
fn with_int<T>(value: i128, act: impl FnOnce(u64) -> T) -> T {
    let item = Integer::from_i128(value).into_word();
    let result = act(item.cast_unsigned());
    // SAFETY: the word is live and this function holds its reference.
    unsafe { typy_rt_int_decrement(item) };
    result
}

fn ints(values: &[i128]) -> *mut SetObject {
    let set = int_set();
    for value in values {
        // SAFETY: the set is live and the item is a live `int` word.
        with_int(*value, |item| unsafe { typy_rt_set_add(set, item) });
    }
    set
}

fn release(set: *mut SetObject) {
    // SAFETY: the set is live and the caller holds a reference.
    unsafe { typy_rt_decrement(set.cast()) };
}

fn message(exception: *mut ExceptionObject) -> String {
    // SAFETY: the exception is live; the test owns it.
    unsafe {
        let shown = text((*exception).message);
        typy_rt_decrement(exception.cast());
        shown
    }
}

/// Runs one scripted step on the two sets.
///
/// # Safety
///
/// Both sets are live sets of `int` items.
unsafe fn run(sets: &mut [*mut SetObject; 2], step: Step) {
    // SAFETY: the caller passes live sets; every item is a live `int` word.
    unsafe {
        match step {
            Step::Add(at, value) => with_int(value, |item| typy_rt_set_add(sets[at], item)),
            Step::Discard(at, value) => {
                with_int(value, |item| typy_rt_set_discard(sets[at], item));
            }
            Step::Pop(at, value) => {
                let popped = typy_rt_set_pop(sets[at]);
                assert!(popped.exception.is_null());
                let expected = Integer::from_i128(value).into_word();
                assert!(crate::int::typy_rt_int_compare(
                    popped.value.cast_signed(),
                    expected,
                    word(0)
                ));
                typy_rt_int_decrement(expected);
                typy_rt_int_decrement(popped.value.cast_signed());
            }
            Step::Combine(at, operation) => {
                let combined = typy_rt_set_combine(sets[at], sets[1 - at], word(operation));
                release(sets[at]);
                sets[at] = combined;
            }
            Step::Update(at, operation) => {
                typy_rt_set_update(sets[at], sets[1 - at], word(operation));
            }
            Step::Clear(at) => typy_rt_set_clear(sets[at]),
            Step::Copy(at) => {
                let copied = typy_rt_set_copy(sets[at]);
                release(sets[at]);
                sets[at] = copied;
            }
            Step::Check(at, expected) => assert_eq!(repr(sets[at]), expected),
        }
    }
}

#[test]
#[cfg_attr(miri, ignore = "hundreds of steps on growing tables take Miri minutes")]
fn every_operation_keeps_cpythons_order() {
    let mut sets = [int_set(), int_set()];
    for (number, step) in steps::STEPS.iter().enumerate() {
        // SAFETY: both sets stay live through the steps.
        let outcome = std::panic::catch_unwind(core::panic::AssertUnwindSafe(|| unsafe {
            run(&mut sets, *step);
        }));
        assert!(outcome.is_ok(), "step {number} failed");
    }
    sets.into_iter().for_each(release);
}

#[test]
fn items_added_in_order_take_cpythons_slots() {
    let set = ints(&[5, 1, 9, 33, 17, 1024, -3]);
    assert_eq!(repr(set), "{1024, 1, 33, 5, 9, 17, -3}");
    // A copy merges the items into a table of its own size, as a constant display does.
    // SAFETY: the set is live.
    let copied = unsafe { typy_rt_set_copy(set) };
    assert_eq!(repr(copied), "{1024, 1, 33, 17, 5, 9, -3}");
    [set, copied].into_iter().for_each(release);
    let empty = int_set();
    assert_eq!(repr(empty), "set()");
    release(empty);
}

#[test]
fn equal_items_are_kept_once() {
    let set = ints(&[3, 3, 1 << 70, 1 << 70, -1, -2]);
    // SAFETY: the set is live.
    assert_eq!(inline_value(unsafe { typy_rt_set_length(set) }), 4);
    // SAFETY: the set is live and each item is a live `int` word.
    unsafe {
        assert!(with_int(1 << 70, |item| typy_rt_set_contains(set, item)));
        assert!(!with_int(7, |item| typy_rt_set_contains(set, item)));
    }
    release(set);
}

#[test]
fn removing_a_missing_item_raises_key_error_with_its_repr() {
    let set = ints(&[1, 2]);
    // SAFETY: the set is live and each item is a live `int` word.
    unsafe {
        assert!(with_int(2, |item| typy_rt_set_remove(set, item)).is_null());
        let error = with_int(2, |item| typy_rt_set_remove(set, item));
        assert_eq!((*error).kind, crate::exception::ExceptionKind::KeyError);
        assert_eq!(message(error), "2");
    }
    release(set);
}

#[test]
fn popping_an_empty_set_raises_key_error() {
    let set = int_set();
    // SAFETY: the set is live.
    let popped = unsafe { typy_rt_set_pop(set) };
    assert_eq!(message(popped.exception), "'pop from an empty set'");
    release(set);
}

#[test]
fn iteration_visits_active_slots_and_notices_a_change_of_size() {
    let set = ints(&[5, 1, 9]);
    // SAFETY: the set is live; each slot came from `typy_rt_set_next`.
    unsafe {
        let mut seen = Vec::new();
        let mut position = word(0);
        loop {
            let next = typy_rt_set_next(set, position, word(3));
            assert!(next.exception.is_null());
            if inline_value(next.value) < 0 {
                break;
            }
            seen.push(inline_value(
                typy_rt_set_item(set, next.value).cast_signed(),
            ));
            position = word(inline_value(next.value) + 1);
        }
        assert_eq!(seen, [1, 5, 9]);
        with_int(4, |item| typy_rt_set_add(set, item));
        let changed = typy_rt_set_next(set, word(0), word(3));
        assert_eq!(
            message(changed.exception),
            "Set changed size during iteration"
        );
    }
    release(set);
}

#[test]
fn comparisons_test_subsets_and_supersets() {
    let small = ints(&[1, 2]);
    let large = ints(&[1, 2, 3]);
    let other = ints(&[2, 1]);
    // SAFETY: the sets are live.
    unsafe {
        let holds = |left, right, code| typy_rt_set_compare(left, right, word(code));
        assert!(holds(small, other, 0));
        assert!(!holds(small, large, 0));
        assert!(holds(small, large, 1));
        assert!(holds(small, large, 2));
        assert!(!holds(small, other, 2));
        assert!(holds(small, other, 3));
        assert!(holds(large, small, 4));
        assert!(holds(large, small, 5));
        assert!(!holds(small, large, 5));
        assert!(!typy_rt_set_isdisjoint(small, large));
        let apart = ints(&[7, 8, 9, 10]);
        assert!(typy_rt_set_isdisjoint(small, apart));
        assert!(typy_rt_set_isdisjoint(apart, small));
        release(apart);
    }
    [small, large, other].into_iter().for_each(release);
}

#[test]
fn operations_with_the_same_set_follow_cpython() {
    let set = ints(&[1, 2, 3]);
    // SAFETY: the set is live.
    unsafe {
        let both = typy_rt_set_combine(set, set, word(1));
        assert_eq!(repr(both), "{1, 2, 3}");
        release(both);
        let joined = typy_rt_set_combine(set, set, word(0));
        assert_eq!(repr(joined), "{1, 2, 3}");
        release(joined);
        typy_rt_set_update(set, set, word(0));
        assert_eq!(repr(set), "{1, 2, 3}");
        typy_rt_set_update(set, set, word(3));
        assert_eq!(repr(set), "set()");
        with_int(4, |item| typy_rt_set_add(set, item));
        typy_rt_set_update(set, set, word(2));
        assert_eq!(repr(set), "set()");
    }
    release(set);
}

#[test]
fn removing_most_items_lets_the_purge_shrink_the_table() {
    let values: Vec<i128> = (0..40).collect();
    let set = ints(&values);
    // SAFETY: the set is live and each item is a live `int` word.
    unsafe {
        for value in 0..36 {
            with_int(value, |item| typy_rt_set_discard(set, item));
        }
        let before = (*set).mask;
        typy_rt_set_purge(set);
        assert!((*set).mask < before);
        assert_eq!(repr(set), "{36, 37, 38, 39}");
        typy_rt_set_purge(set);
        assert_eq!(repr(set), "{36, 37, 38, 39}");
    }
    release(set);
}

#[test]
fn a_larger_other_set_is_intersected_before_its_items_are_removed() {
    let values: Vec<i128> = (0..100).collect();
    let large = ints(&values);
    let small = ints(&[3, 500, 7]);
    // SAFETY: the sets are live.
    unsafe {
        typy_rt_set_update(small, large, word(2));
        assert_eq!(repr(small), "{500}");
    }
    [large, small].into_iter().for_each(release);
}

#[test]
fn float_items_hash_and_order_as_in_cpython() {
    let set = typy_rt_set_new(word(ItemKind::Float as i64), 0);
    let values = [1.5, 0.1, -2.0, 1e300, 0.0, -0.0, f64::INFINITY, 2.5];
    // SAFETY: the set is live; a float needs no reference.
    unsafe {
        for value in values {
            typy_rt_set_add(set, value.to_bits());
        }
        assert_eq!(repr(set), "{0.1, 1.5, 0.0, 2.5, inf, 1e+300, -2.0}");
        typy_rt_set_add(set, f64::NAN.to_bits());
        typy_rt_set_add(set, f64::NAN.to_bits());
        assert_eq!(inline_value(typy_rt_set_length(set)), 9);
    }
    release(set);
}

fn field(kind: ItemKind, offset: u32) -> Field {
    Field {
        kind: kind as u32,
        offset,
        tuple: core::ptr::null(),
    }
}

#[test]
fn tuple_items_are_stored_inline_and_counted() {
    let fields = Box::leak(Box::new([field(ItemKind::Int, 0), field(ItemKind::Int, 8)]));
    let pair = Box::leak(Box::new(TupleType {
        size: 16,
        length: 2,
        fields: fields.as_ptr(),
    }));
    let set = typy_rt_set_new(
        word(ItemKind::Tuple as i64),
        core::ptr::from_ref::<TupleType>(pair).expose_provenance() as u64,
    );
    // SAFETY: the set is live, and each tuple is two inline `int` words.
    unsafe {
        for (first, second) in [(1, 2), (3, 4), (1, 2), (0, 0), (5, -1), (2, 1)] {
            let words = [word(first).cast_unsigned(), word(second).cast_unsigned()];
            typy_rt_set_add(set, words.as_ptr().expose_provenance() as u64);
        }
        assert_eq!(repr(set), "{(1, 2), (2, 1), (3, 4), (0, 0), (5, -1)}");
    }
    release(set);
}

#[test]
fn string_items_keep_their_references() {
    let set = typy_rt_set_new(word(ItemKind::Object as i64), 0);
    let name = from_str("name");
    // SAFETY: the set and the string are live.
    unsafe {
        typy_rt_set_add(set, name.expose_provenance() as u64);
        typy_rt_set_add(set, name.expose_provenance() as u64);
        assert_eq!((*name).header.reference_count, 2);
        typy_rt_set_clear(set);
        assert_eq!((*name).header.reference_count, 1);
        typy_rt_set_add(set, name.expose_provenance() as u64);
        let popped = typy_rt_set_pop(set);
        assert_eq!(popped.value, name.expose_provenance() as u64);
        typy_rt_decrement(name.cast());
        typy_rt_decrement(name.cast());
        typy_rt_set_clear(set);
    }
    release(set);
}
