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
