//! The values TyPy holds by value, boxed as objects where a value of type `object` holds them
//! (TYP-23, ADR-0090): an `int` word, a `float`, `True` and `False`, and `None`. A boxed value
//! answers `type()`, `repr`, `str`, `==`, `hash` and truth as CPython's value does, and an
//! `int`, a `float` and a `bool` compare equal by their numbers, as in CPython.

use crate::Raising;
use crate::header::{
    BIG_INT_TYPE_ID, BOOL_OBJECT_TYPE_ID, BOXED_FLOAT_TYPE_ID, BOXED_INT_TYPE_ID, IMMORTAL,
    NONE_OBJECT_TYPE_ID, ObjectHeader,
};
use crate::string::StrObject;
use crate::word::{inline, is_inline};

/// An `int` boxed as an object: its word, inline or the address of a large integer that the
/// box holds a reference to.
#[repr(C)]
#[derive(Debug)]
pub(crate) struct BoxedInt {
    header: ObjectHeader,
    word: i64,
}

/// A `float` boxed as an object.
#[repr(C)]
#[derive(Debug)]
pub(crate) struct BoxedFloat {
    header: ObjectHeader,
    value: f64,
}

/// The one `True` object.
static TRUE: ObjectHeader = ObjectHeader {
    reference_count: IMMORTAL,
    type_id: BOOL_OBJECT_TYPE_ID,
};

/// The one `False` object.
static FALSE: ObjectHeader = ObjectHeader {
    reference_count: IMMORTAL,
    type_id: BOOL_OBJECT_TYPE_ID,
};

/// The one `None` object.
static NONE: ObjectHeader = ObjectHeader {
    reference_count: IMMORTAL,
    type_id: NONE_OBJECT_TYPE_ID,
};

/// Returns the `int` word `word` boxed as an object, with a reference of its own to a large
/// integer.
///
/// # Safety
///
/// A word that is not inline points to a live large integer.
#[allow(
    clippy::cast_ptr_alignment,
    reason = "every object comes from malloc, a frame block or a static, aligned for every type, and starts with its header"
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn typy_rt_box_int(word: i64) -> *mut ObjectHeader {
    // SAFETY: the caller's guarantee.
    unsafe { crate::refcount::typy_rt_int_increment(word) };
    let boxed = crate::allocator::allocate(size_of::<BoxedInt>()).cast::<BoxedInt>();
    // SAFETY: the allocation is large enough and aligned for a `BoxedInt`.
    unsafe {
        boxed.write(BoxedInt {
            header: ObjectHeader::new(BOXED_INT_TYPE_ID),
            word,
        });
    }
    boxed.cast()
}

/// Returns `value` boxed as an object.
#[allow(
    clippy::cast_ptr_alignment,
    reason = "every object comes from malloc, a frame block or a static, aligned for every type, and starts with its header"
)]
#[unsafe(no_mangle)]
pub extern "C" fn typy_rt_box_float(value: f64) -> *mut ObjectHeader {
    let boxed = crate::allocator::allocate(size_of::<BoxedFloat>()).cast::<BoxedFloat>();
    // SAFETY: the allocation is large enough and aligned for a `BoxedFloat`.
    unsafe {
        boxed.write(BoxedFloat {
            header: ObjectHeader::new(BOXED_FLOAT_TYPE_ID),
            value,
        });
    }
    boxed.cast()
}

/// Returns the immortal `True` or `False` object.
#[unsafe(no_mangle)]
pub extern "C" fn typy_rt_box_bool(value: bool) -> *mut ObjectHeader {
    let object = if value {
        &raw const TRUE
    } else {
        &raw const FALSE
    };
    object.cast_mut()
}

/// Returns the immortal `None` object.
#[unsafe(no_mangle)]
pub extern "C" fn typy_rt_box_none() -> *mut ObjectHeader {
    (&raw const NONE).cast_mut()
}

/// Returns the `int` word of a boxed `int`, a large integer or a `bool`, with a reference of
/// its own to a large integer.
///
/// # Safety
///
/// `object` points to a live object of one of those kinds.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn typy_rt_unbox_int(object: *mut ObjectHeader) -> i64 {
    // SAFETY: the caller's guarantee.
    let word = unsafe { int_word(object) }.unwrap_or(0);
    // SAFETY: the word is the object's, which is live.
    unsafe { crate::refcount::typy_rt_int_increment(word) };
    word
}

/// Returns the value of a boxed `float`.
///
/// # Safety
///
/// `object` points to a live boxed `float`.
#[allow(
    clippy::cast_ptr_alignment,
    reason = "every object comes from malloc, a frame block or a static, aligned for every type, and starts with its header"
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn typy_rt_unbox_float(object: *mut ObjectHeader) -> f64 {
    // SAFETY: the caller's guarantee.
    unsafe { (*object.cast::<BoxedFloat>()).value }
}

/// Returns the value of the `True` or `False` object.
#[unsafe(no_mangle)]
pub extern "C" fn typy_rt_unbox_bool(object: *mut ObjectHeader) -> bool {
    core::ptr::eq(object.cast_const(), &raw const TRUE)
}

/// Releases what a boxed `int` refers to, then frees it.
///
/// # Safety
///
/// `object` is a boxed `int` whose last reference is gone.
pub(crate) unsafe fn destroy_int(object: *mut BoxedInt) {
    // SAFETY: the caller's guarantee.
    unsafe { crate::refcount::typy_rt_int_decrement((*object).word) };
    crate::allocator::free(object.cast());
}

/// A number held as an object: an `int` word, or a `float`.
#[derive(Clone, Copy)]
enum Number {
    Int(i64),
    Float(f64),
}

/// Returns the `int` word an object holds: a boxed `int`, a large integer or a `bool`.
///
/// # Safety
///
/// `object` points to a live object.
#[allow(
    clippy::cast_ptr_alignment,
    reason = "every object comes from malloc, a frame block or a static, aligned for every type, and starts with its header"
)]
unsafe fn int_word(object: *mut ObjectHeader) -> Option<i64> {
    // SAFETY: the caller's guarantee; the type ID says how the object is laid out.
    unsafe {
        match (*object).type_id {
            BOXED_INT_TYPE_ID => Some((*object.cast::<BoxedInt>()).word),
            BIG_INT_TYPE_ID => Some(crate::int::word_of(object.cast())),
            BOOL_OBJECT_TYPE_ID => inline(i64::from(typy_rt_unbox_bool(object))),
            _ => None,
        }
    }
}

/// Returns the number an object holds, or `None` for an object that holds none.
///
/// # Safety
///
/// `object` points to a live object.
unsafe fn number(object: *mut ObjectHeader) -> Option<Number> {
    // SAFETY: the caller's guarantee.
    unsafe {
        if (*object).type_id == BOXED_FLOAT_TYPE_ID {
            return Some(Number::Float(typy_rt_unbox_float(object)));
        }
        int_word(object).map(Number::Int)
    }
}

/// Returns whether an object holds a number: a boxed `int` or `float`, a large integer, or
/// `True` or `False`.
///
/// # Safety
///
/// `object` points to a live object.
pub(crate) unsafe fn holds_number(object: *mut ObjectHeader) -> bool {
    // SAFETY: the caller's guarantee.
    let type_id = unsafe { (*object).type_id };
    matches!(
        type_id,
        BOXED_INT_TYPE_ID | BOXED_FLOAT_TYPE_ID | BIG_INT_TYPE_ID | BOOL_OBJECT_TYPE_ID
    )
}

/// Returns whether two objects hold equal numbers, as `==` compares an `int`, a `float` and a
/// `bool` in CPython: exactly, so a large `int` equals a `float` only when its value is that
/// `float`'s.
///
/// # Safety
///
/// Each points to a live object, `left` to a number.
pub(crate) unsafe extern "C" fn numbers_equal(
    left: *mut ObjectHeader,
    right: *mut ObjectHeader,
) -> bool {
    let equal = inline(0).unwrap_or(1);
    // SAFETY: the caller's guarantee.
    let (Some(left), Some(right)) = (unsafe { number(left) }, unsafe { number(right) }) else {
        return false;
    };
    match (left, right) {
        // SAFETY: the words come from live objects.
        (Number::Int(left), Number::Int(right)) => unsafe {
            crate::int::typy_rt_int_compare(left, right, equal)
        },
        // SAFETY: the word comes from a live object.
        (Number::Int(word), Number::Float(value)) | (Number::Float(value), Number::Int(word)) => unsafe {
            crate::float::convert::typy_rt_compare_int_float(word, value, equal)
        },
        // Two `float`s are equal when their values are, as in CPython.
        #[allow(clippy::float_cmp, reason = "CPython compares floats exactly")]
        (Number::Float(left), Number::Float(right)) => left == right,
    }
}

/// Returns the hash of an object that holds a number, as CPython hashes it: equal numbers
/// hash alike.
///
/// # Safety
///
/// `object` points to a live object that holds a number.
pub(crate) unsafe extern "C" fn number_hash(object: *mut ObjectHeader) -> i64 {
    // SAFETY: the caller's guarantee.
    match unsafe { number(object) } {
        // SAFETY: the word comes from a live object.
        Some(Number::Int(word)) => unsafe { crate::hash::int(word) },
        Some(Number::Float(value)) => crate::hash::float(value),
        None => 0,
    }
}

/// Returns whether an object that holds a number is not zero.
///
/// # Safety
///
/// `object` points to a live object that holds a number.
pub(crate) unsafe extern "C" fn number_is_true(object: *mut ObjectHeader) -> bool {
    // SAFETY: the caller's guarantee.
    match unsafe { number(object) } {
        Some(Number::Int(word)) => !(is_inline(word) && inline(0) == Some(word)),
        Some(Number::Float(value)) => value != 0.0,
        None => true,
    }
}

/// Returns `repr` of a boxed `int` or a large integer: its decimal digits.
///
/// # Safety
///
/// `object` points to a live boxed `int` or large integer.
pub(crate) unsafe extern "C" fn int_repr(object: *mut ObjectHeader) -> Raising<*mut StrObject> {
    // SAFETY: the caller's guarantee.
    let word = unsafe { int_word(object) }.unwrap_or(0);
    // SAFETY: the word comes from a live object.
    unsafe { crate::int::typy_rt_int_to_str(word) }
}

/// Returns `repr` of a boxed `float`, as CPython writes it.
///
/// # Safety
///
/// `object` points to a live boxed `float`.
pub(crate) unsafe extern "C" fn float_repr(object: *mut ObjectHeader) -> Raising<*mut StrObject> {
    // SAFETY: the caller's guarantee.
    Raising::ok(crate::float::typy_rt_float_to_str(unsafe {
        typy_rt_unbox_float(object)
    }))
}

/// Returns `repr` of `True` or `False`.
pub(crate) unsafe extern "C" fn bool_repr(object: *mut ObjectHeader) -> Raising<*mut StrObject> {
    let text = if typy_rt_unbox_bool(object) {
        "True"
    } else {
        "False"
    };
    Raising::ok(crate::string::from_str(text))
}

/// Returns `repr` of `None`.
pub(crate) unsafe extern "C" fn none_repr(_object: *mut ObjectHeader) -> Raising<*mut StrObject> {
    Raising::ok(crate::string::from_str("None"))
}

/// Returns false: `None` is false.
pub(crate) unsafe extern "C" fn none_is_true(_object: *mut ObjectHeader) -> bool {
    false
}

#[cfg(test)]
mod tests;
