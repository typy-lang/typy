//! The kinds of list items, and how each is counted, compared and written as text. An item is
//! eight bytes: an `int` word, the bits of a `float`, a `bool` as 0 or 1, or a pointer.

use crate::Raising;
use crate::header::{LIST_TYPE_ID, ObjectHeader, STR_TYPE_ID};
use crate::string::StrObject;
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
}

impl ItemKind {
    /// Returns the kind with this code; an unknown code is a defect in the compiler.
    pub(crate) fn from_code(code: u32) -> Self {
        match code {
            0 => Self::Int,
            1 => Self::Float,
            2 => Self::Bool,
            3 => Self::Object,
            _ => crate::libc::abort_with(b"typy runtime: a list has an unknown item kind\n"),
        }
    }

    /// Adds a reference to an item that holds one.
    ///
    /// # Safety
    ///
    /// `item` is a live item of this kind.
    pub(crate) unsafe fn retain(self, item: u64) {
        // SAFETY: the caller passes a live item; its bits are an `int` word or a pointer.
        unsafe {
            match self {
                Self::Int => crate::refcount::typy_rt_int_increment(item.cast_signed()),
                Self::Object => crate::refcount::typy_rt_increment(pointer(item)),
                Self::Float | Self::Bool => {}
            }
        }
    }

    /// Drops a reference held by an item.
    ///
    /// # Safety
    ///
    /// `item` is a live item of this kind that the caller holds a reference to.
    pub(crate) unsafe fn release(self, item: u64) {
        // SAFETY: the caller passes a live item it holds a reference to.
        unsafe {
            match self {
                Self::Int => crate::refcount::typy_rt_int_decrement(item.cast_signed()),
                Self::Object => crate::refcount::typy_rt_decrement(pointer(item)),
                Self::Float | Self::Bool => {}
            }
        }
    }

    /// Returns `left == right` for two items of this kind, as CPython compares them.
    #[allow(
        clippy::float_cmp,
        reason = "Python's `==` on floats is IEEE equality, under which `nan` is not equal to itself"
    )]
    ///
    /// # Safety
    ///
    /// Both are live items of this kind.
    pub(crate) unsafe fn equal(self, left: u64, right: u64) -> bool {
        let equal_code = inline(0).unwrap_or(1);
        match self {
            Self::Bool => left == right,
            Self::Float => f64::from_bits(left) == f64::from_bits(right),
            // SAFETY: the caller passes live `int` words.
            Self::Int => unsafe {
                crate::int::typy_rt_int_compare(left.cast_signed(), right.cast_signed(), equal_code)
            },
            // SAFETY: the caller passes pointers to live objects of one type.
            Self::Object => unsafe { objects_equal(pointer(left), pointer(right)) },
        }
    }

    /// Returns `repr(item)`; only an `int` of more than 4300 digits raises.
    ///
    /// # Safety
    ///
    /// `item` is a live item of this kind.
    pub(crate) unsafe fn repr(self, item: u64) -> Raising<*mut StrObject> {
        match self {
            // SAFETY: the caller passes a live `int` word.
            Self::Int => unsafe { crate::int::typy_rt_int_to_str(item.cast_signed()) },
            Self::Float => Raising::ok(crate::float::typy_rt_float_to_str(f64::from_bits(item))),
            Self::Bool => Raising::ok(crate::string::from_str(if item == 0 {
                "False"
            } else {
                "True"
            })),
            // SAFETY: the caller passes a pointer to a live object.
            Self::Object => unsafe { object_repr(pointer(item)) },
        }
    }
}

/// Returns the object an item points to.
fn pointer(item: u64) -> *mut ObjectHeader {
    core::ptr::with_exposed_provenance_mut(usize::try_from(item).unwrap_or(0))
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
