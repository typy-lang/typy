//! Typing values (TYP-18, ADR-0087): a name of `typing` or `collections.abc`, a built-in class
//! given type arguments, or a union of types, read as a value. The compiler emits one immortal
//! object for each distinct value, holding its text, as CPython 3.15 writes it, and a key that
//! the values CPython finds equal share, such as `Optional[int]` and `int | None`. It compares
//! and hashes by its key.

use crate::Raising;
use crate::header::{ObjectHeader, TYPING_VALUE_TYPE_ID};
use crate::string::StrObject;
use crate::type_object::{OBJECT, TypeObject};

/// A typing value, which the compiler emits with static strings.
#[repr(C)]
#[derive(Debug)]
pub(crate) struct TypingValueObject {
    header: ObjectHeader,
    text: *const StrObject,
    key: *const StrObject,
}

/// The type of typing values, which a program reads only through their slots, since `type()`
/// of one is refused (TYP-22).
#[unsafe(export_name = "typy_rt_type_typing_value")]
pub(crate) static TYPING_VALUE: TypeObject = TypeObject {
    hash: Some(typing_value_hash),
    equal: Some(typing_value_equal),
    repr: Some(typing_value_repr),
    module: "typing".as_ptr(),
    module_length: 6,
    ..TypeObject::new("TypingValue", &raw const OBJECT)
};

/// Returns the typing value `object` is.
#[allow(
    clippy::cast_ptr_alignment,
    reason = "a typing value is a static the compiler aligns for its pointers"
)]
const fn value(object: *mut ObjectHeader) -> *const TypingValueObject {
    object.cast_const().cast()
}

/// Returns `repr` of a typing value: its text.
unsafe extern "C" fn typing_value_repr(object: *mut ObjectHeader) -> Raising<*mut StrObject> {
    // SAFETY: the type of the object says it is a typing value, whose text is an immortal
    // static string.
    unsafe {
        let text = (*value(object)).text.cast_mut();
        crate::refcount::typy_rt_increment(text.cast());
        Raising::ok(text)
    }
}

/// Returns true when `right` is a typing value with the key of `left`.
unsafe extern "C" fn typing_value_equal(left: *mut ObjectHeader, right: *mut ObjectHeader) -> bool {
    // SAFETY: the slot is called with a typing value first and a live object second; the
    // second's type ID says whether it is one too.
    unsafe {
        (*right).type_id == TYPING_VALUE_TYPE_ID
            && crate::string::typy_rt_str_compare(
                (*value(left)).key,
                (*value(right)).key,
                crate::word::inline(0).unwrap_or(1),
            )
    }
}

/// Returns the hash of a typing value: its key's, so equal values hash alike.
unsafe extern "C" fn typing_value_hash(object: *mut ObjectHeader) -> i64 {
    // SAFETY: the type of the object says it is a typing value.
    unsafe { crate::hash::text((*value(object)).key) }
}

#[cfg(test)]
mod tests;
