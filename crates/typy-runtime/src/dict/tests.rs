//! Tests of dicts: CPython's results through every operation, keys and values of each kind,
//! views, iteration, errors and freeing.

mod steps;

use super::DictObject;
use super::operations::{
    typy_rt_dict_clear, typy_rt_dict_contains, typy_rt_dict_copy, typy_rt_dict_equal,
    typy_rt_dict_find, typy_rt_dict_get, typy_rt_dict_get_or, typy_rt_dict_has_item,
    typy_rt_dict_has_value, typy_rt_dict_key, typy_rt_dict_length, typy_rt_dict_new,
    typy_rt_dict_next, typy_rt_dict_pop, typy_rt_dict_pop_or, typy_rt_dict_popitem,
    typy_rt_dict_set, typy_rt_dict_setdefault, typy_rt_dict_update, typy_rt_dict_value,
};
use super::text::{typy_rt_dict_repr, typy_rt_dict_view_repr};
use crate::exception::ExceptionObject;
use crate::int::value::Integer;
use crate::list::ItemKind;
use crate::refcount::{typy_rt_decrement, typy_rt_int_decrement};
use crate::string::{StrObject, code_point, from_str, length};
use crate::tuple::{Field, TupleType};
use crate::word::{inline, inline_value};

/// One step on the two dicts of the scripted test.
#[derive(Clone, Copy)]
enum Step {
    /// Sets a key to a value.
    Set(usize, i128, i128),
    /// Pops a key, which gives this value or raises `KeyError`.
    Pop(usize, i128, Option<i128>),
    /// Pops the last item, which is this one, or raises `KeyError`.
    PopItem(usize, Option<(i128, i128)>),
    /// Calls `setdefault` with a key and a default, which gives this value.
    SetDefault(usize, i128, i128, i128),
    /// Updates a dict with the other.
    Update(usize),
    /// Replaces a dict with a copy of it.
    Copy(usize),
    /// Empties a dict.
    Clear(usize),
    /// Checks the text of a dict, and whether the two are equal.
    Check(usize, &'static str, bool),
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

fn shown(result: crate::Raising<*mut StrObject>) -> String {
    assert!(result.exception.is_null());
    let value = text(result.value);
    // SAFETY: the text is a new string the test owns.
    unsafe { typy_rt_decrement(result.value.cast()) };
    value
}

fn repr(dict: *const DictObject) -> String {
    // SAFETY: the dict is live.
    shown(unsafe { typy_rt_dict_repr(dict) })
}

fn int_dict() -> *mut DictObject {
    let int = word(ItemKind::Int as i64);
    typy_rt_dict_new(int, 0, int, 0, word(0))
}

/// Calls `act` with the `int` word of `value`, then drops the word's own reference.
fn with_int<T>(value: i128, act: impl FnOnce(u64) -> T) -> T {
    let item = Integer::from_i128(value).into_word();
    let result = act(item.cast_unsigned());
    // SAFETY: the word is live and this function holds its reference.
    unsafe { typy_rt_int_decrement(item) };
    result
}

fn equal_int(word: u64, value: i128) -> bool {
    let expected = Integer::from_i128(value).into_word();
    // SAFETY: both words are live.
    let equal =
        unsafe { crate::int::typy_rt_int_compare(word.cast_signed(), expected, self::word(0)) };
    // SAFETY: the test holds the reference of `expected`.
    unsafe { typy_rt_int_decrement(expected) };
    equal
}

fn set(dict: *mut DictObject, key: i128, value: i128) {
    with_int(key, |key| {
        // SAFETY: the dict is live and both are live `int` words.
        with_int(value, |value| unsafe { typy_rt_dict_set(dict, key, value) });
    });
}

fn release<T>(object: *mut T) {
    // SAFETY: the object is live and the test holds a reference.
    unsafe { typy_rt_decrement(object.cast()) };
}

fn message(exception: *mut ExceptionObject) -> String {
    // SAFETY: the exception is live; the test owns it.
    unsafe {
        let value = text((*exception).message);
        typy_rt_decrement(exception.cast());
        value
    }
}

/// Runs one scripted step on the two dicts.
///
/// # Safety
///
/// Both dicts are live dicts of `int` keys and values.
unsafe fn run(dicts: &mut [*mut DictObject; 2], step: Step) {
    // SAFETY: the caller passes live dicts; every key and value is a live `int` word.
    unsafe {
        match step {
            Step::Set(at, key, value) => set(dicts[at], key, value),
            Step::Pop(at, key, expected) => {
                let popped = with_int(key, |key| typy_rt_dict_pop(dicts[at], key));
                match expected {
                    Some(value) => {
                        assert!(popped.exception.is_null());
                        assert!(equal_int(popped.value, value));
                        typy_rt_int_decrement(popped.value.cast_signed());
                    }
                    None => assert!(message(popped.exception).contains(&key.to_string())),
                }
            }
            Step::PopItem(at, expected) => {
                let popped = typy_rt_dict_popitem(dicts[at]);
                match expected {
                    Some((key, value)) => {
                        assert!(popped.exception.is_null());
                        let found = typy_rt_dict_key(dicts[at], popped.value);
                        let held = typy_rt_dict_value(dicts[at], popped.value);
                        assert!(equal_int(found, key) && equal_int(held, value));
                        typy_rt_int_decrement(found.cast_signed());
                        typy_rt_int_decrement(held.cast_signed());
                    }
                    None => assert_eq!(
                        message(popped.exception),
                        "'popitem(): dictionary is empty'"
                    ),
                }
            }
            Step::SetDefault(at, key, default, expected) => {
                let got = with_int(key, |key| {
                    with_int(default, |default| {
                        typy_rt_dict_setdefault(dicts[at], key, default)
                    })
                });
                assert!(equal_int(got, expected));
            }
            Step::Update(at) => {
                typy_rt_dict_update(dicts[at], dicts[1 - at].expose_provenance() as u64);
            }
            Step::Copy(at) => {
                let copy = typy_rt_dict_copy(dicts[at]);
                release(dicts[at]);
                dicts[at] = copy;
            }
            Step::Clear(at) => typy_rt_dict_clear(dicts[at]),
            Step::Check(at, expected, equal) => {
                assert_eq!(repr(dicts[at]), expected);
                assert_eq!(typy_rt_dict_equal(dicts[0], dicts[1]), equal);
            }
        }
    }
}

#[test]
#[cfg_attr(miri, ignore = "hundreds of steps on growing tables take Miri minutes")]
fn every_operation_gives_cpythons_results() {
    let mut dicts = [int_dict(), int_dict()];
    for (number, step) in steps::STEPS.iter().enumerate() {
        // SAFETY: both dicts stay live through the steps.
        let outcome = std::panic::catch_unwind(core::panic::AssertUnwindSafe(|| unsafe {
            run(&mut dicts, *step);
        }));
        assert!(outcome.is_ok(), "step {number} failed");
    }
    dicts.into_iter().for_each(release);
}

#[test]
fn keys_keep_their_place_and_lookups_find_them() {
    let dict = int_dict();
    for (key, value) in [(3, 30), (1, 10), (2, 20), (1, 11), (1 << 70, 7)] {
        set(dict, key, value);
    }
    assert_eq!(
        repr(dict),
        "{3: 30, 1: 11, 2: 20, 1180591620717411303424: 7}"
    );
    // SAFETY: the dict is live, and every key is a live `int` word.
    unsafe {
        assert_eq!(inline_value(typy_rt_dict_length(dict)), 4);
        assert!(with_int(2, |key| typy_rt_dict_contains(dict, key)));
        assert_eq!(
            inline_value(with_int(9, |key| typy_rt_dict_find(dict, key))),
            -1
        );
        let found = with_int(1 << 70, |key| typy_rt_dict_get(dict, key));
        assert!(equal_int(found.value, 7));
        assert!(equal_int(
            with_int(2, |key| typy_rt_dict_get_or(
                dict,
                key,
                word(0).cast_unsigned()
            )),
            20
        ));
        assert!(equal_int(
            with_int(9, |key| typy_rt_dict_get_or(
                dict,
                key,
                word(0).cast_unsigned()
            )),
            0
        ));
        let missing = with_int(9, |key| typy_rt_dict_get(dict, key));
        assert_eq!(message(missing.exception), "9");
        let kept = with_int(5, |key| {
            with_int(50, |default| typy_rt_dict_pop_or(dict, key, default))
        });
        assert!(equal_int(kept, 50));
        let taken = with_int(3, |key| {
            with_int(50, |default| typy_rt_dict_pop_or(dict, key, default))
        });
        assert!(equal_int(taken, 30));
        assert!(typy_rt_dict_has_value(dict, word(20).cast_unsigned()));
        assert!(!typy_rt_dict_has_value(dict, word(30).cast_unsigned()));
        assert!(typy_rt_dict_has_item(
            dict,
            word(2).cast_unsigned(),
            word(20).cast_unsigned()
        ));
        assert!(!typy_rt_dict_has_item(
            dict,
            word(2).cast_unsigned(),
            word(21).cast_unsigned()
        ));
        assert!(!typy_rt_dict_has_item(
            dict,
            word(8).cast_unsigned(),
            word(20).cast_unsigned()
        ));
    }
    release(dict);
}

#[test]
fn views_are_written_as_cpython_writes_them() {
    let dict = int_dict();
    set(dict, 1, 2);
    set(dict, 3, 4);
    // SAFETY: the dict is live.
    unsafe {
        assert_eq!(
            shown(typy_rt_dict_view_repr(dict, word(0))),
            "dict_keys([1, 3])"
        );
        assert_eq!(
            shown(typy_rt_dict_view_repr(dict, word(1))),
            "dict_values([2, 4])"
        );
        assert_eq!(
            shown(typy_rt_dict_view_repr(dict, word(2))),
            "dict_items([(1, 2), (3, 4)])"
        );
        typy_rt_dict_clear(dict);
        assert_eq!(repr(dict), "{}");
        assert_eq!(
            shown(typy_rt_dict_view_repr(dict, word(0))),
            "dict_keys([])"
        );
    }
    release(dict);
}

#[test]
fn text_of_a_huge_key_raises_the_repr_error() {
    let dict = int_dict();
    // SAFETY: the dict is live; `10 ** 5000` has more digits than `repr` writes.
    unsafe {
        let large = crate::int::typy_rt_int_power(word(10), word(5000)).value;
        typy_rt_dict_set(dict, word(1).cast_unsigned(), large.cast_unsigned());
        typy_rt_int_decrement(large);
        let result = typy_rt_dict_repr(dict);
        assert!(!result.exception.is_null());
        typy_rt_decrement(result.exception.cast());
        let view = typy_rt_dict_view_repr(dict, word(2));
        assert!(!view.exception.is_null());
        typy_rt_decrement(view.exception.cast());
    }
    release(dict);
}

#[test]
fn iteration_raises_when_the_size_or_the_keys_change() {
    let dict = int_dict();
    set(dict, 1, 1);
    set(dict, 2, 2);
    // SAFETY: the dict is live; each position came from `typy_rt_dict_next`.
    unsafe {
        let first = typy_rt_dict_next(dict, word(0), word(2), word(0));
        assert_eq!(inline_value(first.value), 0);
        let changed = typy_rt_dict_next(dict, word(1), word(3), word(1));
        assert_eq!(
            message(changed.exception),
            "dictionary changed size during iteration"
        );
        let keys = typy_rt_dict_next(dict, word(1), word(2), word(2));
        assert_eq!(
            message(keys.exception),
            "dictionary keys changed during iteration"
        );
        let end = typy_rt_dict_next(dict, word(2), word(2), word(2));
        assert_eq!(inline_value(end.value), -1);
    }
    release(dict);
}

#[test]
fn a_nan_key_is_never_found_again() {
    let float = word(ItemKind::Float as i64);
    let dict = typy_rt_dict_new(float, 0, float, 0, word(0));
    // SAFETY: the dict is live; a float needs no reference.
    unsafe {
        let nan = f64::NAN.to_bits();
        typy_rt_dict_set(dict, nan, 1.0_f64.to_bits());
        typy_rt_dict_set(dict, nan, 2.0_f64.to_bits());
        typy_rt_dict_set(dict, (-0.0_f64).to_bits(), 3.0_f64.to_bits());
        typy_rt_dict_set(dict, 0.0_f64.to_bits(), 4.0_f64.to_bits());
        assert_eq!(repr(dict), "{nan: 1.0, nan: 2.0, -0.0: 4.0}");
        let popped = typy_rt_dict_popitem(dict);
        assert_eq!(inline_value(popped.value), 2);
        let popped = typy_rt_dict_popitem(dict);
        assert_eq!(inline_value(popped.value), 1);
        assert_eq!(repr(dict), "{nan: 1.0}");
        assert!(!typy_rt_dict_contains(dict, nan));
    }
    release(dict);
}

fn field(kind: ItemKind, offset: u32) -> Field {
    Field {
        kind: kind as u32,
        offset,
        tuple: core::ptr::null(),
    }
}

#[test]
fn a_list_of_pairs_updates_in_order_and_strings_keep_their_references() {
    let fields = Box::leak(Box::new([
        field(ItemKind::Object, 0),
        field(ItemKind::Int, 8),
    ]));
    let pair = Box::leak(Box::new(TupleType {
        size: 16,
        length: 2,
        fields: fields.as_ptr(),
    }));
    let pairs = crate::list::new_list(crate::list::ItemType::tuple(pair), 0);
    let (a, b) = (from_str("a"), from_str("b"));
    // SAFETY: the list, the strings and the dict are live.
    unsafe {
        for (name, value) in [(a, 1), (b, 2), (a, 3)] {
            let words = [name.expose_provenance() as u64, word(value).cast_unsigned()];
            crate::list::typy_rt_list_append(pairs, words.as_ptr().expose_provenance() as u64);
        }
        let int = word(ItemKind::Int as i64);
        let dict = typy_rt_dict_new(word(ItemKind::Object as i64), 0, int, 0, word(4));
        typy_rt_dict_update(dict, pairs.expose_provenance() as u64);
        assert_eq!(repr(dict), "{'a': 3, 'b': 2}");
        let copy = typy_rt_dict_copy(dict);
        assert!(typy_rt_dict_equal(dict, copy));
        release(copy);
        typy_rt_decrement(pairs.cast());
        assert_eq!((*a).header.reference_count, 2);
        release(dict);
        assert_eq!((*a).header.reference_count, 1);
        release(a);
        release(b);
    }
}

#[test]
fn a_copy_of_a_dict_with_many_holes_is_compacted() {
    let dict = int_dict();
    for key in 0..20 {
        set(dict, key, key);
    }
    // SAFETY: the dict is live and every key is a live `int` word.
    unsafe {
        for key in 0..15 {
            let popped = with_int(key, |key| typy_rt_dict_pop(dict, key));
            assert!(popped.exception.is_null() && equal_int(popped.value, key));
        }
        let copy = typy_rt_dict_copy(dict);
        assert_eq!((*copy).filled, 5);
        assert_eq!(repr(copy), "{15: 15, 16: 16, 17: 17, 18: 18, 19: 19}");
        release(copy);
        let empty = int_dict();
        let copied = typy_rt_dict_copy(empty);
        assert_eq!(repr(copied), "{}");
        typy_rt_dict_update(empty, dict.expose_provenance() as u64);
        typy_rt_dict_update(dict, dict.expose_provenance() as u64);
        assert_eq!(repr(empty), "{15: 15, 16: 16, 17: 17, 18: 18, 19: 19}");
        [empty, copied].into_iter().for_each(release);
    }
    release(dict);
}

#[test]
fn a_view_reads_its_dict_as_it_changes_and_keeps_it_alive() {
    use super::view::{
        typy_rt_dict_view, typy_rt_dict_view_contains, typy_rt_dict_view_dict,
        typy_rt_dict_view_length,
    };
    let fields = Box::leak(Box::new([field(ItemKind::Int, 0), field(ItemKind::Int, 8)]));
    let pair = Box::leak(Box::new(TupleType {
        size: 16,
        length: 2,
        fields: fields.as_ptr(),
    }));
    let pair = core::ptr::from_ref::<TupleType>(pair).expose_provenance() as u64;
    let dict = int_dict();
    set(dict, 1, 10);
    // SAFETY: the dict and the views are live; each pair is two inline `int` words.
    unsafe {
        let keys = typy_rt_dict_view(dict, word(0));
        let values = typy_rt_dict_view(dict, word(1));
        let items = typy_rt_dict_view(dict, word(2));
        set(dict, 2, 20);
        assert_eq!(
            shown(super::text::typy_rt_dict_view_text(items)),
            "dict_items([(1, 10), (2, 20)])"
        );
        assert_eq!(inline_value(typy_rt_dict_view_length(keys)), 2);
        assert_eq!(typy_rt_dict_view_dict(values), dict);
        assert!(typy_rt_dict_view_contains(keys, word(2).cast_unsigned(), 0));
        assert!(typy_rt_dict_view_contains(
            values,
            word(20).cast_unsigned(),
            0
        ));
        assert!(!typy_rt_dict_view_contains(
            values,
            word(2).cast_unsigned(),
            0
        ));
        let found = [word(2).cast_unsigned(), word(20).cast_unsigned()];
        let wrong = [word(2).cast_unsigned(), word(21).cast_unsigned()];
        let address = |words: &[u64; 2]| words.as_ptr().expose_provenance() as u64;
        assert!(typy_rt_dict_view_contains(items, address(&found), pair));
        assert!(!typy_rt_dict_view_contains(items, address(&wrong), pair));
        release(dict);
        assert_eq!((*dict).header.reference_count, 3);
        for view in [keys, values, items] {
            release(view);
        }
    }
}

#[test]
fn a_merge_clones_a_full_source_or_resizes_the_target_first() {
    let source = int_dict();
    for key in 0..20 {
        set(source, key, key * 10);
    }
    let (empty, small) = (int_dict(), int_dict());
    set(small, 100, 1);
    // SAFETY: the dicts are live.
    unsafe {
        typy_rt_dict_update(empty, source.expose_provenance() as u64);
        assert_eq!((*empty).log2, (*source).log2);
        assert_eq!((*empty).filled, 20);
        assert!(typy_rt_dict_equal(empty, source));
        typy_rt_dict_update(small, source.expose_provenance() as u64);
        assert_eq!(inline_value(typy_rt_dict_length(small)), 21);
        assert!(repr(small).starts_with("{100: 1, 0: 0, 1: 10"));
        let reread = with_int(19, |key| typy_rt_dict_find(small, key));
        assert_eq!(
            inline_value(typy_rt_dict_value(small, reread).cast_signed()),
            190
        );
    }
    [source, empty, small].into_iter().for_each(release);
}

#[test]
fn a_sparse_source_is_merged_entry_by_entry() {
    let int = word(ItemKind::Int as i64);
    let source = typy_rt_dict_new(int, 0, int, 0, word(40));
    for key in 0..5 {
        set(source, key, key);
    }
    let target = int_dict();
    // SAFETY: the dicts are live.
    unsafe {
        typy_rt_dict_update(target, source.expose_provenance() as u64);
        assert!((*target).log2 < (*source).log2);
        assert!(typy_rt_dict_equal(target, source));
    }
    [source, target].into_iter().for_each(release);
}
