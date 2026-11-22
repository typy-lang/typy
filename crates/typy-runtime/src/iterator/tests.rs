//! Tests of the iterators `iter()` makes: their items, their ends and their sources' rules.

use super::{
    IteratorObject, typy_rt_iterator_advance, typy_rt_iterator_item, typy_rt_iterator_new,
};
use crate::header::ObjectHeader;
use crate::list::{ItemKind, ListObject, typy_rt_list_append, typy_rt_list_new};
use crate::refcount::typy_rt_decrement;
use crate::string::{StrObject, code_point, from_str, length};
use crate::tuple::{Field, TupleType};
use crate::word::{inline, inline_value};

/// Returns the `int` word of `value`.
fn word(value: i64) -> i64 {
    inline(value).unwrap()
}

/// Returns a new iterator of `kind` over `source`.
fn iterator(source: *mut ObjectHeader, kind: i64, pair: *const TupleType) -> *mut IteratorObject {
    // SAFETY: the tests pass a live source of the kind's type.
    unsafe { typy_rt_iterator_new(source, word(kind), pair) }
}

/// Steps an iterator and returns its item, or `None` at its end.
fn next(iterator: *mut IteratorObject) -> Option<u64> {
    // SAFETY: the iterator is live.
    unsafe {
        let stepped = typy_rt_iterator_advance(iterator, word(0));
        assert!(stepped.exception.is_null());
        stepped.value.then(|| typy_rt_iterator_item(iterator))
    }
}

/// Returns the message of the exception a step raises, and releases it.
fn failure(iterator: *mut IteratorObject, stop: i64) -> String {
    // SAFETY: the iterator is live, and the exception is owned here.
    unsafe {
        let stepped = typy_rt_iterator_advance(iterator, word(stop));
        let exception = stepped.exception;
        assert!(!exception.is_null());
        let text = crate::exception::classes::BASE_EXCEPTION.repr.unwrap()(exception.cast());
        let shown = text_of(text.value);
        typy_rt_decrement(text.value.cast());
        typy_rt_decrement(exception.cast());
        shown
    }
}

/// Returns the text of a `str`.
fn text_of(text: *const StrObject) -> String {
    // SAFETY: the string is live.
    unsafe {
        (0..length(text))
            .map(|index| char::from_u32(code_point(text, index)).unwrap())
            .collect()
    }
}

/// Returns the name of an iterator's type.
fn type_name(iterator: *const IteratorObject) -> &'static str {
    // SAFETY: the iterator is live.
    unsafe { super::type_of(iterator) }.name()
}

/// Releases an object the test holds a reference to.
fn release<T>(object: *mut T) {
    // SAFETY: the object is live and the test holds a reference.
    unsafe { typy_rt_decrement(object.cast()) };
}

/// Returns a new list of small `int` values.
fn int_list(values: &[i64]) -> *mut ListObject {
    let list = typy_rt_list_new(word(ItemKind::Int as i64), word(0), 0);
    for value in values {
        // SAFETY: the list is live and the items are inline words.
        unsafe { typy_rt_list_append(list, word(*value).cast_unsigned()) };
    }
    list
}

#[test]
fn a_list_iterator_sees_items_appended_until_it_ends_and_then_drops_its_list() {
    let list = int_list(&[1, 2]);
    let items = iterator(list.cast(), 0, core::ptr::null());
    assert_eq!(type_name(items), "list_iterator");
    assert_eq!(next(items), Some(word(1).cast_unsigned()));
    // SAFETY: the list is live.
    unsafe { typy_rt_list_append(list, word(3).cast_unsigned()) };
    assert_eq!(next(items), Some(word(2).cast_unsigned()));
    assert_eq!(next(items), Some(word(3).cast_unsigned()));
    assert_eq!(next(items), None);
    // SAFETY: the list is live; the iterator dropped its reference when it ended.
    unsafe {
        assert_eq!((*list).header.reference_count, 1);
        typy_rt_list_append(list, word(4).cast_unsigned());
    }
    assert_eq!(next(items), None);
    release(items);
    release(list);
}

#[test]
fn an_exhausted_iterator_raises_stop_iteration_with_no_arguments_when_asked() {
    let list = int_list(&[]);
    let items = iterator(list.cast(), 1, core::ptr::null());
    assert_eq!(type_name(items), "tuple_iterator");
    assert_eq!(failure(items, 1), "StopIteration()");
    release(items);
    release(list);
}

#[test]
fn a_str_iterator_gives_strings_of_one_code_point_and_names_its_type_as_cpython() {
    let cases: [(&str, &str, &[&str]); 2] = [
        ("ab", "str_ascii_iterator", &["a", "b"]),
        ("a\u{e9}", "str_iterator", &["a", "\u{e9}"]),
    ];
    for (text, name, expected) in cases {
        let source = from_str(text);
        let characters = iterator(source.cast(), 2, core::ptr::null());
        assert_eq!(type_name(characters), name);
        let mut seen = Vec::new();
        while let Some(item) = next(characters) {
            let character =
                core::ptr::with_exposed_provenance::<StrObject>(usize::try_from(item).unwrap());
            seen.push(text_of(character));
        }
        assert_eq!(seen, expected);
        release(characters);
        release(source);
    }
}

#[test]
fn a_bytes_iterator_gives_int_values() {
    let data = crate::bytes::new_bytes(2);
    // SAFETY: the bytes are new and hold two bytes.
    unsafe {
        crate::string::set_code_point(data, 0, 7);
        crate::string::set_code_point(data, 1, 255);
    }
    let bytes = iterator(data.cast(), 3, core::ptr::null());
    assert_eq!(type_name(bytes), "bytes_iterator");
    let values: Vec<i64> = core::iter::from_fn(|| next(bytes))
        .map(|item| inline_value(item.cast_signed()))
        .collect();
    assert_eq!(values, [7, 255]);
    release(bytes);
    release(data);
}

#[test]
fn a_set_iterator_raises_when_the_set_changes_size() {
    let set = crate::set::typy_rt_set_new(word(ItemKind::Int as i64), 0);
    // SAFETY: the set is live and the items are inline words.
    unsafe {
        crate::set::typy_rt_set_add(set, word(1).cast_unsigned());
        crate::set::typy_rt_set_add(set, word(2).cast_unsigned());
    }
    let items = iterator(set.cast(), 4, core::ptr::null());
    assert_eq!(type_name(items), "set_iterator");
    assert!(next(items).is_some());
    // SAFETY: the set is live.
    unsafe { crate::set::typy_rt_set_add(set, word(3).cast_unsigned()) };
    assert_eq!(
        failure(items, 0),
        "RuntimeError('Set changed size during iteration')"
    );
    release(items);
    release(set);
}

static PAIR_FIELDS: [Field; 2] = [
    Field::plain(ItemKind::Int, 0),
    Field::plain(ItemKind::Int, 8),
];
static PAIR: TupleType = TupleType {
    size: 16,
    length: 2,
    fields: PAIR_FIELDS.as_ptr(),
};

#[test]
fn a_dict_iterator_gives_keys_values_or_items_and_raises_when_the_dict_changes_size() {
    let int = word(ItemKind::Int as i64);
    let dict = crate::dict::typy_rt_dict_new(int, 0, int, 0, word(0));
    // SAFETY: the dict is live and the keys and values are inline words.
    unsafe {
        crate::dict::typy_rt_dict_set(dict, word(1).cast_unsigned(), word(10).cast_unsigned());
        crate::dict::typy_rt_dict_set(dict, word(2).cast_unsigned(), word(20).cast_unsigned());
    }
    let read = |kind: i64, pair: *const TupleType| {
        let items = iterator(dict.cast(), kind, pair);
        let name = type_name(items);
        let found: Vec<i64> = core::iter::from_fn(|| next(items))
            .flat_map(|item| {
                if pair.is_null() {
                    vec![inline_value(item.cast_signed())]
                } else {
                    // SAFETY: the pair holds two `int` words.
                    unsafe {
                        (0..2)
                            .map(|index| {
                                inline_value(crate::tuple::field(pair, item, index).cast_signed())
                            })
                            .collect()
                    }
                }
            })
            .collect();
        release(items);
        (name, found)
    };
    assert_eq!(read(5, core::ptr::null()), ("dict_keyiterator", vec![1, 2]));
    assert_eq!(
        read(6, core::ptr::null()),
        ("dict_valueiterator", vec![10, 20])
    );
    assert_eq!(
        read(7, &raw const PAIR),
        ("dict_itemiterator", vec![1, 10, 2, 20])
    );
    let keys = iterator(dict.cast(), 5, core::ptr::null());
    assert!(next(keys).is_some());
    // SAFETY: the dict is live.
    unsafe {
        crate::dict::typy_rt_dict_set(dict, word(3).cast_unsigned(), word(30).cast_unsigned());
    };
    assert_eq!(
        failure(keys, 0),
        "RuntimeError('dictionary changed size during iteration')"
    );
    release(keys);
    release(dict);
}

#[test]
fn an_iterator_is_written_with_its_type_and_address() {
    let list = int_list(&[]);
    let items = iterator(list.cast(), 0, core::ptr::null());
    // SAFETY: the iterator is live.
    let text = unsafe { super::iterator_repr(items.cast()) };
    let shown = text_of(text.value);
    assert!(shown.starts_with("<list_iterator object at 0x"), "{shown}");
    release(text.value);
    release(items);
    release(list);
}

/// Returns the range from `first` up to `end` by `step`, of `int` words.
fn range(first: i64, end: i64, step: i64) -> *mut ObjectHeader {
    // SAFETY: the words are live.
    let made = unsafe { crate::range::object::typy_rt_range_new(first, end, step) };
    assert!(made.exception.is_null());
    made.value.cast()
}

#[test]
fn a_range_iterator_gives_each_value_and_names_its_type_as_cpython() {
    let source = range(word(10), word(0), word(-4));
    let values = iterator(source, 9, core::ptr::null());
    release(source);
    assert_eq!(type_name(values), "range_iterator");
    let items: Vec<i64> = core::iter::from_fn(|| next(values))
        .map(|item| inline_value(item.cast_signed()))
        .collect();
    assert_eq!(items, [10, 6, 2]);
    assert_eq!(next(values), None);
    release(values);
}

#[test]
#[allow(
    clippy::similar_names,
    reason = "start and stop are a range's own names"
)]
fn a_range_past_a_c_long_gets_a_longrange_iterator_that_owns_its_values() {
    let start = crate::int::value::Integer::from_i128(1 << 70).into_word();
    let stop = crate::int::value::Integer::from_i128((1 << 70) + 4).into_word();
    let source = range(start, stop, word(3));
    // SAFETY: the range holds references of its own to the bounds.
    unsafe {
        crate::refcount::typy_rt_int_decrement(start);
        crate::refcount::typy_rt_int_decrement(stop);
    }
    let values = iterator(source, 9, core::ptr::null());
    release(source);
    assert_eq!(type_name(values), "longrange_iterator");
    let first = next(values).unwrap().cast_signed();
    let second = next(values).unwrap().cast_signed();
    // SAFETY: each item is live until the next step.
    unsafe {
        let expected = crate::int::value::Integer::from_i128((1 << 70) + 3).into_word();
        assert!(crate::int::typy_rt_int_compare(second, expected, word(0)));
        crate::refcount::typy_rt_int_decrement(expected);
    }
    assert!(!crate::word::is_inline(first));
    assert_eq!(next(values), None);
    release(values);
}

#[test]
fn a_range_of_small_bounds_past_a_c_long_in_length_is_a_longrange() {
    let low = crate::int::value::Integer::from_i128(i128::from(i64::MIN)).into_word();
    let high = crate::int::value::Integer::from_i128(i128::from(i64::MAX)).into_word();
    let source = range(low, high, word(1));
    // SAFETY: the range holds references of its own to the bounds.
    unsafe {
        crate::refcount::typy_rt_int_decrement(low);
        crate::refcount::typy_rt_int_decrement(high);
    }
    let values = iterator(source, 9, core::ptr::null());
    release(source);
    assert_eq!(type_name(values), "longrange_iterator");
    release(values);
}

#[test]
fn a_range_that_starts_below_a_c_long_is_a_longrange() {
    let start = crate::int::value::Integer::from_i128(-(1 << 70)).into_word();
    let source = range(start, word(0), word(1));
    // SAFETY: the range holds a reference of its own to the start.
    unsafe { crate::refcount::typy_rt_int_decrement(start) };
    let values = iterator(source, 9, core::ptr::null());
    release(source);
    assert_eq!(type_name(values), "longrange_iterator");
    release(values);
}

#[test]
fn an_iterable_gives_the_iterator_its_header_calls_for() {
    use super::typy_rt_iterable_iterator;
    let list = int_list(&[1, 2]);
    let text = from_str("ab");
    let numbers = range(word(0), word(3), word(1));
    // SAFETY: every source is live, and the views and iterators are released here.
    unsafe {
        let null = core::ptr::null();
        let cases: [(*mut ObjectHeader, &str); 3] = [
            (list.cast(), "list_iterator"),
            (text.cast(), "str_ascii_iterator"),
            (numbers, "range_iterator"),
        ];
        for (source, name) in cases {
            let made = typy_rt_iterable_iterator(source, null).value;
            assert_eq!(type_name(made), name);
            let again = typy_rt_iterable_iterator(made.cast(), null).value;
            assert_eq!(again, made, "an iterator gives itself");
            release(again);
            release(made);
        }
    }
    release(list);
    release(text);
    release(numbers);
}

/// Returns the items a reversed iterator of `source` gives, as `int` values, and releases it.
fn reversed_ints(source: *mut ObjectHeader, pair: *const TupleType) -> Vec<i64> {
    // SAFETY: the tests pass a live source of a type `reversed` reads.
    let items = unsafe { super::reversed::typy_rt_iterator_reversed(source, pair) };
    let found = core::iter::from_fn(|| next(items))
        .flat_map(|item| {
            if pair.is_null() {
                vec![inline_value(item.cast_signed())]
            } else {
                // SAFETY: the pair holds two `int` words.
                unsafe {
                    (0..2)
                        .map(|index| {
                            inline_value(crate::tuple::field(pair, item, index).cast_signed())
                        })
                        .collect()
                }
            }
        })
        .collect();
    release(items);
    found
}

#[test]
fn lib_20_reversed_reads_a_list_a_tuple_a_range_and_bytes_from_the_last_item() {
    let list = int_list(&[1, 2, 3]);
    assert_eq!(reversed_ints(list.cast(), core::ptr::null()), [3, 2, 1]);
    let tuple = int_list(&[4, 5]);
    // SAFETY: the list is new and sealed once.
    unsafe { crate::list::typy_rt_tuple_seal(tuple) };
    assert_eq!(reversed_ints(tuple.cast(), core::ptr::null()), [5, 4]);
    let source = range(word(0), word(7), word(3));
    assert_eq!(reversed_ints(source, core::ptr::null()), [6, 3, 0]);
    let empty = range(word(0), word(0), word(1));
    assert_eq!(reversed_ints(empty, core::ptr::null()), Vec::<i64>::new());
    let data = crate::bytes::new_bytes(2);
    // SAFETY: the bytes are new and hold two bytes.
    unsafe {
        crate::string::set_code_point(data, 0, 7);
        crate::string::set_code_point(data, 1, 9);
    }
    assert_eq!(reversed_ints(data.cast(), core::ptr::null()), [9, 7]);
    for each in [list, tuple] {
        release(each);
    }
    for each in [source, empty, data.cast()] {
        release(each);
    }
}

#[test]
fn lib_20_reversed_reads_a_str_as_strings_of_one_code_point() {
    let text = from_str("ab");
    // SAFETY: the source is a live `str`.
    let items =
        unsafe { super::reversed::typy_rt_iterator_reversed(text.cast(), core::ptr::null()) };
    let found: Vec<String> = core::iter::from_fn(|| next(items))
        .map(|item| {
            let item =
                core::ptr::with_exposed_provenance::<StrObject>(usize::try_from(item).unwrap());
            text_of(item)
        })
        .collect();
    assert_eq!(found, ["b", "a"]);
    release(items);
    release(text);
}

#[test]
fn lib_20_reversed_reads_a_dict_and_its_views_and_raises_when_it_changes_size() {
    let int = word(ItemKind::Int as i64);
    let dict = crate::dict::typy_rt_dict_new(int, 0, int, 0, word(0));
    // SAFETY: the dict is live and the keys and values are inline words.
    unsafe {
        crate::dict::typy_rt_dict_set(dict, word(1).cast_unsigned(), word(10).cast_unsigned());
        crate::dict::typy_rt_dict_set(dict, word(2).cast_unsigned(), word(20).cast_unsigned());
    }
    assert_eq!(reversed_ints(dict.cast(), core::ptr::null()), [2, 1]);
    for (kind, pair, expected) in [
        (0, core::ptr::null(), vec![2, 1]),
        (1, core::ptr::null(), vec![20, 10]),
        (2, &raw const PAIR, vec![2, 20, 1, 10]),
    ] {
        // SAFETY: the dict is live.
        let view = unsafe { crate::dict::typy_rt_dict_view(dict, word(kind)) };
        assert_eq!(reversed_ints(view.cast(), pair), expected, "view {kind}");
        release(view);
    }
    // SAFETY: the dict is live.
    let keys =
        unsafe { super::reversed::typy_rt_iterator_reversed(dict.cast(), core::ptr::null()) };
    assert!(next(keys).is_some());
    // SAFETY: the dict is live.
    unsafe {
        crate::dict::typy_rt_dict_set(dict, word(3).cast_unsigned(), word(30).cast_unsigned());
    };
    assert_eq!(
        failure(keys, 0),
        "RuntimeError('dictionary changed size during iteration')"
    );
    release(keys);
    release(dict);
}

/// Returns a new list of lists of small `int` values, the iterables of a zip.
fn int_lists(lists: &[&[i64]]) -> *mut ListObject {
    let outer = typy_rt_list_new(word(ItemKind::Object as i64), word(0), 0);
    for values in lists {
        let inner = int_list(values);
        // SAFETY: the outer list is live and takes its own reference to the inner one.
        unsafe { typy_rt_list_append(outer, inner.expose_provenance() as u64) };
        release(inner);
    }
    outer
}

/// Returns a zip of `lists` and the `int` values of each of its tuples, up to its end or the
/// message of the exception a step raises.
fn zipped(lists: &[&[i64]], strict: bool) -> (Vec<Vec<i64>>, Option<String>) {
    let iterables = int_lists(lists);
    // SAFETY: the list holds live iterables, and the items are ints.
    let zip = unsafe {
        super::zip_many::typy_rt_zip_many(iterables, word(ItemKind::Int as i64), 0, strict)
    };
    assert!(zip.exception.is_null());
    let zip = zip.value;
    let mut items = Vec::new();
    let error = loop {
        // SAFETY: the zip is live.
        let stepped = unsafe { typy_rt_iterator_advance(zip, word(0)) };
        if !stepped.exception.is_null() {
            // SAFETY: the exception is owned here.
            let text = unsafe {
                crate::exception::classes::BASE_EXCEPTION.repr.unwrap()(stepped.exception.cast())
            };
            let shown = text_of(text.value);
            release(text.value);
            release(stepped.exception);
            break Some(shown);
        }
        if !stepped.value {
            break None;
        }
        // SAFETY: the zip's item is a live tuple of ints.
        let tuple = unsafe {
            &*core::ptr::with_exposed_provenance::<ListObject>(
                usize::try_from(typy_rt_iterator_item(zip)).unwrap(),
            )
        };
        items.push(
            (0..tuple.length)
                // SAFETY: every position is below the tuple's length.
                .map(|index| inline_value(unsafe { tuple.item(index) }.cast_signed()))
                .collect(),
        );
    };
    release(zip);
    release(iterables);
    (items, error)
}

#[test]
fn exp_22_a_zip_of_many_gives_tuples_until_its_shortest_iterable_ends() {
    assert_eq!(
        zipped(&[&[1, 2, 3], &[4, 5, 6], &[7, 8, 9]], false),
        (vec![vec![1, 4, 7], vec![2, 5, 8], vec![3, 6, 9]], None)
    );
    assert_eq!(zipped(&[&[1, 2, 3], &[4]], false), (vec![vec![1, 4]], None));
    assert_eq!(zipped(&[], false), (Vec::new(), None));
    assert_eq!(
        zipped(&[&[1, 2], &[3, 4]], true),
        (vec![vec![1, 3], vec![2, 4]], None)
    );
}

#[test]
fn exp_22_a_strict_zip_of_many_names_the_argument_of_another_length() {
    let (items, error) = zipped(&[&[1, 2], &[3]], true);
    assert_eq!(items, vec![vec![1, 3]]);
    assert_eq!(
        error.as_deref(),
        Some("ValueError('zip() argument 2 is shorter than argument 1')")
    );
    let (items, error) = zipped(&[&[1], &[2], &[3, 4]], true);
    assert_eq!(items, vec![vec![1, 2, 3]]);
    assert_eq!(
        error.as_deref(),
        Some("ValueError('zip() argument 3 is longer than arguments 1-2')")
    );
}
