//! `tuple` items stored inline in a list (DAT-08, ADR-0001, ADR-0009). The compiler emits a
//! descriptor for each tuple type a list holds: the size of an item, the number of items of the
//! tuple, and where each item lies. The functions here count, compare and write a tuple from
//! its descriptor, so the compiler emits data and no code.

use crate::Raising;
use crate::list::{ItemKind, ItemType};
use crate::string::StrObject;

/// How a tuple type is laid out, as the compiler emits it.
#[repr(C)]
#[derive(Debug)]
pub struct TupleType {
    /// The bytes an item takes, a multiple of eight and at least eight.
    pub size: u64,
    /// The number of items of the tuple, `None` items included.
    pub length: u64,
    /// The items, `length` of them.
    pub fields: *const Field,
}

/// One item of a tuple type.
#[repr(C)]
#[derive(Debug)]
pub struct Field {
    /// The kind, as an `ItemKind` code. A `bool` field is one byte; `None` takes no bytes.
    pub kind: u32,
    /// The byte offset from the start of the tuple.
    pub offset: u32,
    /// The descriptor of a nested tuple, or null.
    pub tuple: *const TupleType,
}

impl TupleType {
    /// Returns the number of eight-byte words an item takes.
    pub(crate) fn words(&self) -> usize {
        usize::try_from(self.size / 8).unwrap_or(1).max(1)
    }

    /// Returns the fields.
    ///
    /// # Safety
    ///
    /// The descriptor is one the compiler emitted, whose `fields` holds `length` entries.
    unsafe fn fields(&self) -> &[Field] {
        if self.length == 0 {
            return &[];
        }
        // SAFETY: the caller's guarantee.
        unsafe {
            core::slice::from_raw_parts(self.fields, usize::try_from(self.length).unwrap_or(0))
        }
    }
}

impl Field {
    fn item_type(&self) -> ItemType {
        ItemType::of(self.kind, self.tuple)
    }

    /// Returns the field of the tuple at `address` as an item: its eight bytes, a `bool` or a
    /// `bool | None` widened from its byte, 0 for `None`, or for a nested tuple, optional or
    /// not, its address.
    ///
    /// # Safety
    ///
    /// `address` is the address of a live tuple this field belongs to.
    unsafe fn value(&self, address: u64) -> u64 {
        let at = address + u64::from(self.offset);
        let pointer = core::ptr::with_exposed_provenance::<u8>(usize::try_from(at).unwrap_or(0));
        // SAFETY: the field lies inside the caller's live tuple, at its own alignment.
        unsafe {
            match ItemKind::from_code(self.kind) {
                ItemKind::Bool | ItemKind::OptionalBool => u64::from(*pointer),
                ItemKind::NoneValue => 0,
                ItemKind::Tuple | ItemKind::OptionalTuple => at,
                #[allow(
                    clippy::cast_ptr_alignment,
                    reason = "the compiler places an eight-byte field at an offset that is a multiple of eight"
                )]
                _ => *pointer.cast::<u64>(),
            }
        }
    }
}

/// Returns item `index` of the tuple at `address`, as `Field::value` gives it.
///
/// # Safety
///
/// As for `retain`, and the tuple has more than `index` items.
pub(crate) unsafe fn field(tuple: *const TupleType, address: u64, index: usize) -> u64 {
    // SAFETY: the caller's guarantee.
    unsafe { (*tuple).fields()[index].value(address) }
}

/// Returns the descriptor of item `index` of a tuple type, a tuple itself.
///
/// # Safety
///
/// `tuple` is a descriptor the compiler emitted whose item `index` is a tuple.
pub(crate) unsafe fn field_tuple(tuple: *const TupleType, index: usize) -> *const TupleType {
    // SAFETY: the caller's guarantee.
    unsafe { (*tuple).fields()[index].tuple }
}

/// Adds a reference to every object the tuple at `address` holds.
///
/// # Safety
///
/// `tuple` is a descriptor the compiler emitted, and `address` the address of a live tuple of
/// that layout.
pub(crate) unsafe fn retain(tuple: *const TupleType, address: u64) {
    // SAFETY: the caller's guarantee covers each field.
    unsafe {
        for field in (*tuple).fields() {
            field.item_type().retain(field.value(address));
        }
    }
}

/// Drops a reference to every object the tuple at `address` holds.
///
/// # Safety
///
/// As for `retain`, and the caller holds the references.
pub(crate) unsafe fn release(tuple: *const TupleType, address: u64) {
    // SAFETY: the caller's guarantee covers each field.
    unsafe {
        for field in (*tuple).fields() {
            field.item_type().release(field.value(address));
        }
    }
}

/// Returns the hash of the tuple at `address`, from the hashes of its items as CPython
/// combines them.
///
/// # Safety
///
/// As for `retain`.
pub(crate) unsafe fn hash(tuple: *const TupleType, address: u64) -> i64 {
    // SAFETY: the caller's guarantee covers each field.
    unsafe {
        let fields = (*tuple).fields();
        let hashes = fields
            .iter()
            .map(|field| field.item_type().hash(field.value(address)));
        crate::hash::tuple(hashes, fields.len())
    }
}

/// Returns `left == right` for two tuples of one layout: every item equal.
///
/// # Safety
///
/// `tuple` is a descriptor the compiler emitted, and both are addresses of live tuples of it.
pub(crate) unsafe fn equal(tuple: *const TupleType, left: u64, right: u64) -> bool {
    // SAFETY: the caller's guarantee covers each field.
    unsafe {
        (*tuple).fields().iter().all(|field| {
            field
                .item_type()
                .equal(field.value(left), field.value(right))
        })
    }
}

/// Returns whether two tuples of one layout stand in `comparison`, an `int` word, as CPython
/// compares tuples: the first pair of items that are not equal decides, and tuples whose items
/// are all equal are equal.
///
/// # Safety
///
/// As for `equal`.
pub(crate) unsafe fn order(
    tuple: *const TupleType,
    left: u64,
    right: u64,
    comparison: i64,
) -> bool {
    // SAFETY: the caller's guarantee covers each field.
    unsafe {
        for field in (*tuple).fields() {
            let item = field.item_type();
            let (first, second) = (field.value(left), field.value(right));
            if item.equal(first, second) {
                continue;
            }
            return match crate::word::inline_value(comparison) {
                0 => false,
                1 => true,
                _ => item.order(first, second, comparison),
            };
        }
        crate::compare::holds(core::cmp::Ordering::Equal, comparison)
    }
}

/// Returns `repr(tuple)`: the `repr` of each item between parentheses, with a comma after the
/// only item of a one-item tuple, as `(1,)`.
///
/// # Safety
///
/// `tuple` is a descriptor the compiler emitted, and `address` that of a live tuple of it.
pub(crate) unsafe fn repr(tuple: *const TupleType, address: u64) -> Raising<*mut StrObject> {
    // SAFETY: the caller's guarantee covers each field.
    unsafe {
        let fields = (*tuple).fields();
        let suffix = if fields.len() == 1 { ",)" } else { ")" };
        crate::list::join_reprs(
            fields
                .iter()
                .map(|field| (field.item_type(), field.value(address))),
            fields.len(),
            "(",
            suffix,
        )
    }
}

#[cfg(test)]
mod tests;
