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
