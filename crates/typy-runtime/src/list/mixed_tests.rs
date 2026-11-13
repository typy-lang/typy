//! Tests of object items of different types, as a union of objects holds them: each type is
//! equal only to objects of its own type, unless a class's `__eq__` says otherwise (TYP-14).

use super::{ItemKind, ItemType};
use crate::string::from_str;

/// Returns a `bytes` with the bytes of `text`, which is ASCII.
fn bytes_of(text: &str) -> u64 {
    let object = from_str(text);
    // SAFETY: the string was just made, one byte per code point, and nothing else refers to it.
    unsafe { (*object).header.type_id = crate::header::BYTES_TYPE_ID };
    object.expose_provenance() as u64
}

#[test]
fn a_str_and_bytes_of_the_same_text_are_not_equal() {
    let text = from_str("a").expose_provenance() as u64;
    let same = from_str("a").expose_provenance() as u64;
    let raw = bytes_of("a");
    let objects = ItemType::from(ItemKind::Object);
    // SAFETY: each item is a live object until it is released below.
    unsafe {
        assert!(objects.equal(text, same));
        assert!(!objects.equal(text, raw));
        assert!(!objects.equal(raw, text));
        for item in [text, same, raw] {
            objects.release(item);
        }
    }
}

/// The descriptor of an `int | float | str | bool` union: a tag, then a slot for each member.
fn union_type() -> ItemType {
    use crate::tuple::{Field, TupleType};
    let field = |kind: ItemKind, offset: u32| Field {
        kind: kind as u32,
        offset,
        tuple: core::ptr::null(),
    };
    let fields = Box::leak(Box::new([
        field(ItemKind::Int, 0),
        field(ItemKind::Int, 8),
        field(ItemKind::Float, 16),
        field(ItemKind::Object, 24),
        field(ItemKind::Bool, 32),
    ]));
    let union = Box::leak(Box::new(TupleType {
        size: 40,
        length: 5,
        fields: fields.as_ptr(),
    }));
    ItemType::of(ItemKind::Union as u32, union)
}

fn word(value: i64) -> u64 {
    crate::word::inline(value).unwrap().cast_unsigned()
}

#[test]
fn a_union_item_is_the_member_its_tag_names() {
    let item_type = union_type();
    let text = from_str("a").expose_provenance() as u64;
    let one = [word(0), word(1), 0, 0, 0];
    let one_float = [word(1), word(0), 1.0_f64.to_bits(), 0, 0];
    let half = [word(1), word(0), 0.5_f64.to_bits(), 0, 0];
    let string = [word(2), word(0), 0, text, 0];
    let truth = [word(3), word(0), 0, 0, 1];
    let address = |words: &[u64; 5]| words.as_ptr().expose_provenance() as u64;
    let list = super::new_list(item_type, 0);
    // SAFETY: the list and each union are live; the list adds its own references.
    unsafe {
        for each in [&one, &half, &string, &truth] {
            super::typy_rt_list_append(list, address(each));
        }
        let written = super::typy_rt_list_repr(list).value;
        let shown: String = (0..crate::string::length(written))
            .map(|index| char::from_u32(crate::string::code_point(written, index)).unwrap())
            .collect();
        assert_eq!(shown, "[1, 0.5, 'a', True]");
        crate::refcount::typy_rt_decrement(written.cast());
        crate::refcount::typy_rt_decrement(list.cast());
        // Members compare as CPython compares them: 1 == 1.0 == True, and 'a' is no number.
        assert!(item_type.equal(address(&one), address(&one_float)));
        assert!(item_type.equal(address(&one), address(&truth)));
        assert!(item_type.equal(address(&truth), address(&one_float)));
        assert!(!item_type.equal(address(&one), address(&half)));
        assert!(!item_type.equal(address(&string), address(&one)));
        assert_eq!(
            item_type.hash(address(&one)),
            item_type.hash(address(&one_float))
        );
        assert!(item_type.is_true(address(&string)));
        crate::refcount::typy_rt_decrement(core::ptr::with_exposed_provenance_mut(
            usize::try_from(text).unwrap(),
        ));
    }
}

#[test]
fn an_int_equals_only_a_whole_finite_float_of_its_value() {
    let item_type = union_type();
    let address = |words: &[u64; 5]| words.as_ptr().expose_provenance() as u64;
    let three = [word(0), word(3), 0, 0, 0];
    let float = |value: f64| [word(1), word(0), value.to_bits(), 0, 0];
    // SAFETY: each union is live and holds only uncounted values.
    unsafe {
        assert!(item_type.equal(address(&three), address(&float(3.0))));
        for other in [3.5, f64::INFINITY, f64::NAN, 1e19] {
            assert!(
                !item_type.equal(address(&three), address(&float(other))),
                "{other}"
            );
        }
    }
}
