//! The type objects of the runtime's own types, each with `object` as its base.

use super::{OBJECT, TypeObject};
use crate::Raising;
use crate::dict::DictObject;
use crate::header::ObjectHeader;
use crate::list::ListObject;
use crate::set::SetObject;
use crate::string::StrObject;
use crate::word::inline;

/// The comparison code of `==`, as an `int` word.
fn equal_code() -> i64 {
    inline(0).unwrap_or(1)
}

#[allow(
    clippy::cast_ptr_alignment,
    reason = "every object comes from malloc, aligned for every type, and starts with its header"
)]
unsafe extern "C" fn text_is_true(object: *mut ObjectHeader) -> bool {
    // SAFETY: the type of the object says it is a `str` or a `bytes`.
    unsafe { (*object.cast::<StrObject>()).length != 0 }
}

#[allow(
    clippy::cast_ptr_alignment,
    reason = "every object comes from malloc, aligned for every type, and starts with its header"
)]
unsafe extern "C" fn text_hash(object: *mut ObjectHeader) -> i64 {
    // SAFETY: the type of the object says it is a `str` or a `bytes`.
    unsafe { crate::hash::text(object.cast::<StrObject>()) }
}

#[allow(
    clippy::cast_ptr_alignment,
    reason = "every object comes from malloc, aligned for every type, and starts with its header"
)]
unsafe extern "C" fn text_equal(left: *mut ObjectHeader, right: *mut ObjectHeader) -> bool {
    // SAFETY: the types of the objects say they are both `str` or both `bytes`.
    unsafe { crate::string::typy_rt_str_compare(left.cast(), right.cast(), equal_code()) }
}

#[allow(
    clippy::cast_ptr_alignment,
    reason = "every object comes from malloc, aligned for every type, and starts with its header"
)]
unsafe extern "C" fn text_order(
    left: *mut ObjectHeader,
    right: *mut ObjectHeader,
    comparison: i64,
) -> bool {
    // SAFETY: the types of the objects say they are both `str` or both `bytes`.
    unsafe { crate::string::typy_rt_str_compare(left.cast(), right.cast(), comparison) }
}

#[allow(
    clippy::cast_ptr_alignment,
    reason = "every object comes from malloc, aligned for every type, and starts with its header"
)]
unsafe extern "C" fn str_repr(object: *mut ObjectHeader) -> Raising<*mut StrObject> {
    // SAFETY: the type of the object says it is a `str`.
    Raising::ok(unsafe { crate::string::typy_rt_str_repr(object.cast()) })
}

#[allow(
    clippy::cast_ptr_alignment,
    reason = "every object comes from malloc, aligned for every type, and starts with its header"
)]
unsafe extern "C" fn bytes_repr(object: *mut ObjectHeader) -> Raising<*mut StrObject> {
    // SAFETY: the type of the object says it is a `bytes`.
    Raising::ok(unsafe { crate::bytes::typy_rt_bytes_repr(object.cast()) })
}

#[allow(
    clippy::cast_ptr_alignment,
    reason = "every object comes from malloc, aligned for every type, and starts with its header"
)]
unsafe extern "C" fn list_is_true(object: *mut ObjectHeader) -> bool {
    // SAFETY: the type of the object says it is a list or a tuple.
    unsafe { (*object.cast::<ListObject>()).length != 0 }
}

#[allow(
    clippy::cast_ptr_alignment,
    reason = "every object comes from malloc, aligned for every type, and starts with its header"
)]
unsafe extern "C" fn tuple_hash(object: *mut ObjectHeader) -> i64 {
    // SAFETY: the type of the object says it is a sealed tuple; every index is below its length.
    unsafe {
        let tuple = &*object.cast::<ListObject>();
        let item_type = tuple.item_type();
        let hashes = (0..tuple.length).map(|index| item_type.hash(tuple.item(index)));
        crate::hash::tuple(hashes, tuple.length)
    }
}

#[allow(
    clippy::cast_ptr_alignment,
    reason = "every object comes from malloc, aligned for every type, and starts with its header"
)]
unsafe extern "C" fn list_equal(left: *mut ObjectHeader, right: *mut ObjectHeader) -> bool {
    // SAFETY: the types of the objects say they are lists, or tuples, of one item type.
    unsafe { crate::list::typy_rt_list_equal(left.cast(), right.cast()) }
}

#[allow(
    clippy::cast_ptr_alignment,
    reason = "every object comes from malloc, aligned for every type, and starts with its header"
)]
unsafe extern "C" fn list_order(
    left: *mut ObjectHeader,
    right: *mut ObjectHeader,
    comparison: i64,
) -> bool {
    // SAFETY: the types of the objects say they are lists, or tuples, of one item type.
    unsafe { crate::list::typy_rt_list_compare(left.cast(), right.cast(), comparison) }
}

#[allow(
    clippy::cast_ptr_alignment,
    reason = "every object comes from malloc, aligned for every type, and starts with its header"
)]
unsafe extern "C" fn list_repr(object: *mut ObjectHeader) -> Raising<*mut StrObject> {
    // SAFETY: the type of the object says it is a list or a tuple.
    unsafe { crate::list::typy_rt_list_repr(object.cast()) }
}

#[allow(
    clippy::cast_ptr_alignment,
    reason = "every object comes from malloc, aligned for every type, and starts with its header"
)]
unsafe extern "C" fn set_is_true(object: *mut ObjectHeader) -> bool {
    // SAFETY: the type of the object says it is a set.
    unsafe { (*object.cast::<SetObject>()).used != 0 }
}

#[allow(
    clippy::cast_ptr_alignment,
    reason = "every object comes from malloc, aligned for every type, and starts with its header"
)]
unsafe extern "C" fn set_equal(left: *mut ObjectHeader, right: *mut ObjectHeader) -> bool {
    // SAFETY: the types of the objects say they are sets of one item type.
    unsafe { crate::set::typy_rt_set_compare(left.cast(), right.cast(), equal_code()) }
}

#[allow(
    clippy::cast_ptr_alignment,
    reason = "every object comes from malloc, aligned for every type, and starts with its header"
)]
unsafe extern "C" fn set_order(
    left: *mut ObjectHeader,
    right: *mut ObjectHeader,
    comparison: i64,
) -> bool {
    // SAFETY: the types of the objects say they are sets of one item type.
    unsafe { crate::set::typy_rt_set_compare(left.cast(), right.cast(), comparison) }
}

#[allow(
    clippy::cast_ptr_alignment,
    reason = "every object comes from malloc, aligned for every type, and starts with its header"
)]
unsafe extern "C" fn set_repr(object: *mut ObjectHeader) -> Raising<*mut StrObject> {
    // SAFETY: the type of the object says it is a set.
    unsafe { crate::set::typy_rt_set_repr(object.cast()) }
}

#[allow(
    clippy::cast_ptr_alignment,
    reason = "every object comes from malloc, aligned for every type, and starts with its header"
)]
unsafe extern "C" fn dict_is_true(object: *mut ObjectHeader) -> bool {
    // SAFETY: the type of the object says it is a dict.
    unsafe { (*object.cast::<DictObject>()).used != 0 }
}

#[allow(
    clippy::cast_ptr_alignment,
    reason = "every object comes from malloc, aligned for every type, and starts with its header"
)]
unsafe extern "C" fn dict_equal(left: *mut ObjectHeader, right: *mut ObjectHeader) -> bool {
    // SAFETY: the types of the objects say they are dicts of one key and one value type.
    unsafe { crate::dict::typy_rt_dict_equal(left.cast(), right.cast()) }
}

#[allow(
    clippy::cast_ptr_alignment,
    reason = "every object comes from malloc, aligned for every type, and starts with its header"
)]
unsafe extern "C" fn dict_repr(object: *mut ObjectHeader) -> Raising<*mut StrObject> {
    // SAFETY: the type of the object says it is a dict.
    unsafe { crate::dict::typy_rt_dict_repr(object.cast()) }
}

#[allow(
    clippy::cast_ptr_alignment,
    reason = "every object comes from malloc, aligned for every type, and starts with its header"
)]
unsafe extern "C" fn view_is_true(object: *mut ObjectHeader) -> bool {
    // SAFETY: the type of the object says it is a view of a dict.
    unsafe { crate::dict::view_length(object.cast()) != 0 }
}

#[allow(
    clippy::cast_ptr_alignment,
    reason = "every object comes from malloc, aligned for every type, and starts with its header"
)]
unsafe extern "C" fn view_repr(object: *mut ObjectHeader) -> Raising<*mut StrObject> {
    // SAFETY: the type of the object says it is a view of a dict.
    unsafe { crate::dict::typy_rt_dict_view_text(object.cast()) }
}

/// Returns the type object of a runtime type, whose base is `object`.
const fn builtin(name: &'static str) -> TypeObject {
    TypeObject::new(name, &raw const OBJECT)
}

/// The type of an `int`, whose large values are heap objects.
pub(crate) static INT: TypeObject = builtin("int");

/// The type of a `str`.
pub(crate) static STR: TypeObject = TypeObject {
    truth: Some(text_is_true),
    hash: Some(text_hash),
    equal: Some(text_equal),
    order: Some(text_order),
    repr: Some(str_repr),
    ..builtin("str")
};

/// The type of a `bytes`.
pub(crate) static BYTES: TypeObject = TypeObject {
    truth: Some(text_is_true),
    hash: Some(text_hash),
    equal: Some(text_equal),
    order: Some(text_order),
    repr: Some(bytes_repr),
    ..builtin("bytes")
};

/// The type of a `list`.
pub(crate) static LIST: TypeObject = TypeObject {
    truth: Some(list_is_true),
    equal: Some(list_equal),
    order: Some(list_order),
    repr: Some(list_repr),
    ..builtin("list")
};

/// The type of a `tuple[T, ...]`.
pub(crate) static TUPLE: TypeObject = TypeObject {
    truth: Some(list_is_true),
    hash: Some(tuple_hash),
    equal: Some(list_equal),
    order: Some(list_order),
    repr: Some(list_repr),
    ..builtin("tuple")
};

/// The type of a `set`.
pub(crate) static SET: TypeObject = TypeObject {
    truth: Some(set_is_true),
    equal: Some(set_equal),
    order: Some(set_order),
    repr: Some(set_repr),
    ..builtin("set")
};

/// The type of a `dict`.
pub(crate) static DICT: TypeObject = TypeObject {
    truth: Some(dict_is_true),
    equal: Some(dict_equal),
    repr: Some(dict_repr),
    ..builtin("dict")
};

/// Returns the type object of one kind of view of a dict.
const fn view(name: &'static str) -> TypeObject {
    TypeObject {
        truth: Some(view_is_true),
        repr: Some(view_repr),
        ..builtin(name)
    }
}

/// The type of `dict.keys()`.
pub(crate) static DICT_KEYS: TypeObject = view("dict_keys");
/// The type of `dict.values()`.
pub(crate) static DICT_VALUES: TypeObject = view("dict_values");
/// The type of `dict.items()`.
pub(crate) static DICT_ITEMS: TypeObject = view("dict_items");

/// The type of an exception.
pub(crate) static BASE_EXCEPTION: TypeObject = builtin("BaseException");
