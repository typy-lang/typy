//! Exceptions a call makes with `*` and `**` (EXP-22, ADR-0083). The number of values `*`
//! gives is known only when the call runs, so the runtime lays out the arguments itself: one
//! field per value, in the block that holds the values, which the layout follows, so freeing
//! the arguments frees their layout too. `args` of such an exception is the member
//! `tuple[T, ...]` of the union of tuples the program reads, and an exception class takes no
//! keywords, so the dict `**` gives must be empty.

use super::part::{ARGUMENTS, Part, part_of};
use super::{ExceptionObject, new};
use crate::allocator::allocate;
use crate::dict::DictObject;
use crate::exception::ExceptionKind;
use crate::list::{ItemType, ListObject};
use crate::message::MessageBuffer;
use crate::tuple::{Field, TupleType};
use crate::type_object::TypeObject;

/// The layout of no arguments.
static NO_ARGUMENTS: TupleType = TupleType {
    size: 8,
    length: 0,
    fields: core::ptr::null(),
};

/// Returns a new exception of `class`, not yet raised, whose arguments are the items of
/// `values`, each with a reference of its own; the caller keeps its list.
///
/// # Safety
///
/// `class` is an exception class, and `values` points to a live list.
#[allow(
    clippy::cast_ptr_alignment,
    reason = "malloc returns memory aligned for every type, and the values take a multiple of eight bytes"
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn typy_rt_exception_from_list(
    class: *const TypeObject,
    values: *const ListObject,
) -> *mut ExceptionObject {
    // SAFETY: the caller's guarantee.
    let (class, values) = unsafe { (&*class, &*values) };
    if values.length == 0 {
        return new(
            class,
            core::ptr::null_mut(),
            &raw const NO_ARGUMENTS,
            core::ptr::null_mut(),
        );
    }
    let item = values.words() * 8;
    let size = values.length * item;
    // A field's offset is 32 bits, as the compiler lays out every tuple.
    if u32::try_from(size).is_err() {
        crate::libc::abort_with(b"typy runtime: the arguments of an exception exceed 4 GiB\n");
    }
    let block = allocate(
        size + core::mem::size_of::<TupleType>() + values.length * core::mem::size_of::<Field>(),
    );
    // SAFETY: the block holds the values, then the layout, then its fields; the list's items
    // are live, and the copy takes its own references to them.
    unsafe {
        core::ptr::copy_nonoverlapping(values.items.cast::<u8>(), block, size);
        let layout = block.add(size).cast::<TupleType>();
        let fields = layout.add(1).cast::<Field>();
        for index in 0..values.length {
            fields.add(index).write(Field {
                kind: values.kind,
                offset: u32::try_from(index * item).unwrap_or(u32::MAX),
                tuple: values.tuple,
            });
        }
        layout.write(TupleType {
            size: u64::try_from(size).unwrap_or(u64::MAX),
            length: u64::try_from(values.length).unwrap_or(u64::MAX),
            fields,
        });
        crate::tuple::retain(layout, block.expose_provenance() as u64);
        new(class, core::ptr::null_mut(), layout, block)
    }
}

/// Returns `TypeError` for a call of the exception class `class` whose `**` gave keywords,
/// `ValueError() takes no keyword arguments`, as CPython's `BaseException.__init__` raises it;
/// null when `keywords` is empty.
///
/// # Safety
///
/// `class` is a live class, and `keywords` points to a live dict.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn typy_rt_exception_keywords(
    class: *const TypeObject,
    keywords: *const DictObject,
) -> *mut ExceptionObject {
    // SAFETY: the caller's guarantee.
    let (class, keywords) = unsafe { (&*class, &*keywords) };
    if keywords.used == 0 {
        return core::ptr::null_mut();
    }
    let mut message = MessageBuffer::new();
    message.push_str(class.name());
    message.push_str("() takes no keyword arguments");
    message.raise(ExceptionKind::TypeError)
}

/// Returns true when every item of `args` of `exception` is held as the items of a list of
/// `kind` and `tuple` are, so `args` reads as the member `tuple[T, ...]` of its union.
///
/// # Safety
///
/// `exception` points to a live exception, `kind` is the `int` word of an item kind, and
/// `tuple` is the descriptor of tuple items, or 0.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn typy_rt_exception_arguments_all(
    exception: *const ExceptionObject,
    kind: i64,
    tuple: u64,
) -> bool {
    let wanted = item_type(kind, tuple);
    // SAFETY: the caller's guarantee; a nested descriptor lives as long as its tuple.
    unsafe {
        match part_of(&*exception, ARGUMENTS) {
            Part::One(ty, _) => held_alike(ty, wanted),
            Part::Tuple(arguments) => {
                (0..usize::try_from(arguments.count()).unwrap_or(0)).all(|index| {
                    held_alike(crate::tuple::field_type(arguments.layout(), index), wanted)
                })
            }
            Part::Empty | Part::Nothing => true,
        }
    }
}

/// Returns `args` of `exception` as a new `tuple[T, ...]` of items of `kind` and `tuple`, each
/// with a reference of its own.
///
/// # Safety
///
/// As for `typy_rt_exception_arguments_all`, which is true for `exception`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn typy_rt_exception_arguments_tuple(
    exception: *const ExceptionObject,
    kind: i64,
    tuple: u64,
) -> *mut ListObject {
    // SAFETY: the caller's guarantee; each item is live, and the list has room for them all.
    unsafe {
        let part = part_of(&*exception, ARGUMENTS);
        let count = match &part {
            Part::One(..) => 1,
            Part::Tuple(arguments) => usize::try_from(arguments.count()).unwrap_or(0),
            Part::Empty | Part::Nothing => 0,
        };
        let list = crate::list::new_list(item_type(kind, tuple), count);
        match part {
            Part::One(_, value) => crate::list::push_retained(list, value),
            Part::Tuple(arguments) => {
                for index in 0..count {
                    let value = crate::tuple::field(arguments.layout(), arguments.address, index);
                    crate::list::push_retained(list, value);
                }
            }
            Part::Empty | Part::Nothing => {}
        }
        crate::list::typy_rt_tuple_seal(list);
        list
    }
}

/// Returns the item type a list of the kind the `int` word `kind` names holds, with the
/// descriptor at `tuple` for tuple items.
fn item_type(kind: i64, tuple: u64) -> ItemType {
    ItemType::of(
        u32::try_from(crate::word::inline_value(kind)).unwrap_or(u32::MAX),
        core::ptr::with_exposed_provenance(usize::try_from(tuple).unwrap_or(0)),
    )
}

/// Returns true when items of `held` are stored as items of `wanted` are: one kind, and for
/// tuples one layout.
///
/// # Safety
///
/// The descriptors of both are live.
unsafe fn held_alike(held: ItemType, wanted: ItemType) -> bool {
    let (left, right) = (held.descriptor(), wanted.descriptor());
    held.kind() == wanted.kind()
        && (left.is_null() && right.is_null()
            // SAFETY: the caller's guarantee.
            || !left.is_null() && !right.is_null() && unsafe { crate::tuple::same_layout(left, right) })
}

#[cfg(test)]
mod tests;
