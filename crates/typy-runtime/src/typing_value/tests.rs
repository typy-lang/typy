//! Tests of typing values.

use crate::header::{IMMORTAL, ObjectHeader, TYPING_VALUE_TYPE_ID};
use crate::string::from_str;

fn object(
    text: *mut crate::string::StrObject,
    key: *mut crate::string::StrObject,
) -> super::TypingValueObject {
    super::TypingValueObject {
        header: ObjectHeader {
            reference_count: IMMORTAL,
            type_id: TYPING_VALUE_TYPE_ID,
        },
        text,
        key,
    }
}

#[test]
fn typ_18_a_typing_value_writes_its_text_and_compares_and_hashes_by_its_key() {
    let (optional, union) = (from_str("int | None"), from_str("int | None"));
    let (key, other_key) = (from_str("{NoneType,int}"), from_str("list[int]"));
    let listed = from_str("list[int]");
    let (left, right, other) = (
        object(optional, key),
        object(union, key),
        object(listed, other_key),
    );
    // SAFETY: the objects and the strings live for the whole test.
    unsafe {
        let left = core::ptr::from_ref(&left).cast_mut().cast();
        let right = core::ptr::from_ref(&right).cast_mut().cast();
        let other = core::ptr::from_ref(&other).cast_mut().cast();
        let written = super::typing_value_repr(left).value;
        let shown: String = (0..crate::string::length(written))
            .map(|index| char::from_u32(crate::string::code_point(written, index)).unwrap())
            .collect();
        assert_eq!(shown, "int | None");
        crate::refcount::typy_rt_decrement(written.cast());
        assert!(super::typing_value_equal(left, right));
        assert!(!super::typing_value_equal(left, other));
        assert_eq!(
            super::typing_value_hash(left),
            super::typing_value_hash(right)
        );
        let ty = crate::type_object::type_of(left);
        assert!(core::ptr::eq(ty, &raw const super::TYPING_VALUE));
        // An object of another type is never equal to a typing value.
        assert!(!super::typing_value_equal(left, key.cast()));
        for each in [optional, union, key, other_key, listed] {
            crate::refcount::typy_rt_decrement(each.cast());
        }
    }
}
