//! The views `dict.keys()`, `dict.values()` and `dict.items()`: small objects that refer to
//! their dict and read it as it changes, as CPython's views do (DAT-09).

use super::{DictObject, find};
use crate::allocator::{allocate, free};
use crate::header::{DICT_VIEW_TYPE_ID, ObjectHeader};
use crate::tuple::TupleType;
use crate::word::{inline, inline_value};

/// A view of a dict.
#[repr(C)]
pub struct ViewObject {
    /// The object header.
    pub header: ObjectHeader,
    /// What the view shows: 0 for the keys, 1 for the values and 2 for the items.
    pub kind: u32,
    padding: u32,
    /// The dict, which the view holds a reference to.
    pub dict: *mut DictObject,
}

/// Returns a new view of `dict` of the kind whose code is the `int` word `kind`.
///
/// # Safety
///
/// `dict` points to a live dict.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn typy_rt_dict_view(dict: *mut DictObject, kind: i64) -> *mut ViewObject {
    #[allow(
        clippy::cast_ptr_alignment,
        reason = "malloc returns memory aligned for every type"
    )]
    let view = allocate(core::mem::size_of::<ViewObject>()).cast::<ViewObject>();
    // SAFETY: `view` points to enough writable bytes, and the caller passes a live dict.
    unsafe {
        crate::refcount::typy_rt_increment(dict.cast());
        view.write(ViewObject {
            header: ObjectHeader::new(DICT_VIEW_TYPE_ID),
            kind: u32::try_from(inline_value(kind)).unwrap_or(0),
            padding: 0,
            dict,
        });
    }
    view
}

/// Returns the dict a view reads, borrowed from the view.
///
/// # Safety
///
/// `view` points to a live view.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn typy_rt_dict_view_dict(view: *const ViewObject) -> *mut DictObject {
    // SAFETY: the caller's guarantee.
    unsafe { (*view).dict }
}

/// Returns the number of entries of the view's dict, as an `int` word (`len`).
///
/// # Safety
///
/// `view` points to a live view.
pub(crate) unsafe fn length(view: *const ViewObject) -> usize {
    // SAFETY: the caller's guarantee.
    unsafe { (*(*view).dict).used }
}

/// Returns whether the view holds `item`: a key, a value equal to it, or for the items a
/// `(key, value)` tuple laid out by `pair`, whose key maps to an equal value.
///
/// # Safety
///
/// `view` points to a live view, `item` is a live item of what it shows, and `pair` is the
/// descriptor of the pair of a view of items.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn typy_rt_dict_view_contains(
    view: *const ViewObject,
    item: u64,
    pair: u64,
) -> bool {
    // SAFETY: the caller's guarantee.
    unsafe {
        let dict = &*(*view).dict;
        match (*view).kind {
            0 => find(dict, item, dict.key_type().hash(item)).is_some(),
            1 => super::operations::typy_rt_dict_has_value(dict, item),
            _ => {
                let pair = core::ptr::with_exposed_provenance::<TupleType>(
                    usize::try_from(pair).unwrap_or(0),
                );
                let key = crate::tuple::field(pair, item, 0);
                let value = crate::tuple::field(pair, item, 1);
                super::operations::typy_rt_dict_has_item(dict, key, value)
            }
        }
    }
}

/// Returns the number of entries a view shows, as an `int` word.
///
/// # Safety
///
/// `view` points to a live view.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn typy_rt_dict_view_length(view: *const ViewObject) -> i64 {
    // SAFETY: the caller's guarantee.
    let count = unsafe { length(view) };
    inline(i64::try_from(count).unwrap_or(i64::MAX)).unwrap_or(1)
}

/// Frees a view whose last reference is gone, and drops its reference to the dict.
///
/// # Safety
///
/// `view` points to a live view that nothing else refers to.
pub(crate) unsafe fn destroy(view: *mut ViewObject) {
    // SAFETY: the caller's guarantee.
    unsafe { crate::refcount::typy_rt_decrement((*view).dict.cast()) };
    free(view.cast());
}
