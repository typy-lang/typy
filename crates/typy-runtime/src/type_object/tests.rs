//! Tests of the type objects and of the functions each gives.

use super::{
    BASE_EXCEPTION, BYTES, DICT, DICT_ITEMS, DICT_KEYS, DICT_VALUES, INT, LIST, OBJECT, SET, STR,
    TUPLE, TYPE, TypeObject, type_of,
};
use crate::header::{IMMORTAL, ObjectHeader, TYPE_TYPE_ID};

/// Returns whether two type objects are one.
fn same(left: &TypeObject, right: &TypeObject) -> bool {
    core::ptr::from_ref(left) == core::ptr::from_ref(right)
}

const EVERY_TYPE: [&TypeObject; 13] = [
    &OBJECT,
    &TYPE,
    &INT,
    &STR,
    &BYTES,
    &LIST,
    &TUPLE,
    &SET,
    &DICT,
    &DICT_KEYS,
    &DICT_VALUES,
    &DICT_ITEMS,
    &BASE_EXCEPTION,
];

#[test]
fn every_chain_of_bases_ends_at_object() {
    for ty in EVERY_TYPE {
        let mut root = ty;
        while let Some(base) = root.base() {
            root = base;
        }
        assert!(same(root, &OBJECT), "{}", ty.name());
    }
    assert!(OBJECT.base().is_none());
}

#[test]
fn type_objects_are_immortal_objects_of_type_type() {
    for ty in EVERY_TYPE {
        assert_eq!(
            ty.header,
            ObjectHeader {
                reference_count: IMMORTAL,
                type_id: TYPE_TYPE_ID
            }
        );
        // SAFETY: a type object is a live, immortal object.
        let ty = unsafe { type_of(&raw const ty.header) };
        assert!(same(ty, &TYPE));
    }
}

#[test]
fn types_have_python_names() {
    let names: Vec<&str> = EVERY_TYPE.iter().map(|ty| ty.name()).collect();
    assert_eq!(
        names,
        [
            "object",
            "type",
            "int",
            "str",
            "bytes",
            "list",
            "tuple",
            "set",
            "dict",
            "dict_keys",
            "dict_values",
            "dict_items",
            "BaseException"
        ]
    );
}

#[test]
fn objects_find_their_types() {
    let text = crate::string::from_str("abc");
    let word = |value| crate::word::inline(value).unwrap_or(1);
    let list = crate::list::typy_rt_list_new(word(0), word(0), 0);
    let dict = crate::dict::typy_rt_dict_new(word(0), 0, word(0), 0, word(0));
    // SAFETY: each object is live until its last reference is dropped.
    unsafe {
        let views = [0, 1, 2].map(|kind| crate::dict::typy_rt_dict_view(dict, word(kind)));
        assert!(same(type_of(text.cast()), &STR));
        assert!(same(type_of(list.cast()), &LIST));
        assert!(same(type_of(dict.cast()), &DICT));
        for (view, ty) in views
            .into_iter()
            .zip([&DICT_KEYS, &DICT_VALUES, &DICT_ITEMS])
        {
            assert!(same(type_of(view.cast()), ty));
            crate::refcount::typy_rt_decrement(view.cast());
        }
        for object in [text.cast(), list.cast(), dict.cast()] {
            crate::refcount::typy_rt_decrement(object);
        }
    }
}

#[test]
fn object_has_no_functions_of_its_own() {
    assert!(OBJECT.truth.is_none());
    assert!(OBJECT.hash.is_none());
    assert!(OBJECT.equal.is_none());
    assert!(OBJECT.order.is_none());
    assert!(OBJECT.repr.is_none());
    assert!(STR.hash.is_some());
    assert!(LIST.hash.is_none());
    assert!(DICT.order.is_none());
}

#[test]
fn each_runtime_type_tests_hashes_compares_and_writes_its_objects() {
    let word = |value| crate::word::inline(value).unwrap_or(1);
    let text = crate::string::from_str("ab");
    let bytes = crate::bytes::new_bytes(0);
    let list = crate::list::typy_rt_list_new(word(0), word(1), 0);
    let tuple = crate::list::typy_rt_list_new(word(0), word(1), 0);
    let set = crate::set::typy_rt_set_new(word(0), 0);
    let dict = crate::dict::typy_rt_dict_new(word(0), 0, word(0), 0, word(0));
    // SAFETY: every object is live until its last reference is dropped at the end, and each
    // type's functions take objects of that type.
    unsafe {
        crate::list::typy_rt_list_append(list, word(1).cast_unsigned());
        crate::list::typy_rt_list_append(tuple, word(2).cast_unsigned());
        crate::list::typy_rt_tuple_seal(tuple);
        crate::set::typy_rt_set_add(set, word(3).cast_unsigned());
        let view = crate::dict::typy_rt_dict_view(dict, word(0));
        let objects: [(*mut crate::header::ObjectHeader, bool); 7] = [
            (text.cast(), true),
            (bytes.cast(), false),
            (list.cast(), true),
            (tuple.cast(), true),
            (set.cast(), true),
            (dict.cast(), false),
            (view.cast(), false),
        ];
        for (object, filled) in objects {
            let ty = type_of(object);
            assert_eq!(
                ty.truth.map(|truth| truth(object)),
                Some(filled),
                "{}",
                ty.name()
            );
            if let Some(equal) = ty.equal {
                assert!(equal(object, object), "{}", ty.name());
            }
            if let Some(order) = ty.order {
                // The comparison code 3 asks for `<=`, which every object holds against itself.
                assert!(order(object, object, word(3)), "{}", ty.name());
            }
            if let Some(hash) = ty.hash {
                assert_eq!(hash(object), hash(object));
            }
            let written = (ty.repr.expect("every runtime type writes its objects"))(object);
            assert!(written.exception.is_null());
            crate::refcount::typy_rt_decrement(written.value.cast());
            crate::refcount::typy_rt_decrement(object);
        }
    }
}
