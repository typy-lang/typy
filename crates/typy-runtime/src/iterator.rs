//! Iterators that `iter()` makes of the built-in iterables: a source and a position, stepped
//! with the source's own rules, as CPython's iterators are (CST-09, ADR-0014).

#[cfg(test)]
mod tests;

use crate::Raising;
use crate::dict::DictObject;
use crate::exception::{ExceptionKind, raise_bare};
use crate::header::{ITERATOR_TYPE_ID, ObjectHeader};
use crate::list::ListObject;
use crate::refcount::{typy_rt_decrement, typy_rt_increment};
use crate::set::SetObject;
use crate::string::{StrObject, code_point, length};
use crate::tuple::TupleType;
use crate::type_object::{OBJECT, TypeObject};
use crate::word::{inline, inline_value};

/// What an iterator reads, as the compiler gives it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum IteratorKind {
    /// A list.
    List = 0,
    /// A `tuple[T, ...]`.
    Tuple = 1,
    /// A `str` with a code point outside ASCII.
    Str = 2,
    /// A `bytes`.
    Bytes = 3,
    /// A set.
    Set = 4,
    /// A dict's keys.
    DictKeys = 5,
    /// A dict's values.
    DictValues = 6,
    /// A dict's items.
    DictItems = 7,
    /// A `str` of ASCII code points only, which CPython iterates with a type of its own.
    StrAscii = 8,
}

impl IteratorKind {
    /// Returns the kind with this code; an unknown code is a defect in the compiler.
    fn from_code(code: i64) -> Self {
        match code {
            0 => Self::List,
            1 => Self::Tuple,
            2 => Self::Str,
            3 => Self::Bytes,
            4 => Self::Set,
            5 => Self::DictKeys,
            6 => Self::DictValues,
            7 => Self::DictItems,
            8 => Self::StrAscii,
            _ => crate::libc::abort_with(b"typy runtime: an iterator has an unknown kind\n"),
        }
    }
}

/// An iterator: a heap object with the iterator type ID.
#[repr(C)]
#[derive(Debug)]
pub(crate) struct IteratorObject {
    header: ObjectHeader,
    kind: IteratorKind,
    /// The source, with a reference of its own; null once the iterator is exhausted, as CPython
    /// drops its source then.
    source: *mut ObjectHeader,
    /// The next index, slot or entry to read.
    position: usize,
    /// The number of items of a set or dict when the iterator was made.
    length: usize,
    /// How many entries of a dict the iterator has given.
    visited: usize,
    /// The current item, borrowed from the source or from the iterator.
    current: u64,
    /// The current item of a `str`, a string of one code point the iterator owns.
    held: *mut StrObject,
    /// The layout of a dict's items, and room for the current one.
    pair_type: *const TupleType,
    pair: *mut u8,
}

/// Returns a type of iterator with CPython's name: hashed by identity and written as
/// `<name object at 0x...>`.
const fn iterator_type(name: &'static str) -> TypeObject {
    TypeObject {
        hash: Some(crate::instance::typy_rt_identity_hash),
        repr: Some(iterator_repr),
        ..TypeObject::new(name, &raw const OBJECT)
    }
}

static LIST_ITERATOR: TypeObject = iterator_type("list_iterator");
static TUPLE_ITERATOR: TypeObject = iterator_type("tuple_iterator");
static STR_ITERATOR: TypeObject = iterator_type("str_iterator");
static STR_ASCII_ITERATOR: TypeObject = iterator_type("str_ascii_iterator");
static BYTES_ITERATOR: TypeObject = iterator_type("bytes_iterator");
static SET_ITERATOR: TypeObject = iterator_type("set_iterator");
static DICT_KEY_ITERATOR: TypeObject = iterator_type("dict_keyiterator");
static DICT_VALUE_ITERATOR: TypeObject = iterator_type("dict_valueiterator");
static DICT_ITEM_ITERATOR: TypeObject = iterator_type("dict_itemiterator");

/// Returns the type of an iterator.
///
/// # Safety
///
/// `iterator` points to a live iterator.
pub(crate) unsafe fn type_of(iterator: *const IteratorObject) -> &'static TypeObject {
    // SAFETY: the caller's guarantee.
    match unsafe { (*iterator).kind } {
        IteratorKind::List => &LIST_ITERATOR,
        IteratorKind::Tuple => &TUPLE_ITERATOR,
        IteratorKind::Str => &STR_ITERATOR,
        IteratorKind::StrAscii => &STR_ASCII_ITERATOR,
        IteratorKind::Bytes => &BYTES_ITERATOR,
        IteratorKind::Set => &SET_ITERATOR,
        IteratorKind::DictKeys => &DICT_KEY_ITERATOR,
        IteratorKind::DictValues => &DICT_VALUE_ITERATOR,
        IteratorKind::DictItems => &DICT_ITEM_ITERATOR,
    }
}

/// Writes an iterator as CPython's `object.__repr__` does.
unsafe extern "C" fn iterator_repr(object: *mut ObjectHeader) -> Raising<*mut StrObject> {
    // SAFETY: the type of the object says it is an iterator.
    let ty = unsafe { type_of(object.cast()) };
    Raising::ok(crate::instance::object_text(object, "", ty.name()))
}

/// Returns a new iterator of `kind` over `source`, which it takes a reference to. A dict's
/// items come with `pair_type`, the layout of the tuple of a key and a value, which the
/// iterator builds each item in; it is null for any other kind. A `str` of ASCII code points
/// only makes CPython's `str_ascii_iterator`.
///
/// # Safety
///
/// `source` points to a live object of the kind's type: a list or tuple, a `str`, a `bytes`, a
/// set or a dict, and `kind` is an `int` word of an `IteratorKind`.
#[allow(
    clippy::cast_ptr_alignment,
    reason = "every object comes from malloc, aligned for every type, and starts with its header"
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn typy_rt_iterator_new(
    source: *mut ObjectHeader,
    kind: i64,
    pair_type: *const TupleType,
) -> *mut IteratorObject {
    let mut kind = IteratorKind::from_code(inline_value(kind));
    // SAFETY: the caller's guarantee for each kind.
    let length = unsafe {
        typy_rt_increment(source);
        match kind {
            IteratorKind::Set => (*source.cast::<SetObject>()).used,
            IteratorKind::DictKeys | IteratorKind::DictValues | IteratorKind::DictItems => {
                (*source.cast::<DictObject>()).used
            }
            IteratorKind::Str => {
                let text = source.cast::<StrObject>();
                if (0..length(text)).all(|index| code_point(text, index) < 0x80) {
                    kind = IteratorKind::StrAscii;
                }
                0
            }
            _ => 0,
        }
    };
    let pair = if pair_type.is_null() {
        core::ptr::null_mut()
    } else {
        // SAFETY: the compiler gives the descriptor of the pair.
        crate::allocator::allocate(usize::try_from(unsafe { (*pair_type).size }).unwrap_or(8))
    };
    #[allow(
        clippy::cast_ptr_alignment,
        reason = "malloc returns memory aligned for every type"
    )]
    let iterator =
        crate::allocator::allocate(core::mem::size_of::<IteratorObject>()).cast::<IteratorObject>();
    // SAFETY: the allocation holds an iterator.
    unsafe {
        iterator.write(IteratorObject {
            header: ObjectHeader::new(ITERATOR_TYPE_ID),
            kind,
            source,
            position: 0,
            length,
            visited: 0,
            current: 0,
            held: core::ptr::null_mut(),
            pair_type,
            pair,
        });
    }
    iterator
}

/// Moves an iterator to its next item and returns true, or returns false when it has none:
/// then it drops its source, and raises `StopIteration` when `stop` is the `int` word of 1, as
/// `next()` without a default does. A set or dict that changed size raises `RuntimeError`.
///
/// # Safety
///
/// `iterator` points to a live iterator, and `stop` is an `int` word.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn typy_rt_iterator_advance(
    iterator: *mut IteratorObject,
    stop: i64,
) -> Raising<bool> {
    // SAFETY: the caller's guarantee; the iterator owns a reference to its source.
    let stepped = unsafe { step(&mut *iterator) };
    match stepped {
        Err(error) => Raising::error(false, error),
        Ok(true) => Raising::ok(true),
        Ok(false) => {
            // SAFETY: the iterator is live.
            unsafe { drop_source(&mut *iterator) };
            if inline_value(stop) == 1 {
                Raising::error(false, raise_bare(ExceptionKind::StopIteration))
            } else {
                Raising::ok(false)
            }
        }
    }
}

/// Returns the item `typy_rt_iterator_advance` moved to, borrowed until the next step.
///
/// # Safety
///
/// `iterator` points to a live iterator whose last step found an item.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn typy_rt_iterator_item(iterator: *const IteratorObject) -> u64 {
    // SAFETY: the caller's guarantee.
    unsafe { (*iterator).current }
}

/// Steps the iterator, and returns whether it found an item, or the exception its source's
/// rules raise.
///
/// # Safety
///
/// The iterator is live, and its source is null or live.
#[allow(
    clippy::cast_ptr_alignment,
    reason = "every object comes from malloc, aligned for every type, and starts with its header"
)]
unsafe fn step(iterator: &mut IteratorObject) -> Result<bool, *mut crate::ExceptionObject> {
    let source = iterator.source;
    if source.is_null() {
        return Ok(false);
    }
    let position = iterator.position;
    // SAFETY: the source is live and of the kind's type.
    unsafe {
        match iterator.kind {
            IteratorKind::List | IteratorKind::Tuple => {
                let list = &*source.cast::<ListObject>();
                if position >= list.length {
                    return Ok(false);
                }
                iterator.current = list.item(position);
            }
            IteratorKind::Str | IteratorKind::StrAscii | IteratorKind::Bytes => {
                let text = source.cast::<StrObject>();
                if position >= length(text) {
                    return Ok(false);
                }
                let point = code_point(text, position);
                if iterator.kind == IteratorKind::Bytes {
                    iterator.current = inline(i64::from(point)).unwrap_or(1).cast_unsigned();
                } else {
                    let character =
                        crate::string::typy_rt_chr(inline(i64::from(point)).unwrap_or(1)).value;
                    let old = core::mem::replace(&mut iterator.held, character);
                    if !old.is_null() {
                        typy_rt_decrement(old.cast());
                    }
                    iterator.current = character.expose_provenance() as u64;
                }
            }
            IteratorKind::Set => {
                let set = source.cast::<SetObject>();
                let found =
                    crate::set::typy_rt_set_next(set, word(position), word(iterator.length));
                if !found.exception.is_null() {
                    return Err(found.exception);
                }
                let Ok(slot) = usize::try_from(inline_value(found.value)) else {
                    return Ok(false);
                };
                iterator.current = crate::set::typy_rt_set_item(set, found.value);
                iterator.position = slot + 1;
                return Ok(true);
            }
            IteratorKind::DictKeys | IteratorKind::DictValues | IteratorKind::DictItems => {
                return dict_step(iterator, source.cast());
            }
        }
    }
    iterator.position = position + 1;
    Ok(true)
}

/// Steps an iterator of a dict's keys, values or items, as CPython's dict iterator does.
///
/// # Safety
///
/// The iterator is live and `dict` is its live source.
unsafe fn dict_step(
    iterator: &mut IteratorObject,
    dict: *const DictObject,
) -> Result<bool, *mut crate::ExceptionObject> {
    // SAFETY: the caller's guarantee; the entry found holds an item.
    unsafe {
        let found = crate::dict::typy_rt_dict_next(
            dict,
            word(iterator.position),
            word(iterator.length),
            word(iterator.visited),
        );
        if !found.exception.is_null() {
            return Err(found.exception);
        }
        let Ok(entry) = usize::try_from(inline_value(found.value)) else {
            return Ok(false);
        };
        let key = crate::dict::typy_rt_dict_key(dict, found.value);
        let value = crate::dict::typy_rt_dict_value(dict, found.value);
        iterator.current = match iterator.kind {
            IteratorKind::DictKeys => key,
            IteratorKind::DictValues => value,
            _ => {
                let address = iterator.pair.expose_provenance() as u64;
                crate::tuple::store_field(iterator.pair_type, address, 0, key);
                crate::tuple::store_field(iterator.pair_type, address, 1, value);
                address
            }
        };
        iterator.position = entry + 1;
        iterator.visited += 1;
    }
    Ok(true)
}

/// Returns the `int` word of a count or a position.
fn word(value: usize) -> i64 {
    inline(i64::try_from(value).unwrap_or(i64::MAX)).unwrap_or(1)
}

/// Drops an exhausted iterator's source, as CPython's iterators do, so it gives nothing more.
///
/// # Safety
///
/// The iterator is live.
unsafe fn drop_source(iterator: &mut IteratorObject) {
    let source = core::mem::replace(&mut iterator.source, core::ptr::null_mut());
    if !source.is_null() {
        // SAFETY: the iterator held a reference to its source.
        unsafe { typy_rt_decrement(source) };
    }
}

/// Frees an iterator whose last reference is gone, with its source and current string.
///
/// # Safety
///
/// `iterator` points to a live iterator that nothing else refers to.
pub(crate) unsafe fn destroy(iterator: *mut IteratorObject) {
    // SAFETY: the caller's guarantee.
    unsafe {
        let object = &mut *iterator;
        drop_source(object);
        if !object.held.is_null() {
            typy_rt_decrement(object.held.cast());
        }
        if !object.pair.is_null() {
            crate::allocator::free(object.pair);
        }
    }
    crate::allocator::free(iterator.cast());
}
