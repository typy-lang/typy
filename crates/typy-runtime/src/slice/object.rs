//! `slice` as a value (CLS-23): the object `x[a:b:c]` passes to a class's `__getitem__`, and
//! `slice(a, b, c)` makes. Each bound is an `int` word, or 0 for `None`, as an `int | None`
//! holds it (ADR-0011); the object owns its words. It is compared, hashed and written as
//! CPython's `slice` is.

use crate::Raising;
use crate::exception::{ExceptionKind, raise};
use crate::header::{ObjectHeader, SLICE_TYPE_ID};
use crate::int::typy_rt_int_compare;
use crate::refcount::{typy_rt_int_decrement, typy_rt_int_increment};
use crate::string::StrObject;
use crate::type_object::{OBJECT, TypeObject};
use crate::word::{inline, inline_value, is_inline};

/// A slice: a heap object with the slice type ID, which owns its three words.
#[repr(C)]
#[derive(Debug)]
pub(crate) struct SliceObject {
    header: ObjectHeader,
    /// The start, stop and step, each an `int` word or 0 for `None`.
    bounds: [i64; 3],
}

/// The type `slice`: its objects are compared, hashed and written as CPython's.
#[unsafe(export_name = "typy_rt_type_slice")]
pub(crate) static SLICE: TypeObject = TypeObject {
    hash: Some(slice_hash),
    equal: Some(slice_equal),
    repr: Some(slice_repr),
    ..TypeObject::new("slice", &raw const OBJECT)
};

/// Returns `slice(first, last, stride)`, its start, stop and step, which takes its own
/// references to the words.
///
/// # Safety
///
/// Each word is an `int` word whose large integer is live, or 0.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn typy_rt_slice_new(first: i64, last: i64, stride: i64) -> *mut SliceObject {
    for bound in [first, last, stride] {
        // SAFETY: the caller's guarantee.
        unsafe { typy_rt_int_increment(bound) };
    }
    #[allow(
        clippy::cast_ptr_alignment,
        reason = "malloc returns memory aligned for every type"
    )]
    let slice =
        crate::allocator::allocate(core::mem::size_of::<SliceObject>()).cast::<SliceObject>();
    // SAFETY: the allocation holds a slice.
    unsafe {
        slice.write(SliceObject {
            header: ObjectHeader::new(SLICE_TYPE_ID),
            bounds: [first, last, stride],
        });
    }
    slice
}

/// Returns `start`, `stop` or `step` of a slice, by its position 0, 1 or 2 as an `int` word,
/// with a reference of its own: an `int` word, or 0 for `None`.
///
/// # Safety
///
/// `slice` points to a live slice.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn typy_rt_slice_bound(slice: *const SliceObject, part: i64) -> i64 {
    // SAFETY: the caller's guarantee; the position is one of the three.
    unsafe {
        let bound = (*slice).bounds[usize::try_from(inline_value(part)).unwrap_or(0).min(2)];
        typy_rt_int_increment(bound);
        bound
    }
}

/// Returns the value at position `part`, 0, 1 or 2 as an `int` word, of
/// `slice.indices(length)`: the start and stop adjusted to a sequence of `length` items, and the
/// step, as CPython's `slice.indices` gives them, for a length of any size. A negative length
/// and a step of zero raise CPython's errors.
///
/// # Safety
///
/// `slice` points to a live slice, and `length` is an `int` word whose large integer is live.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn typy_rt_slice_index(
    slice: *const SliceObject,
    length: i64,
    part: i64,
) -> Raising<i64> {
    // SAFETY: the caller's guarantee.
    unsafe {
        let [first, last, stride] = (*slice).bounds;
        if !is_inline(length) {
            if crate::int::is_negative(length) {
                return Raising::error(1, negative_length());
            }
            return large_index([first, last, stride], length, part);
        }
        let Ok(count) = usize::try_from(inline_value(length)) else {
            return Raising::error(1, negative_length());
        };
        let present = super::START | super::STOP | super::STEP;
        let selection = match super::select(first, last, stride, word(present), count) {
            Ok(selection) => selection,
            Err(error) => return Raising::error(1, error),
        };
        let value = match inline_value(part) {
            0 => word(selection.start),
            1 => word(selection.stop),
            // The step is given back as it is, which may be past 64 bits.
            _ if stride == 0 => word(1),
            _ => {
                typy_rt_int_increment(stride);
                stride
            }
        };
        Raising::ok(value)
    }
}

/// Returns what `typy_rt_slice_index` does for a length past an inline `int`, with integers of
/// any size, as CPython's `_PySlice_GetLongIndices` computes them: a bound counts from the end
/// when negative, then is held between the first and the last position a step can reach.
///
/// # Safety
///
/// Each bound is 0 or an `int` word whose large integer is live, and `length` is a positive
/// large integer.
unsafe fn large_index(bounds: [i64; 3], length: i64, part: i64) -> Raising<i64> {
    use crate::int::value::Integer;
    let [first, last, stride] = bounds;
    // SAFETY: the caller's guarantee.
    let (step, length) = unsafe {
        let step = if stride == 0 {
            Integer::from_i128(1)
        } else {
            Integer::from_word(stride)
        };
        (step, Integer::from_word(length))
    };
    if step.is_zero() {
        return Raising::error(
            1,
            raise(ExceptionKind::ValueError, "slice step cannot be zero"),
        );
    }
    let backwards = step.negative;
    // The lowest and the highest position a bound is held to.
    let lower = || Integer::from_i128(if backwards { -1 } else { 0 });
    let upper = || {
        if backwards {
            length.subtract(&Integer::from_i128(1))
        } else {
            // `Integer` is not `Clone`; adding zero gives a copy of the length.
            length.add(&Integer::from_i128(0))
        }
    };
    let adjust = |word: i64, missing: Integer| {
        if word == 0 {
            return missing;
        }
        // SAFETY: the caller's guarantee.
        let bound = unsafe { Integer::from_word(word) };
        if bound.negative {
            let from_end = bound.add(&length);
            if from_end.compare(&lower()).is_lt() {
                lower()
            } else {
                from_end
            }
        } else if bound.compare(&upper()).is_gt() {
            upper()
        } else {
            bound
        }
    };
    let value = match inline_value(part) {
        0 => adjust(first, if backwards { upper() } else { lower() }).into_word(),
        1 => adjust(last, if backwards { lower() } else { upper() }).into_word(),
        _ if stride == 0 => word(1),
        _ => {
            // SAFETY: the caller's guarantee.
            unsafe { typy_rt_int_increment(stride) };
            stride
        }
    };
    Raising::ok(value)
}

/// Returns the `ValueError` of `slice.indices` given a negative length.
fn negative_length() -> *mut crate::exception::ExceptionObject {
    raise(ExceptionKind::ValueError, "length should not be negative")
}

/// Returns the `int` word of `value`.
fn word(value: i64) -> i64 {
    inline(value).unwrap_or(1)
}

/// Returns the bounds of a slice object.
///
/// # Safety
///
/// `object` points to a live slice.
#[allow(
    clippy::cast_ptr_alignment,
    reason = "every object comes from malloc, a frame block or a static, aligned for every type, and starts with its header"
)]
unsafe fn bounds(object: *mut ObjectHeader) -> [i64; 3] {
    // SAFETY: the caller's guarantee.
    unsafe { (*object.cast::<SliceObject>()).bounds }
}

/// Returns whether two slices are equal, as `==` asks.
///
/// # Safety
///
/// Both point to live slices.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn typy_rt_slice_equal(
    left: *mut SliceObject,
    right: *mut SliceObject,
) -> bool {
    // SAFETY: the caller's guarantee.
    unsafe { slice_equal(left.cast(), right.cast()) }
}

/// Returns whether two slices are equal: each bound is `None` in both or equal in both, as
/// CPython compares the tuples of their bounds.
unsafe extern "C" fn slice_equal(left: *mut ObjectHeader, right: *mut ObjectHeader) -> bool {
    // SAFETY: the types of the objects say they are slices, whose words are live.
    unsafe {
        bounds(left).iter().zip(bounds(right)).all(|(left, right)| {
            if *left == 0 || right == 0 {
                *left == right
            } else {
                typy_rt_int_compare(*left, right, word(0))
            }
        })
    }
}

/// Hashes a slice as CPython 3.12 and later do: the xxHash-based combination of the hashes
/// of its bounds, without the length a tuple's hash adds.
unsafe extern "C" fn slice_hash(object: *mut ObjectHeader) -> i64 {
    const PRIME_1: u64 = 11_400_714_785_074_694_791;
    const PRIME_2: u64 = 14_029_467_366_897_019_727;
    const PRIME_5: u64 = 2_870_177_450_012_600_261;
    // SAFETY: the type of the object says it is a slice, whose words are live.
    let lanes = unsafe { bounds(object) }.map(|bound| {
        if bound == 0 {
            crate::hash::NONE
        } else {
            // SAFETY: the word is an `int` word whose large integer is live.
            unsafe { crate::hash::int(bound) }
        }
    });
    let mut accumulator = PRIME_5;
    for lane in lanes {
        accumulator = accumulator.wrapping_add(lane.cast_unsigned().wrapping_mul(PRIME_2));
        accumulator = accumulator.rotate_left(31);
        accumulator = accumulator.wrapping_mul(PRIME_1);
    }
    if accumulator == u64::MAX {
        return 1_546_275_796;
    }
    accumulator.cast_signed()
}

/// Writes a slice as CPython does: `slice(1, 2, None)`.
unsafe extern "C" fn slice_repr(object: *mut ObjectHeader) -> Raising<*mut StrObject> {
    let mut message = crate::message::MessageBuffer::new();
    message.push_str("slice(");
    // SAFETY: the type of the object says it is a slice, whose words are live.
    for (index, bound) in unsafe { bounds(object) }.into_iter().enumerate() {
        if index > 0 {
            message.push_str(", ");
        }
        if bound == 0 {
            message.push_str("None");
            continue;
        }
        // SAFETY: the word is an `int` word whose large integer is live.
        let text = unsafe { crate::int::typy_rt_int_to_str(bound) };
        if !text.exception.is_null() {
            return Raising::error(core::ptr::null_mut(), text.exception);
        }
        // SAFETY: the text is a new string, released once it is copied.
        unsafe {
            message.push_text(text.value);
            crate::refcount::typy_rt_decrement(text.value.cast());
        }
    }
    message.push_str(")");
    Raising::ok(message.text())
}

/// Releases a slice's words and frees it.
///
/// # Safety
///
/// `slice` points to a slice whose last reference is gone.
pub(crate) unsafe fn destroy(slice: *mut SliceObject) {
    // SAFETY: the caller's guarantee; the slice owns its words.
    unsafe {
        for bound in (*slice).bounds {
            typy_rt_int_decrement(bound);
        }
    }
    crate::allocator::free(slice.cast());
}

#[cfg(test)]
mod tests {
    use super::{
        SliceObject, slice_equal, slice_hash, slice_repr, typy_rt_slice_bound, typy_rt_slice_index,
        typy_rt_slice_new,
    };
    use crate::word::{inline, inline_value};

    fn word(value: i64) -> i64 {
        inline(value).unwrap()
    }

    fn shown(slice: *mut SliceObject) -> String {
        // SAFETY: the slice is live, and the text is new and released here.
        unsafe {
            let text = slice_repr(slice.cast()).value;
            let shown = (0..crate::string::length(text))
                .map(|index| char::from_u32(crate::string::code_point(text, index)).unwrap())
                .collect();
            crate::refcount::typy_rt_decrement(text.cast());
            shown
        }
    }

    #[test]
    fn a_slice_is_written_compared_and_hashed_as_cpython_does() {
        // SAFETY: the words are inline, and every slice made here is released here.
        unsafe {
            let one = typy_rt_slice_new(word(1), word(2), 0);
            let same = typy_rt_slice_new(word(1), word(2), 0);
            let other = typy_rt_slice_new(0, word(5), 0);
            assert_eq!(shown(one), "slice(1, 2, None)");
            assert!(slice_equal(one.cast(), same.cast()));
            assert!(!slice_equal(one.cast(), other.cast()));
            // `hash(slice(None, 5, None))` in CPython 3.12 to 3.15.
            assert_eq!(slice_hash(other.cast()), 1_765_991_802_173_826_621);
            // These bounds bring the sum to -1, which CPython gives as 1546275796.
            let summed = typy_rt_slice_new(word(6), word(2), word(737_508_419_540_398_855));
            assert_eq!(slice_hash(summed.cast()), 1_546_275_796);
            crate::refcount::typy_rt_decrement(summed.cast());
            assert_eq!(inline_value(typy_rt_slice_bound(one, word(1))), 2);
            assert_eq!(typy_rt_slice_bound(one, word(2)), 0);
            for slice in [one, same, other] {
                crate::refcount::typy_rt_decrement(slice.cast());
            }
        }
    }

    #[test]
    fn indices_adjust_the_bounds_to_a_length_and_refuse_a_bad_one() {
        // SAFETY: the words are inline, and every object made here is released here.
        unsafe {
            let backwards = typy_rt_slice_new(word(-1), 0, word(-2));
            let parts: Vec<i64> = (0..3)
                .map(|part| inline_value(typy_rt_slice_index(backwards, word(5), word(part)).value))
                .collect();
            assert_eq!(parts, [4, -1, -2]);
            let error = typy_rt_slice_index(backwards, word(-1), word(0)).exception;
            assert!(!error.is_null());
            crate::refcount::typy_rt_decrement(error.cast());
            let zero = typy_rt_slice_new(0, 0, word(0));
            let error = typy_rt_slice_index(zero, word(5), word(0)).exception;
            assert!(!error.is_null());
            crate::refcount::typy_rt_decrement(error.cast());
            for slice in [backwards, zero] {
                crate::refcount::typy_rt_decrement(slice.cast());
            }
        }
    }

    #[test]
    fn indices_give_a_missing_step_as_one_and_refuse_a_large_negative_length() {
        let text = crate::string::from_str(&format!("-{}", "9".repeat(30)));
        // SAFETY: the words are live, and every object made here is released here; the base
        // is the `int` word of 10.
        unsafe {
            let negative = crate::int::typy_rt_int_from_str(text, word(10)).value;
            crate::refcount::typy_rt_decrement(text.cast());
            let forwards = typy_rt_slice_new(word(1), word(3), 0);
            let parts: Vec<i64> = (0..3)
                .map(|part| inline_value(typy_rt_slice_index(forwards, word(5), word(part)).value))
                .collect();
            assert_eq!(parts, [1, 3, 1]);
            let error = typy_rt_slice_index(forwards, negative, word(0)).exception;
            assert_eq!((*error).class().name(), "ValueError");
            crate::refcount::typy_rt_decrement(error.cast());
            crate::refcount::typy_rt_int_decrement(negative);
            crate::refcount::typy_rt_decrement(forwards.cast());
        }
    }

    /// Returns the decimal text of an `int` word, which is released.
    fn decimal(value: i64) -> String {
        // SAFETY: the word is live; it and the text are released here.
        unsafe {
            let text = crate::int::typy_rt_int_to_str(value).value;
            crate::refcount::typy_rt_int_decrement(value);
            let shown = (0..crate::string::length(text))
                .map(|index| char::from_u32(crate::string::code_point(text, index)).unwrap())
                .collect();
            crate::refcount::typy_rt_decrement(text.cast());
            shown
        }
    }

    /// Returns the `int` word of the decimal `text`, which the caller releases.
    fn number(text: &str) -> i64 {
        let string = crate::string::from_str(text);
        // SAFETY: the text is live and released here; the base is the `int` word of 10.
        unsafe {
            let value = crate::int::typy_rt_int_from_str(string, word(10)).value;
            crate::refcount::typy_rt_decrement(string.cast());
            value
        }
    }

    #[test]
    #[cfg_attr(
        miri,
        ignore = "building a 4300-digit integer takes hours under Miri; the same code runs in the other tests"
    )]
    fn a_bound_of_more_than_4300_digits_is_not_written() {
        let mut big = crate::int::value::Integer::from_i128(1);
        let ten = crate::int::value::Integer::from_i128(10);
        for _ in 0..4300 {
            big = big.multiply(&ten);
        }
        let bound = big.into_word();
        // SAFETY: the slice takes its own reference to the word; the word, the slice and the
        // exception are released here.
        unsafe {
            let slice = typy_rt_slice_new(bound, 0, 0);
            crate::refcount::typy_rt_int_decrement(bound);
            let written = slice_repr(slice.cast());
            assert!(written.value.is_null());
            let error = written.exception;
            assert_eq!((*error).class().name(), "ValueError");
            crate::refcount::typy_rt_decrement(error.cast());
            crate::refcount::typy_rt_decrement(slice.cast());
        }
    }

    #[test]
    fn indices_hold_each_bound_inside_a_length_past_an_inline_int() {
        let length = "9".repeat(30);
        let below = format!("-{}", "9".repeat(31));
        // The bounds of each slice, with the start, the stop and the step CPython gives for
        // `length`.
        let cases = [
            ((Some("1"), Some("3"), None), ("1", "3", "1")),
            (
                (None, None, Some("-1")),
                ("999999999999999999999999999998", "-1", "-1"),
            ),
            (
                (Some("-5"), None, None),
                (
                    "999999999999999999999999999994",
                    "999999999999999999999999999999",
                    "1",
                ),
            ),
            (
                (Some(length.as_str()), Some(below.as_str()), Some("-2")),
                ("999999999999999999999999999998", "-1", "-2"),
            ),
        ];
        for ((start, stop, step), (from, to, by)) in cases {
            let bound = |text: Option<&str>| text.map_or(0, number);
            let given = number(&length);
            // SAFETY: every word is live, and every object made here is released here.
            unsafe {
                let slice = typy_rt_slice_new(bound(start), bound(stop), bound(step));
                let found = (
                    decimal(typy_rt_slice_index(slice, given, word(0)).value),
                    decimal(typy_rt_slice_index(slice, given, word(1)).value),
                    decimal(typy_rt_slice_index(slice, given, word(2)).value),
                );
                assert_eq!(found, (from.to_owned(), to.to_owned(), by.to_owned()));
                crate::refcount::typy_rt_int_decrement(given);
                crate::refcount::typy_rt_decrement(slice.cast());
            }
        }
        let given = number(&length);
        // SAFETY: as above.
        unsafe {
            let still = typy_rt_slice_new(0, 0, word(0));
            let error = typy_rt_slice_index(still, given, word(0)).exception;
            assert_eq!((*error).class().name(), "ValueError");
            crate::refcount::typy_rt_decrement(error.cast());
            crate::refcount::typy_rt_int_decrement(given);
            crate::refcount::typy_rt_decrement(still.cast());
        }
    }
}
