//! Tests of the type objects and of the functions each gives.

use super::{
    BOOL, BYTES, DICT, DICT_ITEMS, DICT_KEYS, DICT_VALUES, FLOAT, INT, LIST, NONE_TYPE, OBJECT,
    SET, STR, TUPLE, TYPE, TypeObject, type_of, typy_rt_type_name, typy_rt_type_text,
};
use crate::header::{IMMORTAL, ObjectHeader, TYPE_TYPE_ID};

/// Returns whether two type objects are one.
fn same(left: &TypeObject, right: &TypeObject) -> bool {
    core::ptr::from_ref(left) == core::ptr::from_ref(right)
}

const EVERY_TYPE: [&TypeObject; 16] = [
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
    &crate::exception::classes::BASE_EXCEPTION,
    &FLOAT,
    &BOOL,
    &NONE_TYPE,
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
            "BaseException",
            "float",
            "bool",
            "NoneType"
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

/// Returns the text of a runtime string, and releases it.
fn text(string: *mut crate::string::StrObject) -> String {
    use crate::string::{code_point, length};
    // SAFETY: the runtime returns a live string, and every index is in bounds.
    let text = (0..unsafe { length(string) })
        .map(|index| char::from_u32(unsafe { code_point(string, index) }).unwrap_or('?'))
        .collect();
    // SAFETY: the string is no longer used.
    unsafe { crate::refcount::typy_rt_decrement(string.cast()) };
    text
}

#[test]
fn types_are_written_as_cpython_writes_them() {
    let mut class = TypeObject::new("Point", &raw const OBJECT);
    class.module = "__main__".as_ptr();
    class.module_length = 8;
    // SAFETY: each type object is live for the whole test.
    unsafe {
        assert_eq!(text(typy_rt_type_text(&raw const INT)), "<class 'int'>");
        assert_eq!(
            text(typy_rt_type_text(&raw const class)),
            "<class '__main__.Point'>"
        );
        assert_eq!(text(typy_rt_type_name(&raw const class)), "Point");
        assert_eq!(text(typy_rt_type_name(&raw const NONE_TYPE)), "NoneType");
        let header = (&raw const FLOAT).cast_mut().cast();
        let written = (TYPE.repr.expect("a type is written"))(header);
        assert_eq!(text(written.value), "<class 'float'>");
        let hash = TYPE.hash.expect("a type has a hash");
        assert_eq!(hash(header), hash(header));
    }
}

#[test]
fn the_traceback_is_one_immortal_object_written_with_its_address() {
    let traceback = super::typy_rt_traceback();
    // SAFETY: the traceback is immortal.
    unsafe {
        assert!(same(type_of(traceback), &super::TRACEBACK_TYPE));
        let text = super::typy_rt_object_repr(traceback.cast_mut());
        let length = crate::string::length(text.value);
        let shown: String = (0..length)
            .map(|index| char::from_u32(crate::string::code_point(text.value, index)).unwrap())
            .collect();
        assert!(shown.starts_with("<traceback object at 0x"), "{shown}");
        crate::refcount::typy_rt_decrement(text.value.cast());
        crate::refcount::typy_rt_decrement(traceback.cast_mut());
        assert_eq!((*traceback).reference_count, IMMORTAL);
    }
}
