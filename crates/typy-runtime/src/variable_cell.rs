//! Cells (FUN-04, ADR-0061): the value of a variable that a nested function captures and that
//! may change after the `def`, or have no value yet, which every function that captures it
//! shares. A cell holds the descriptor of its layout, the value then whether it is assigned,
//! which the compiler emits once for the type, and releases the value when it is freed.

use crate::exception::{ExceptionKind, ExceptionObject, raise_with_message};
use crate::header::{CELL_TYPE_ID, ObjectHeader};
use crate::string::StrObject;
use crate::tuple::TupleType;

/// A cell: the header, the descriptor of its layout, then the layout.
#[repr(C)]
#[derive(Debug)]
pub(crate) struct CellObject {
    header: ObjectHeader,
    layout: *const TupleType,
}

/// The bytes before a cell's layout: its header and its descriptor.
const PREFIX: usize = core::mem::size_of::<CellObject>();

/// Returns a new cell of `layout` with one reference and every byte of its layout zero: no
/// value, which holds no reference.
///
/// # Safety
///
/// `layout` is a descriptor the compiler emitted for a cell's layout.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn typy_rt_cell_new(layout: *const TupleType) -> *mut CellObject {
    // SAFETY: the caller's guarantee.
    let size = unsafe { (*layout).size };
    let length = PREFIX + usize::try_from(size).unwrap_or(0);
    #[allow(
        clippy::cast_ptr_alignment,
        reason = "malloc returns memory aligned for every type"
    )]
    let cell = crate::allocator::allocate(length).cast::<CellObject>();
    // SAFETY: the allocation holds `length` bytes, aligned for every type.
    unsafe {
        core::ptr::write_bytes(cell.cast::<u8>(), 0, length);
        cell.write(CellObject {
            header: ObjectHeader::new(CELL_TYPE_ID),
            layout,
        });
    }
    cell
}

/// Frees a cell after releasing its value, which is zero when it holds none.
///
/// # Safety
///
/// `cell` is a cell whose last reference is gone.
pub(crate) unsafe fn destroy(cell: *mut CellObject) {
    // SAFETY: the caller's guarantee; the descriptor lays out the value after the prefix.
    unsafe { crate::tuple::release((*cell).layout, (cell.expose_provenance() + PREFIX) as u64) };
    crate::allocator::free(cell.cast());
}

/// Returns `NameError` for the captured variable `name`, whose cell holds no value yet, with
/// the message CPython 3.12 to 3.15 give.
///
/// # Safety
///
/// `name` points to a live `str` object.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn typy_rt_free_variable_unbound(
    name: *const StrObject,
) -> *mut ExceptionObject {
    let mut text = crate::string::StrBuilder::with_capacity(96);
    crate::function::push_ascii(&mut text, "cannot access free variable '");
    // SAFETY: the caller's guarantee.
    unsafe { text.push_str(name) };
    crate::function::push_ascii(
        &mut text,
        "' where it is not associated with a value in enclosing scope",
    );
    raise_with_message(ExceptionKind::NameError, text.finish())
}

#[cfg(test)]
mod tests;
