//! `range` as a value: a sequence of `int` with its length, items, slices, membership,
//! equality, hash and text, as CPython's `range` gives them (DAT-17). The bounds are `int`
//! words, so a range may reach past 64 bits as in CPython.

use crate::Raising;
use crate::exception::{ExceptionKind, raise};
use crate::header::{ObjectHeader, RANGE_TYPE_ID};
use crate::int::{
    typy_rt_int_add, typy_rt_int_compare, typy_rt_int_modulo, typy_rt_int_multiply,
    typy_rt_int_subtract,
};
use crate::refcount::{typy_rt_int_decrement, typy_rt_int_increment};
use crate::string::StrObject;
use crate::type_object::{OBJECT, TypeObject};
use crate::word::{inline, inline_value, is_inline};

/// A range: a heap object with the range type ID, which owns its four `int` words.
#[repr(C)]
#[derive(Debug)]
pub(crate) struct RangeObject {
    header: ObjectHeader,
    start: i64,
    stop: i64,
    step: i64,
    /// The number of values.
    length: i64,
}

/// The type `range`: its objects are true when not empty, compared, hashed and written as
/// CPython's.
#[unsafe(export_name = "typy_rt_type_range")]
pub(crate) static RANGE: TypeObject = TypeObject {
    truth: Some(range_is_true),
    hash: Some(range_hash),
    equal: Some(range_equal_objects),
    repr: Some(range_repr),
    ..TypeObject::new("range", &raw const OBJECT)
};

/// Returns the `int` word of `value`.
fn word(value: i64) -> i64 {
    inline(value).unwrap_or(1)
}

/// The comparison codes of `typy_rt_int_compare`.
const EQUAL: i64 = 0;
const LESS: i64 = 2;
const GREATER: i64 = 4;

/// Compares two `int` words.
///
/// # Safety
///
/// Both are `int` words whose large integers are live.
unsafe fn compare(left: i64, right: i64, code: i64) -> bool {
    // SAFETY: the caller's guarantee.
    unsafe { typy_rt_int_compare(left, right, word(code)) }
}

/// Returns `range(start, stop, step)`, which takes its own references to the words; a step
/// of zero raises `ValueError`, as `range()` does.
///
/// # Safety
///
/// Each argument is an `int` word whose large integer is live.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn typy_rt_range_new(
    start: i64,
    stop: i64,
    step: i64,
) -> Raising<*mut RangeObject> {
    if step == word(0) {
        return Raising::error(
            core::ptr::null_mut(),
            raise(ExceptionKind::ValueError, "range() arg 3 must not be zero"),
        );
    }
    // SAFETY: the caller passes live words; each is counted for the range.
    unsafe {
        for value in [start, stop, step] {
            typy_rt_int_increment(value);
        }
        Raising::ok(made(start, stop, step, super::length(start, stop, step)))
    }
}

/// Returns a new range of words it takes over.
fn made(start: i64, stop: i64, step: i64, length: i64) -> *mut RangeObject {
    #[allow(
        clippy::cast_ptr_alignment,
        reason = "malloc returns memory aligned for every type"
    )]
    let range =
        crate::allocator::allocate(core::mem::size_of::<RangeObject>()).cast::<RangeObject>();
    // SAFETY: the allocation holds a range.
    unsafe {
        range.write(RangeObject {
            header: ObjectHeader::new(RANGE_TYPE_ID),
            start,
            stop,
            step,
            length,
        });
    }
    range
}

/// Returns `len(range)`; a length past `sys.maxsize` raises `OverflowError`, as CPython's
/// `len` does.
///
/// # Safety
///
/// `range` points to a live range.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn typy_rt_range_length(range: *const RangeObject) -> Raising<i64> {
    // SAFETY: the caller's guarantee.
    let length = unsafe { (*range).length };
    if !is_inline(length) {
        return Raising::error(
            0,
            raise(
                ExceptionKind::OverflowError,
                "Python int too large to convert to C ssize_t",
            ),
        );
    }
    Raising::ok(length)
}

/// Returns the start, the stop or the step of a range, for `which` 0, 1 or 2 as an `int`
/// word, with a reference of its own.
///
/// # Safety
///
/// `range` points to a live range.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn typy_rt_range_bound(range: *const RangeObject, which: i64) -> i64 {
    // SAFETY: the caller's guarantee.
    unsafe {
        let value = match inline_value(which) {
            0 => (*range).start,
            1 => (*range).stop,
            _ => (*range).step,
        };
        typy_rt_int_increment(value);
        value
    }
}

/// Returns `range[index]`, counting a negative index from the end; one outside raises
/// `IndexError: range object index out of range`.
///
/// # Safety
///
/// `range` points to a live range, and `index` is a live `int` word.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn typy_rt_range_get(range: *const RangeObject, index: i64) -> Raising<i64> {
    // SAFETY: the caller passes live values; each new word is released once used.
    unsafe {
        let range = &*range;
        let position = if compare(index, word(0), LESS) {
            typy_rt_int_add(index, range.length)
        } else {
            typy_rt_int_increment(index);
            index
        };
        if compare(position, word(0), LESS) || !compare(position, range.length, LESS) {
            typy_rt_int_decrement(position);
            return Raising::error(
                0,
                raise(ExceptionKind::IndexError, "range object index out of range"),
            );
        }
        let offset = typy_rt_int_multiply(position, range.step);
        let value = typy_rt_int_add(range.start, offset);
        typy_rt_int_decrement(offset);
        typy_rt_int_decrement(position);
        Raising::ok(value)
    }
}

/// Returns `range[start:stop:step]`, a new range, as CPython's `compute_slice` builds it from
/// the slice's indices; `present` says which bounds are given. A range longer than
/// `sys.maxsize` raises `OverflowError`.
///
/// # Safety
///
/// `range` points to a live range, and the bounds are `int` words.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn typy_rt_range_slice(
    range: *const RangeObject,
    start: i64,
    stop: i64,
    stride: i64,
    present: i64,
) -> Raising<*mut RangeObject> {
    // SAFETY: the caller passes live values; each new word is released or kept once.
    unsafe {
        let range = &*range;
        let length = typy_rt_range_length(range);
        if !length.exception.is_null() {
            return Raising::error(core::ptr::null_mut(), length.exception);
        }
        let count = usize::try_from(inline_value(length.value)).unwrap_or(0);
        let selection = match crate::slice::select(start, stop, stride, present, count) {
            Ok(selection) => selection,
            Err(error) => return Raising::error(core::ptr::null_mut(), error),
        };
        let at = |index: i64| {
            let offset = typy_rt_int_multiply(word(index), range.step);
            let value = typy_rt_int_add(range.start, offset);
            typy_rt_int_decrement(offset);
            value
        };
        let new_start = at(selection.start);
        let new_stop = at(selection.stop);
        let new_step = typy_rt_int_multiply(range.step, word(selection.step));
        let length = super::length(new_start, new_stop, new_step);
        Raising::ok(made(new_start, new_stop, new_step, length))
    }
}

/// Returns whether `value` is one of the range's values, without stepping through them.
///
/// # Safety
///
/// `range` points to a live range, and `value` is a live `int` word.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn typy_rt_range_contains(range: *const RangeObject, value: i64) -> bool {
    // SAFETY: the caller passes live values; each new word is released once used.
    unsafe {
        let range = &*range;
        let inside = if compare(range.step, word(0), GREATER) {
            !compare(value, range.start, LESS) && compare(value, range.stop, LESS)
        } else {
            !compare(value, range.start, GREATER) && compare(value, range.stop, GREATER)
        };
        if !inside {
            return false;
        }
        let offset = typy_rt_int_subtract(value, range.start);
        let remainder = typy_rt_int_modulo(offset, range.step).value;
        let found = compare(remainder, word(0), EQUAL);
        typy_rt_int_decrement(offset);
        typy_rt_int_decrement(remainder);
        found
    }
}

/// Returns whether two ranges give the same values, as CPython's `range_equals` decides: the
/// same length, and for a length past 0 the same start, and past 1 the same step.
///
/// # Safety
///
/// Both point to live ranges.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn typy_rt_range_equal(
    left: *const RangeObject,
    right: *const RangeObject,
) -> bool {
    // SAFETY: the caller's guarantee.
    unsafe {
        let (left, right) = (&*left, &*right);
        if !compare(left.length, right.length, EQUAL) {
            return false;
        }
        if left.length == word(0) {
            return true;
        }
        if !compare(left.start, right.start, EQUAL) {
            return false;
        }
        left.length == word(1) || compare(left.step, right.step, EQUAL)
    }
}

#[allow(
    clippy::cast_ptr_alignment,
    reason = "every object comes from malloc, aligned for every type, and starts with its header"
)]
unsafe extern "C" fn range_equal_objects(
    left: *mut ObjectHeader,
    right: *mut ObjectHeader,
) -> bool {
    // SAFETY: the types of the objects say they are ranges.
    unsafe { typy_rt_range_equal(left.cast(), right.cast()) }
}

#[allow(
    clippy::cast_ptr_alignment,
    reason = "every object comes from malloc, aligned for every type, and starts with its header"
)]
unsafe extern "C" fn range_is_true(object: *mut ObjectHeader) -> bool {
    // SAFETY: the type of the object says it is a range.
    unsafe { (*object.cast::<RangeObject>()).length != word(0) }
}

/// Hashes a range as CPython's `range_hash`: the hash of the tuple of its length, and its
/// start and step, each `None` where the length leaves it out.
#[allow(
    clippy::cast_ptr_alignment,
    reason = "every object comes from malloc, aligned for every type, and starts with its header"
)]
unsafe extern "C" fn range_hash(object: *mut ObjectHeader) -> i64 {
    // SAFETY: the type of the object says it is a range, whose words are live.
    unsafe {
        let range = &*object.cast::<RangeObject>();
        let length = crate::hash::int(range.length);
        let (start, step) = if range.length == word(0) {
            (crate::hash::NONE, crate::hash::NONE)
        } else if range.length == word(1) {
            (crate::hash::int(range.start), crate::hash::NONE)
        } else {
            (crate::hash::int(range.start), crate::hash::int(range.step))
        };
        crate::hash::tuple([length, start, step].into_iter(), 3)
    }
}

/// Writes a range as CPython does: `range(start, stop)`, with the step when it is not 1.
#[allow(
    clippy::cast_ptr_alignment,
    reason = "every object comes from malloc, aligned for every type, and starts with its header"
)]
unsafe extern "C" fn range_repr(object: *mut ObjectHeader) -> Raising<*mut StrObject> {
    // SAFETY: the type of the object says it is a range, whose words are live.
    unsafe {
        let range = &*object.cast::<RangeObject>();
        let mut parts = [range.start, range.stop, range.step]
            .map(|value| crate::int::typy_rt_int_to_str(value).value);
        let shown = if range.step == word(1) { 2 } else { 3 };
        let mut text = crate::string::from_str("range(");
        for (index, part) in parts.iter_mut().enumerate().take(shown) {
            if index > 0 {
                text = append(text, crate::string::from_str(", "));
            }
            text = append(text, *part);
            *part = core::ptr::null_mut();
        }
        for part in parts.into_iter().filter(|part| !part.is_null()) {
            crate::refcount::typy_rt_decrement(part.cast());
        }
        Raising::ok(append(text, crate::string::from_str(")")))
    }
}

/// Returns `left + right` and releases both.
///
/// # Safety
///
/// Both point to live strings owned here.
unsafe fn append(left: *mut StrObject, right: *mut StrObject) -> *mut StrObject {
    // SAFETY: the caller's guarantee.
    unsafe {
        let joined = crate::string::typy_rt_str_concat(left, right);
        crate::refcount::typy_rt_decrement(left.cast());
        crate::refcount::typy_rt_decrement(right.cast());
        joined
    }
}

/// Returns the value at `position`, a new `int` word, or `None` past the last value.
///
/// # Safety
///
/// `range` points to a live range.
pub(crate) unsafe fn item(range: *const RangeObject, position: usize) -> Option<i64> {
    let index = word(i64::try_from(position).unwrap_or(i64::MAX));
    // SAFETY: the caller's guarantee; the offset is released once added.
    unsafe {
        let range = &*range;
        if !compare(index, range.length, LESS) {
            return None;
        }
        let offset = typy_rt_int_multiply(index, range.step);
        let value = typy_rt_int_add(range.start, offset);
        typy_rt_int_decrement(offset);
        Some(value)
    }
}

/// Returns whether the start, stop, step and length all fit a C `long`, which makes CPython's
/// iterator of the range a `range_iterator` and not a `longrange_iterator`.
///
/// # Safety
///
/// `range` points to a live range.
pub(crate) unsafe fn fits_long(range: *const RangeObject) -> bool {
    // SAFETY: the caller's guarantee.
    let range = unsafe { &*range };
    let words = [range.start, range.stop, range.step, range.length];
    if words.iter().all(|&value| is_inline(value)) {
        return true;
    }
    let low = crate::int::value::Integer::from_i128(i128::from(i64::MIN)).into_word();
    let high = crate::int::value::Integer::from_i128(i128::from(i64::MAX)).into_word();
    // SAFETY: the words are live, and the two limits are released once compared.
    unsafe {
        let fits = words
            .iter()
            .all(|&value| !compare(value, low, LESS) && !compare(value, high, GREATER));
        typy_rt_int_decrement(low);
        typy_rt_int_decrement(high);
        fits
    }
}

/// Frees a range whose last reference is gone, with its words.
///
/// # Safety
///
/// `range` points to a live range that nothing else refers to.
pub(crate) unsafe fn destroy(range: *mut RangeObject) {
    // SAFETY: the caller's guarantee; the range owns its words.
    unsafe {
        let object = &*range;
        for value in [object.start, object.stop, object.step, object.length] {
            typy_rt_int_decrement(value);
        }
    }
    crate::allocator::free(range.cast());
}

#[cfg(test)]
mod tests;
