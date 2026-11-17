//! Tests of `ctypes` objects: scalars, pointers and arrays over their buffers.

use super::{
    ARRAY, CDataClass, CDataObject, POINTER, SCALAR, ctype, typy_rt_cdata_argument,
    typy_rt_cdata_contents, typy_rt_cdata_from_address, typy_rt_cdata_get_address,
    typy_rt_cdata_get_bool, typy_rt_cdata_get_bytes, typy_rt_cdata_get_float,
    typy_rt_cdata_get_int, typy_rt_cdata_item, typy_rt_cdata_length, typy_rt_cdata_new,
    typy_rt_cdata_pointer, typy_rt_cdata_set_address, typy_rt_cdata_set_bool,
    typy_rt_cdata_set_bytes, typy_rt_cdata_set_float, typy_rt_cdata_set_int,
};
use crate::int::value::Integer;
use crate::word::inline;

const fn class(name: &'static str, kind: u32, ctype: u32, size: u64) -> CDataClass {
    CDataClass {
        name: name.as_ptr(),
        name_length: name.len() as u64,
        kind,
        ctype,
        size,
        item: core::ptr::null(),
        length: 0,
    }
}

static INT: CDataClass = class("c_int", SCALAR, ctype::INT32, 4);
static UBYTE: CDataClass = class("c_ubyte", SCALAR, ctype::UINT8, 1);
static ULONG: CDataClass = class("c_ulong", SCALAR, ctype::UINT64, 8);
static FLOAT: CDataClass = class("c_float", SCALAR, ctype::FLOAT, 4);
static BOOL: CDataClass = class("c_bool", SCALAR, ctype::BOOL, 1);
static TEXT: CDataClass = class("c_char_p", SCALAR, ctype::CHAR_POINTER, 8);
static ADDRESS: CDataClass = class("c_void_p", SCALAR, ctype::VOID_POINTER, 8);

/// Classes whose item is a static, made once for each test and not moved after.
struct Classes {
    pointer: CDataClass,
    array: CDataClass,
}

fn classes() -> Classes {
    Classes {
        pointer: CDataClass {
            item: &raw const INT,
            ..class("LP_c_int", POINTER, 0, 8)
        },
        array: CDataClass {
            item: &raw const INT,
            length: 3,
            ..class("c_int_Array_3", ARRAY, 0, 12)
        },
    }
}

fn int_word(value: i128) -> i64 {
    Integer::from_i128(value).into_word()
}

fn int_of(word: i64) -> i128 {
    // SAFETY: the word is live and dropped here.
    unsafe {
        let value = Integer::from_word(word).to_i128().unwrap();
        crate::refcount::typy_rt_int_decrement(word);
        value
    }
}

fn drop_object(object: *mut CDataObject) {
    // SAFETY: the object is the test's own.
    unsafe { crate::refcount::typy_rt_decrement(object.cast()) };
}

fn text(string: *mut crate::string::StrObject) -> String {
    // SAFETY: the string is live and dropped here.
    unsafe {
        let shown = (0..crate::string::length(string))
            .filter_map(|index| char::from_u32(crate::string::code_point(string, index)))
            .collect();
        crate::refcount::typy_rt_decrement(string.cast());
        shown
    }
}

fn repr(object: *mut CDataObject) -> String {
    // SAFETY: the object is live.
    text(unsafe { super::cdata_repr(object.cast()).value })
}

fn truth(object: *mut CDataObject) -> bool {
    // SAFETY: the object is live.
    unsafe { super::cdata_truth(object.cast()) }
}

#[test]
fn a_new_scalar_is_zero_and_wraps_what_it_is_assigned() {
    // SAFETY: each object is made, used and dropped here.
    unsafe {
        let number = typy_rt_cdata_new(&raw const INT);
        assert_eq!(int_of(typy_rt_cdata_get_int(number)), 0);
        assert!(!truth(number));
        let word = int_word((1 << 40) + 3);
        typy_rt_cdata_set_int(number, word);
        crate::refcount::typy_rt_int_decrement(word);
        assert_eq!(int_of(typy_rt_cdata_get_int(number)), 3);
        typy_rt_cdata_set_int(number, inline(-5).unwrap());
        assert_eq!(int_of(typy_rt_cdata_get_int(number)), -5);
        assert_eq!(repr(number), "c_int(-5)");
        assert!(truth(number));
        drop_object(number);
        let byte = typy_rt_cdata_new(&raw const UBYTE);
        typy_rt_cdata_set_int(byte, inline(300).unwrap());
        assert_eq!(repr(byte), "c_ubyte(44)");
        drop_object(byte);
        let wide = typy_rt_cdata_new(&raw const ULONG);
        typy_rt_cdata_set_int(wide, inline(-1).unwrap());
        assert_eq!(int_of(typy_rt_cdata_get_int(wide)), i128::from(u64::MAX));
        assert_eq!(repr(wide), "c_ulong(18446744073709551615)");
        drop_object(wide);
    }
}

#[test]
fn floats_bools_strings_and_addresses_hold_their_values() {
    // SAFETY: each object is made, used and dropped here.
    unsafe {
        let single = typy_rt_cdata_new(&raw const FLOAT);
        typy_rt_cdata_set_float(single, 0.1);
        assert_eq!(typy_rt_cdata_get_float(single), f64::from(0.1_f32));
        assert_eq!(repr(single), "c_float(0.10000000149011612)");
        drop_object(single);
        let flag = typy_rt_cdata_new(&raw const BOOL);
        assert_eq!(repr(flag), "c_bool(False)");
        typy_rt_cdata_set_bool(flag, true);
        assert!(typy_rt_cdata_get_bool(flag) && truth(flag));
        drop_object(flag);
        let string = typy_rt_cdata_new(&raw const TEXT);
        assert!(typy_rt_cdata_get_bytes(string).is_null());
        assert_eq!(repr(string), "c_char_p(None)");
        let bytes = crate::bytes::from_slice(b"abc");
        typy_rt_cdata_set_bytes(string, bytes);
        crate::refcount::typy_rt_decrement(bytes.cast());
        let read = typy_rt_cdata_get_bytes(string);
        assert_eq!(crate::string::length(read), 3);
        crate::refcount::typy_rt_decrement(read.cast());
        assert!(truth(string));
        typy_rt_cdata_set_bytes(string, core::ptr::null_mut());
        assert!(!truth(string));
        drop_object(string);
        let address = typy_rt_cdata_new(&raw const ADDRESS);
        assert_eq!(typy_rt_cdata_get_address(address), 0);
        typy_rt_cdata_set_address(address, inline(16).unwrap());
        assert_eq!(int_of(typy_rt_cdata_get_address(address)), 16);
        assert_eq!(repr(address), "c_void_p(16)");
        drop_object(address);
    }
}

#[test]
fn a_pointer_reads_and_writes_its_target_which_it_keeps_alive() {
    let classes = classes();
    // SAFETY: each object is made, used and dropped here.
    unsafe {
        let target = typy_rt_cdata_new(&raw const INT);
        typy_rt_cdata_set_int(target, inline(7).unwrap());
        let pointer = typy_rt_cdata_pointer(&raw const classes.pointer, target);
        drop_object(target);
        assert!(truth(pointer));
        assert_eq!(typy_rt_cdata_argument(pointer), (*target).address);
        let contents = typy_rt_cdata_contents(pointer).value;
        assert_eq!(int_of(typy_rt_cdata_get_int(contents)), 7);
        typy_rt_cdata_set_int(contents, inline(9).unwrap());
        drop_object(contents);
        let item = typy_rt_cdata_item(pointer, inline(0).unwrap()).value;
        assert_eq!(int_of(typy_rt_cdata_get_int(item)), 9);
        drop_object(item);
        assert!(repr(pointer).starts_with("<ctypes.LP_c_int object at 0x"));
        drop_object(pointer);
        let null = typy_rt_cdata_from_address(&raw const classes.pointer, core::ptr::null_mut());
        assert!(!truth(null));
        for result in [
            typy_rt_cdata_contents(null),
            typy_rt_cdata_item(null, inline(0).unwrap()),
        ] {
            assert!(!result.exception.is_null());
            crate::refcount::typy_rt_decrement(result.exception.cast());
        }
        drop_object(null);
    }
}

#[test]
fn an_array_checks_its_indexes_and_counts_negative_ones_from_the_end() {
    let classes = classes();
    // SAFETY: each object is made, used and dropped here.
    unsafe {
        let array = typy_rt_cdata_new(&raw const classes.array);
        assert_eq!(typy_rt_cdata_length(array), inline(3).unwrap());
        assert!(truth(array));
        for (index, value) in [(0, 10), (1, 20), (2, 30)] {
            let item = typy_rt_cdata_item(array, inline(index).unwrap()).value;
            typy_rt_cdata_set_int(item, inline(value).unwrap());
            drop_object(item);
        }
        let last = typy_rt_cdata_item(array, inline(-1).unwrap()).value;
        assert_eq!(int_of(typy_rt_cdata_get_int(last)), 30);
        drop_object(last);
        assert_eq!(typy_rt_cdata_argument(array), (*array).address);
        assert!(typy_rt_cdata_argument(core::ptr::null_mut()).is_null());
        assert_eq!(super::typy_rt_cdata_size(array), inline(12).unwrap());
        for index in [3, -4, 1 << 40] {
            let outside = typy_rt_cdata_item(array, inline(index).unwrap());
            assert!(!outside.exception.is_null(), "{index}");
            crate::refcount::typy_rt_decrement(outside.exception.cast());
        }
        assert!(repr(array).starts_with("<__main__.c_int_Array_3 object at 0x"));
        drop_object(array);
    }
}

#[test]
fn bytes_written_through_an_item_or_a_pointer_live_as_long_as_the_memory_owner() {
    let texts = CDataClass {
        item: &raw const TEXT,
        length: 2,
        ..class("c_char_p_Array_2", ARRAY, 0, 16)
    };
    let pointer = CDataClass {
        item: &raw const TEXT,
        ..class("LP_c_char_p", POINTER, 0, 8)
    };
    // SAFETY: each object is made, used and dropped here.
    unsafe {
        let array = typy_rt_cdata_new(&raw const texts);
        for (index, text) in [(0, &b"first"[..]), (1, b"second"), (0, b"third")] {
            let item = typy_rt_cdata_item(array, inline(index).unwrap()).value;
            let bytes = crate::bytes::from_slice(text);
            typy_rt_cdata_set_bytes(item, bytes);
            crate::refcount::typy_rt_decrement(bytes.cast());
            drop_object(item);
        }
        for (index, length) in [(0, 5), (1, 6)] {
            let item = typy_rt_cdata_item(array, inline(index).unwrap()).value;
            let read = typy_rt_cdata_get_bytes(item);
            assert_eq!(crate::string::length(read), length);
            crate::refcount::typy_rt_decrement(read.cast());
            drop_object(item);
        }
        let cleared = typy_rt_cdata_item(array, inline(1).unwrap()).value;
        typy_rt_cdata_set_bytes(cleared, core::ptr::null_mut());
        assert!(typy_rt_cdata_get_bytes(cleared).is_null());
        drop_object(cleared);
        drop_object(array);
        // Bytes written through a pointer's contents live in the target.
        let target = typy_rt_cdata_new(&raw const TEXT);
        let to_target = typy_rt_cdata_pointer(&raw const pointer, target);
        let contents = typy_rt_cdata_contents(to_target).value;
        let bytes = crate::bytes::from_slice(b"kept");
        typy_rt_cdata_set_bytes(contents, bytes);
        crate::refcount::typy_rt_decrement(bytes.cast());
        drop_object(contents);
        drop_object(to_target);
        let read = typy_rt_cdata_get_bytes(target);
        assert_eq!(crate::string::length(read), 4);
        crate::refcount::typy_rt_decrement(read.cast());
        drop_object(target);
    }
}

static DOUBLE: CDataClass = class("c_double", SCALAR, ctype::DOUBLE, 8);

#[test]
fn a_double_a_true_bool_and_an_index_too_large_for_a_word_read_as_cpython_reads_them() {
    let classes = classes();
    // SAFETY: each object is made, used and dropped here.
    unsafe {
        let real = typy_rt_cdata_new(&raw const DOUBLE);
        typy_rt_cdata_set_float(real, 0.1);
        assert_eq!(typy_rt_cdata_get_float(real), 0.1);
        assert_eq!(repr(real), "c_double(0.1)");
        drop_object(real);
        let flag = typy_rt_cdata_new(&raw const BOOL);
        typy_rt_cdata_set_bool(flag, true);
        assert_eq!(repr(flag), "c_bool(True)");
        drop_object(flag);
        let array = typy_rt_cdata_new(&raw const classes.array);
        let outside = typy_rt_cdata_item(array, int_word(1 << 100));
        assert!(!outside.exception.is_null());
        crate::refcount::typy_rt_decrement(outside.exception.cast());
        // A pointer C gave keeps nothing; an object read through it keeps the pointer.
        let first = typy_rt_cdata_item(array, inline(0).unwrap()).value;
        typy_rt_cdata_set_int(first, inline(5).unwrap());
        drop_object(first);
        let given = typy_rt_cdata_from_address(&raw const classes.pointer, (*array).address);
        let item = typy_rt_cdata_item(given, inline(0).unwrap()).value;
        assert_eq!(int_of(typy_rt_cdata_get_int(item)), 5);
        assert!(core::ptr::eq((*item).owner, given.cast()));
        drop_object(item);
        drop_object(given);
        drop_object(array);
    }
}

#[test]
fn a_table_of_kept_bytes_grows_and_forgets_an_entry_set_to_none() {
    let texts = CDataClass {
        item: &raw const TEXT,
        length: 6,
        ..class("c_char_p_Array_6", ARRAY, 0, 48)
    };
    // SAFETY: each object is made, used and dropped here.
    unsafe {
        let single = typy_rt_cdata_new(&raw const TEXT);
        typy_rt_cdata_set_bytes(single, core::ptr::null_mut());
        assert!(typy_rt_cdata_get_bytes(single).is_null());
        drop_object(single);
        let array = typy_rt_cdata_new(&raw const texts);
        let set = |index: i64, bytes: *mut crate::string::StrObject| {
            let item = typy_rt_cdata_item(array, inline(index).unwrap()).value;
            typy_rt_cdata_set_bytes(item, bytes);
            drop_object(item);
        };
        set(0, core::ptr::null_mut());
        for index in 0..6 {
            let bytes = crate::bytes::from_slice(&b"abcdef"[..=usize::try_from(index).unwrap()]);
            set(index, bytes);
            crate::refcount::typy_rt_decrement(bytes.cast());
        }
        set(2, core::ptr::null_mut());
        for (index, length) in [(0, Some(1)), (2, None), (5, Some(6))] {
            let item = typy_rt_cdata_item(array, inline(index).unwrap()).value;
            let read = typy_rt_cdata_get_bytes(item);
            assert_eq!(
                (!read.is_null()).then(|| crate::string::length(read)),
                length
            );
            if !read.is_null() {
                crate::refcount::typy_rt_decrement(read.cast());
            }
            drop_object(item);
        }
        drop_object(array);
    }
}
