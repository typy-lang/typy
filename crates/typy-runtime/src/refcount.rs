//! Reference counting of heap objects (ADR-0001, MEM-01).

use crate::header::{BIG_INT_TYPE_ID, EXCEPTION_TYPE_ID, IMMORTAL, ObjectHeader, STR_TYPE_ID};

/// Adds a reference to an object.
///
/// # Safety
///
/// `object` points to a live object with a valid header.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn typy_rt_increment(object: *mut ObjectHeader) {
    // SAFETY: the caller passes a live object.
    let header = unsafe { &mut *object };
    if header.reference_count != IMMORTAL {
        // A count that reaches the immortal value keeps the object alive for good, which is safe.
        header.reference_count += 1;
    }
}

/// Drops a reference to an object, and frees it when it was the last.
///
/// # Safety
///
/// `object` points to a live object with a valid header, and the caller owns the reference it drops.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn typy_rt_decrement(object: *mut ObjectHeader) {
    // SAFETY: the caller passes a live object it holds a reference to.
    let header = unsafe { &mut *object };
    match header.reference_count {
        IMMORTAL => {}
        1 => {
            let type_id = header.type_id;
            // SAFETY: this was the last reference, so nothing else uses the object.
            unsafe { destroy(object, type_id) };
        }
        count => header.reference_count = count - 1,
    }
}

/// Adds a reference to an `int` word when it points to a large integer.
///
/// # Safety
///
/// A word with its lowest bit clear points to a live large integer.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn typy_rt_int_increment(word: i64) {
    if word & 1 == 0 {
        // SAFETY: a word with its lowest bit clear is the address of a large integer.
        unsafe { typy_rt_increment(crate::int::object_of(word).cast()) };
    }
}

/// Drops a reference to an `int` word when it points to a large integer.
///
/// # Safety
///
/// A word with its lowest bit clear points to a live large integer that the caller holds a reference to.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn typy_rt_int_decrement(word: i64) {
    if word & 1 == 0 {
        // SAFETY: a word with its lowest bit clear is the address of a large integer.
        unsafe { typy_rt_decrement(crate::int::object_of(word).cast()) };
    }
}

/// Frees an object whose last reference is gone, after releasing what it refers to.
unsafe fn destroy(object: *mut ObjectHeader, type_id: u32) {
    match type_id {
        STR_TYPE_ID | BIG_INT_TYPE_ID => crate::allocator::free(object.cast()),
        // SAFETY: an object with the exception type ID is an `ExceptionObject`.
        EXCEPTION_TYPE_ID => unsafe { crate::exception::destroy(object.cast()) },
        _ => crate::libc::abort_with(b"typy runtime: an object has an unknown type\n"),
    }
}

#[cfg(test)]
mod tests {
    use super::{
        typy_rt_decrement, typy_rt_increment, typy_rt_int_decrement, typy_rt_int_increment,
    };
    use crate::header::{IMMORTAL, ObjectHeader, STR_TYPE_ID};

    #[test]
    fn immortal_objects_are_never_counted() {
        let mut header = ObjectHeader {
            reference_count: IMMORTAL,
            type_id: STR_TYPE_ID,
        };
        // SAFETY: the header is live for the whole test.
        unsafe {
            typy_rt_increment(&raw mut header);
            typy_rt_decrement(&raw mut header);
        }
        assert_eq!(header.reference_count, IMMORTAL);
    }

    #[test]
    fn counts_go_up_and_down_and_the_last_reference_frees() {
        let object = crate::string::from_str("abc");
        // SAFETY: the string is live until its last reference is dropped at the end.
        unsafe {
            typy_rt_increment(object.cast());
            assert_eq!((*object).header.reference_count, 2);
            typy_rt_decrement(object.cast());
            assert_eq!((*object).header.reference_count, 1);
            typy_rt_decrement(object.cast());
        }
    }

    #[test]
    fn inline_int_words_are_not_counted() {
        // SAFETY: odd words are inline integers and are never dereferenced.
        unsafe {
            typy_rt_int_increment(7);
            typy_rt_int_decrement(7);
        }
    }

    #[test]
    fn large_int_words_are_counted_like_objects() {
        let word = crate::int::value::Integer::from_i128(1 << 70).into_word();
        let object = crate::int::object_of(word);
        // SAFETY: the integer is live until its last reference is dropped at the end.
        unsafe {
            typy_rt_int_increment(word);
            assert_eq!((*object).header.reference_count, 2);
            typy_rt_int_decrement(word);
            assert_eq!((*object).header.reference_count, 1);
            typy_rt_int_decrement(word);
        }
    }
}
