//! Type objects: the name of each type, its base, and the functions that test, hash, compare
//! and write its objects (ADR-0012, CLS-10). Every chain of bases ends at `object`. The runtime
//! frees its own types by their type IDs, so a program that only frees strings links none of
//! the other types' functions (NFR-01).

mod builtin;
#[cfg(test)]
mod tests;

pub(crate) use builtin::{
    BASE_EXCEPTION, BYTES, DICT, DICT_ITEMS, DICT_KEYS, DICT_VALUES, INT, LIST, SET, STR, TUPLE,
};

use crate::Raising;
use crate::header::{
    BIG_INT_TYPE_ID, BYTES_TYPE_ID, DICT_TYPE_ID, DICT_VIEW_TYPE_ID, EXCEPTION_TYPE_ID, IMMORTAL,
    LIST_TYPE_ID, ObjectHeader, SET_TYPE_ID, STR_TYPE_ID, TUPLE_TYPE_ID, TYPE_TYPE_ID,
};
use crate::string::StrObject;

/// Returns whether an object is true.
pub type Truth = unsafe extern "C" fn(*mut ObjectHeader) -> bool;
/// Returns the hash of an object.
pub type Hash = unsafe extern "C" fn(*mut ObjectHeader) -> i64;
/// Returns whether two objects of the type are equal.
pub type Equal = unsafe extern "C" fn(*mut ObjectHeader, *mut ObjectHeader) -> bool;
/// Returns whether two objects of the type stand in a comparison, given as an `int` word.
pub type Order = unsafe extern "C" fn(*mut ObjectHeader, *mut ObjectHeader, i64) -> bool;
/// Returns `repr` of an object, or the exception that writing it raised.
pub type Repr = unsafe extern "C" fn(*mut ObjectHeader) -> Raising<*mut StrObject>;

/// A type: itself an immortal object, whose type is `type`.
#[repr(C)]
#[derive(Debug)]
pub struct TypeObject {
    /// The object header, immortal, with the type ID of `type`.
    pub header: ObjectHeader,
    /// The UTF-8 bytes of the type's name.
    pub name: *const u8,
    /// The number of bytes of the name.
    pub name_length: u64,
    /// The base type, or null for `object`, which has none.
    pub base: *const TypeObject,
    /// Tests an object; absent when every object of the type is true, as for `object`.
    pub truth: Option<Truth>,
    /// Hashes an object; absent for a type whose objects are not hashable.
    pub hash: Option<Hash>,
    /// Compares two objects for equality; absent when equality is identity, as for `object`.
    pub equal: Option<Equal>,
    /// Orders two objects; absent for a type whose objects have no order.
    pub order: Option<Order>,
    /// Writes an object as `repr` does; absent for a type with no text of its own.
    pub repr: Option<Repr>,
}

// SAFETY: a type object is immortal and never written after it is built, so threads may share it.
unsafe impl Sync for TypeObject {}

impl TypeObject {
    /// Returns a type object with a name, a base and no functions of its own.
    pub(crate) const fn new(name: &'static str, base: *const Self) -> Self {
        Self {
            header: ObjectHeader {
                reference_count: IMMORTAL,
                type_id: TYPE_TYPE_ID,
            },
            name: name.as_ptr(),
            name_length: name.len() as u64,
            base,
            truth: None,
            hash: None,
            equal: None,
            order: None,
            repr: None,
        }
    }

    /// Returns the type's name.
    #[cfg(test)]
    pub(crate) fn name(&self) -> &str {
        let length = usize::try_from(self.name_length).unwrap_or(0);
        // SAFETY: `name` points to `name_length` bytes of UTF-8 that live as long as the type.
        unsafe { core::str::from_utf8_unchecked(core::slice::from_raw_parts(self.name, length)) }
    }

    /// Returns the base type, or `None` for `object`.
    #[cfg(test)]
    pub(crate) fn base(&self) -> Option<&Self> {
        // SAFETY: a base is null or a live, immortal type object.
        unsafe { self.base.as_ref() }
    }
}

/// The root of every type: its objects are true, equal only to themselves, and have no base.
pub(crate) static OBJECT: TypeObject = TypeObject::new("object", core::ptr::null());

/// The type of every type object.
pub(crate) static TYPE: TypeObject = TypeObject::new("type", &raw const OBJECT);

/// Returns the type of a heap object.
///
/// # Safety
///
/// `object` points to a live object with a valid header.
#[allow(
    clippy::cast_ptr_alignment,
    reason = "every object comes from malloc, aligned for every type, and starts with its header"
)]
pub(crate) unsafe fn type_of(object: *const ObjectHeader) -> &'static TypeObject {
    // SAFETY: the caller passes a live object; the type ID says which it is.
    unsafe {
        match (*object).type_id {
            STR_TYPE_ID => &STR,
            BYTES_TYPE_ID => &BYTES,
            BIG_INT_TYPE_ID => &INT,
            LIST_TYPE_ID => &LIST,
            TUPLE_TYPE_ID => &TUPLE,
            SET_TYPE_ID => &SET,
            DICT_TYPE_ID => &DICT,
            DICT_VIEW_TYPE_ID => match (*object.cast::<crate::dict::ViewObject>()).kind {
                0 => &DICT_KEYS,
                1 => &DICT_VALUES,
                _ => &DICT_ITEMS,
            },
            EXCEPTION_TYPE_ID => &BASE_EXCEPTION,
            TYPE_TYPE_ID => &TYPE,
            _ => crate::libc::abort_with(b"typy runtime: an object has an unknown type\n"),
        }
    }
}
