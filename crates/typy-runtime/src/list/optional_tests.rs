//! Tests of optional items: `None` is written, compared, hashed and tested as CPython does.

use super::item::NONE_FLOAT;
use super::{ItemKind, ItemType, ListObject, new_list, typy_rt_list_append, typy_rt_list_repr};
use crate::string::{code_point, from_str, length};
use crate::tuple::{Field, TupleType};
use crate::word::inline;

fn word(value: i64) -> u64 {
    inline(value).unwrap().cast_unsigned()
}

fn holding(item_type: ItemType, items: &[u64]) -> *mut ListObject {
    let list = new_list(item_type, 0);
    for item in items {
        // SAFETY: the list is live and each item is a live item of its type.
        unsafe { typy_rt_list_append(list, *item) };
    }
    list
}

fn shown(list: *const ListObject) -> String {
    // SAFETY: the list is live; its text is a new string.
    unsafe {
        let text = typy_rt_list_repr(list).value;
        let shown = (0..length(text))
            .map(|index| char::from_u32(code_point(text, index)).unwrap())
            .collect();
        crate::refcount::typy_rt_decrement(text.cast());
        shown
    }
}

fn release(list: *mut ListObject) {
    // SAFETY: the list is live and the test holds its reference.
    unsafe { crate::refcount::typy_rt_decrement(list.cast()) };
}

#[test]
fn optional_items_write_compare_hash_and_test_none() {
    let ints = ItemType::from(ItemKind::OptionalInt);
    let floats = ItemType::from(ItemKind::OptionalFloat);
    let bools = ItemType::from(ItemKind::OptionalBool);
    let objects = ItemType::from(ItemKind::OptionalObject);
    let name = from_str("a");
    let name_word = name.expose_provenance() as u64;
    let cases = [
        (ints, vec![word(1), 0, word(-2)], "[1, None, -2]"),
        (floats, vec![1.5_f64.to_bits(), NONE_FLOAT], "[1.5, None]"),
        (bools, vec![1, 2, 0], "[True, None, False]"),
        (objects, vec![name_word, 0], "['a', None]"),
    ];
    for (item_type, items, text) in cases {
        let list = holding(item_type, &items);
        assert_eq!(shown(list), text);
        release(list);
        // SAFETY: each item is a live item of its type.
        unsafe {
            let (value, none) = (items[0], items[1]);
            assert!(item_type.equal(none, none));
            assert!(!item_type.equal(value, none) && !item_type.equal(none, value));
            assert!(item_type.equal(value, value));
            assert_eq!(item_type.hash(none), crate::hash::NONE);
            assert!(!item_type.is_true(none));
            assert!(item_type.is_true(value));
            assert!(item_type.order(none, none, inline(3).unwrap()));
            assert!(item_type.order(value, value, inline(3).unwrap()));
            assert_eq!(item_type.hash(value), item_type.base().unwrap().hash(value));
        }
    }
    // SAFETY: the string's own reference is dropped once the lists are gone.
    unsafe { crate::refcount::typy_rt_decrement(name.cast()) };
}

#[test]
fn an_optional_tuple_is_a_flag_and_a_tuple() {
    let field = |kind: ItemKind, offset: u32, tuple: *const TupleType| Field {
        kind: kind as u32,
        offset,
        tuple,
    };
    let inner_fields = Box::leak(Box::new([field(ItemKind::Int, 0, core::ptr::null())]));
    let inner = Box::leak(Box::new(TupleType {
        size: 8,
        length: 1,
        fields: inner_fields.as_ptr(),
    }));
    let pair_fields = Box::leak(Box::new([
        field(ItemKind::Bool, 0, core::ptr::null()),
        field(ItemKind::Tuple, 8, inner),
    ]));
    let pair = Box::leak(Box::new(TupleType {
        size: 16,
        length: 2,
        fields: pair_fields.as_ptr(),
    }));
    let item_type = ItemType::of(ItemKind::OptionalTuple as u32, pair);
    let present = [1, word(7)];
    let absent = [0, 0];
    let address = |words: &[u64; 2]| words.as_ptr().expose_provenance() as u64;
    let list = holding(item_type, &[address(&present), address(&absent)]);
    assert_eq!(shown(list), "[(7,), None]");
    release(list);
    // SAFETY: both pairs are live.
    unsafe {
        assert!(!item_type.equal(address(&present), address(&absent)));
        assert!(item_type.is_true(address(&present)));
    }
}
