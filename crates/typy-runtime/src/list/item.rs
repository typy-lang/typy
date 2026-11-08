//! The types of list items, and how each is stored, counted, compared and written as text. An
//! item of a fixed kind is eight bytes: an `int` word, the bits of a `float`, a `bool` as 0 or
//! 1, a pointer, or 0 for `None`. A tuple is stored inline, as many words as its descriptor
//! says, and crosses the runtime's interface as the address of its bytes (ADR-0001, ADR-0009).

use crate::Raising;
use crate::header::{LIST_TYPE_ID, ObjectHeader, STR_TYPE_ID};
use crate::string::StrObject;
use crate::tuple::TupleType;
use crate::word::inline;

/// How a list stores its items. The compiler passes the code of the kind it chooses.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u32)]
pub(crate) enum ItemKind {
    /// An `int` word: inline, or a pointer to a large integer.
    Int = 0,
    /// The bits of a `float`.
    Float = 1,
    /// A `bool`, 0 or 1.
    Bool = 2,
    /// A pointer to a `str` or a `list`.
    Object = 3,
    /// `None`, stored as 0.
    NoneValue = 4,
    /// A tuple, stored inline as its descriptor lays it out.
    Tuple = 5,
}

impl ItemKind {
    /// Returns the kind with this code; an unknown code is a defect in the compiler.
    pub(crate) fn from_code(code: u32) -> Self {
        match code {
            0 => Self::Int,
            1 => Self::Float,
            2 => Self::Bool,
            3 => Self::Object,
            4 => Self::NoneValue,
            5 => Self::Tuple,
            _ => crate::libc::abort_with(b"typy runtime: a list has an unknown item kind\n"),
        }
    }
}

/// The type of a list's items: a kind, and for a tuple its descriptor.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct ItemType {
    kind: ItemKind,
    tuple: *const TupleType,
}

impl From<ItemKind> for ItemType {
    fn from(kind: ItemKind) -> Self {
        Self {
            kind,
            tuple: core::ptr::null(),
        }
    }
}

impl ItemType {
    /// Returns the type of tuple items laid out by `tuple`.
    pub(crate) const fn tuple(tuple: *const TupleType) -> Self {
        Self {
            kind: ItemKind::Tuple,
            tuple,
        }
    }

    /// Returns the kind.
    pub(crate) const fn kind(self) -> ItemKind {
        self.kind
    }

    /// Returns the descriptor of tuple items, or null.
    pub(crate) const fn descriptor(self) -> *const TupleType {
        self.tuple
    }

    /// Returns the number of eight-byte words an item takes.
    pub(crate) fn words(self) -> usize {
        match self.kind {
            // SAFETY: a tuple item type always has a live descriptor, emitted by the compiler.
            ItemKind::Tuple => unsafe { (*self.tuple).words() },
            _ => 1,
        }
    }

    /// Returns the item stored at `slot`: its eight bytes, or for a tuple the slot's address.
    ///
    /// # Safety
    ///
    /// `slot` points to an item of this type.
    pub(crate) unsafe fn load(self, slot: *const u64) -> u64 {
        match self.kind {
            ItemKind::Tuple => slot.expose_provenance() as u64,
            // SAFETY: the caller passes a slot that holds an item.
            _ => unsafe { *slot },
        }
    }

    /// Writes `item` into `slot`, without counting a reference: its eight bytes, or for a
    /// tuple the words at the address `item` holds.
    ///
    /// # Safety
    ///
    /// `slot` has room for an item of this type, and `item` is one, which does not overlap it.
    pub(crate) unsafe fn store(self, slot: *mut u64, item: u64) {
        // SAFETY: the caller passes a slot with room for the item and a live item.
        unsafe {
            match self.kind {
                ItemKind::Tuple => {
                    core::ptr::copy_nonoverlapping(address(item), slot, self.words());
                }
                _ => *slot = item,
            }
        }
    }

    /// Adds a reference to every object an item holds.
    ///
    /// # Safety
    ///
    /// `item` is a live item of this type.
    pub(crate) unsafe fn retain(self, item: u64) {
        // SAFETY: the caller passes a live item; its bits are an `int` word, a pointer or an
        // address.
        unsafe {
            match self.kind {
                ItemKind::Int => crate::refcount::typy_rt_int_increment(item.cast_signed()),
                ItemKind::Object => crate::refcount::typy_rt_increment(pointer(item)),
                ItemKind::Tuple => crate::tuple::retain(self.tuple, item),
                ItemKind::Float | ItemKind::Bool | ItemKind::NoneValue => {}
            }
        }
    }

    /// Drops a reference to every object an item holds.
    ///
    /// # Safety
    ///
    /// `item` is a live item of this type that the caller holds references for.
    pub(crate) unsafe fn release(self, item: u64) {
        // SAFETY: the caller passes a live item it holds references for.
        unsafe {
            match self.kind {
                ItemKind::Int => crate::refcount::typy_rt_int_decrement(item.cast_signed()),
                ItemKind::Object => crate::refcount::typy_rt_decrement(pointer(item)),
                ItemKind::Tuple => crate::tuple::release(self.tuple, item),
                ItemKind::Float | ItemKind::Bool | ItemKind::NoneValue => {}
            }
        }
    }

    /// Returns `left == right` for two items of this type, as CPython compares them.
    #[allow(
        clippy::float_cmp,
        reason = "Python's `==` on floats is IEEE equality, under which `nan` is not equal to itself"
    )]
    ///
    /// # Safety
    ///
    /// Both are live items of this type.
    pub(crate) unsafe fn equal(self, left: u64, right: u64) -> bool {
        let equal_code = inline(0).unwrap_or(1);
        match self.kind {
            ItemKind::Bool => left == right,
            ItemKind::NoneValue => true,
            ItemKind::Float => f64::from_bits(left) == f64::from_bits(right),
            // SAFETY: the caller passes live `int` words.
            ItemKind::Int => unsafe {
                crate::int::typy_rt_int_compare(left.cast_signed(), right.cast_signed(), equal_code)
            },
            // SAFETY: the caller passes pointers to live objects of one type.
            ItemKind::Object => unsafe { objects_equal(pointer(left), pointer(right)) },
            // SAFETY: the caller passes the addresses of two live tuples of this layout.
            ItemKind::Tuple => unsafe { crate::tuple::equal(self.tuple, left, right) },
        }
    }

    /// Returns whether `left` and `right` stand in the comparison whose code is the `int` word
    /// `comparison`, as CPython orders two items of this type: numbers by value, and strings,
    /// lists and tuples item by item. `None` is not ordered; the checker refuses it.
    ///
    /// # Safety
    ///
    /// Both are live items of this type.
    #[allow(
        clippy::float_cmp,
        reason = "Python's comparisons on floats are IEEE comparisons, under which `nan` is unordered"
    )]
    pub(crate) unsafe fn order(self, left: u64, right: u64, comparison: i64) -> bool {
        match self.kind {
            ItemKind::Bool | ItemKind::NoneValue => {
                crate::compare::holds(left.cmp(&right), comparison)
            }
            ItemKind::Float => {
                let (left, right) = (f64::from_bits(left), f64::from_bits(right));
                match crate::word::inline_value(comparison) {
                    0 => left == right,
                    1 => left != right,
                    2 => left < right,
                    3 => left <= right,
                    4 => left > right,
                    _ => left >= right,
                }
            }
            // SAFETY: the caller passes live `int` words.
            ItemKind::Int => unsafe {
                crate::int::typy_rt_int_compare(left.cast_signed(), right.cast_signed(), comparison)
            },
            // SAFETY: the caller passes pointers to live objects of one type.
            ItemKind::Object => unsafe { objects_order(pointer(left), pointer(right), comparison) },
            // SAFETY: the caller passes the addresses of two live tuples of this layout.
            ItemKind::Tuple => unsafe { crate::tuple::order(self.tuple, left, right, comparison) },
        }
    }

    /// Returns the truth of an item, as `bool(item)` gives it: a number is true when it is not
    /// zero, a `str`, a list or a tuple when it is not empty, and `None` never.
    ///
    /// # Safety
    ///
    /// `item` is a live item of this type.
    pub(crate) unsafe fn is_true(self, item: u64) -> bool {
        match self.kind {
            ItemKind::Bool => item != 0,
            ItemKind::NoneValue => false,
            // A large integer is never zero, because zero is always inline.
            ItemKind::Int => item.cast_signed() != inline(0).unwrap_or(1),
            ItemKind::Float => f64::from_bits(item) != 0.0,
            // SAFETY: the caller passes a pointer to a live object.
            ItemKind::Object => unsafe { object_is_true(pointer(item)) },
            // SAFETY: a tuple item type always has a live descriptor.
            ItemKind::Tuple => unsafe { (*self.tuple).length != 0 },
        }
    }

    /// Returns `repr(item)`; only an `int` of more than 4300 digits raises.
    ///
    /// # Safety
    ///
    /// `item` is a live item of this type.
    pub(crate) unsafe fn repr(self, item: u64) -> Raising<*mut StrObject> {
        match self.kind {
            // SAFETY: the caller passes a live `int` word.
            ItemKind::Int => unsafe { crate::int::typy_rt_int_to_str(item.cast_signed()) },
            ItemKind::Float => {
                Raising::ok(crate::float::typy_rt_float_to_str(f64::from_bits(item)))
            }
            ItemKind::Bool => Raising::ok(crate::string::from_str(if item == 0 {
                "False"
            } else {
                "True"
            })),
            ItemKind::NoneValue => Raising::ok(crate::string::from_str("None")),
            // SAFETY: the caller passes a pointer to a live object.
            ItemKind::Object => unsafe { object_repr(pointer(item)) },
            // SAFETY: the caller passes the address of a live tuple of this layout.
            ItemKind::Tuple => unsafe { crate::tuple::repr(self.tuple, item) },
        }
    }
}

/// Returns the words an item address points to.
fn address(item: u64) -> *const u64 {
    core::ptr::with_exposed_provenance(usize::try_from(item).unwrap_or(0))
}

/// Returns the object an item points to.
fn pointer(item: u64) -> *mut ObjectHeader {
    core::ptr::with_exposed_provenance_mut(usize::try_from(item).unwrap_or(0))
}

/// Returns whether a string or a list is not empty.
#[allow(
    clippy::cast_ptr_alignment,
    reason = "every object comes from malloc, aligned for every type, and starts with its header"
)]
///
/// # Safety
///
/// `object` points to a live string or list.
unsafe fn object_is_true(object: *mut ObjectHeader) -> bool {
    // SAFETY: the caller passes a live object; the type ID says which it is.
    unsafe {
        match (*object).type_id {
            STR_TYPE_ID => (*object.cast::<StrObject>()).length != 0,
            LIST_TYPE_ID => (*object.cast::<super::ListObject>()).length != 0,
            _ => crate::libc::abort_with(b"typy runtime: a list holds an object it cannot test\n"),
        }
    }
}

/// Returns `left == right` for two objects of one type: strings, or lists.
#[allow(
    clippy::cast_ptr_alignment,
    reason = "every object comes from malloc, aligned for every type, and starts with its header"
)]
///
/// # Safety
///
/// Both point to live objects of one type.
unsafe fn objects_equal(left: *mut ObjectHeader, right: *mut ObjectHeader) -> bool {
    // SAFETY: the caller passes live objects; the type ID says which they are.
    unsafe {
        match (*left).type_id {
            STR_TYPE_ID => crate::string::typy_rt_str_compare(
                left.cast::<StrObject>(),
                right.cast::<StrObject>(),
                inline(0).unwrap_or(1),
            ),
            LIST_TYPE_ID => super::typy_rt_list_equal(left.cast(), right.cast()),
            _ => {
                crate::libc::abort_with(b"typy runtime: a list holds an object it cannot compare\n")
            }
        }
    }
}

/// Returns whether two strings, or two lists, stand in the comparison `comparison`.
#[allow(
    clippy::cast_ptr_alignment,
    reason = "every object comes from malloc, aligned for every type, and starts with its header"
)]
unsafe fn objects_order(
    left: *mut ObjectHeader,
    right: *mut ObjectHeader,
    comparison: i64,
) -> bool {
    // SAFETY: the caller passes live objects; the type ID says which they are.
    unsafe {
        match (*left).type_id {
            STR_TYPE_ID => crate::string::typy_rt_str_compare(
                left.cast::<StrObject>(),
                right.cast::<StrObject>(),
                comparison,
            ),
            LIST_TYPE_ID => super::typy_rt_list_compare(left.cast(), right.cast(), comparison),
            _ => crate::libc::abort_with(b"typy runtime: a list holds an object it cannot order\n"),
        }
    }
}

/// Returns `repr(object)` for a string or a list.
#[allow(
    clippy::cast_ptr_alignment,
    reason = "every object comes from malloc, aligned for every type, and starts with its header"
)]
///
/// # Safety
///
/// `object` points to a live string or list.
unsafe fn object_repr(object: *mut ObjectHeader) -> Raising<*mut StrObject> {
    // SAFETY: the caller passes a live object; the type ID says which it is.
    unsafe {
        match (*object).type_id {
            STR_TYPE_ID => Raising::ok(crate::string::typy_rt_str_repr(object.cast::<StrObject>())),
            LIST_TYPE_ID => super::typy_rt_list_repr(object.cast()),
            _ => crate::libc::abort_with(b"typy runtime: a list holds an object it cannot write\n"),
        }
    }
}
