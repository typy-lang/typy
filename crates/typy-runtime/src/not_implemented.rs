//! `NotImplemented`, the value an operator or comparison method returns to give way to the
//! other operand's reflected method (CLS-21): one immortal object of the type
//! `NotImplementedType`, written as `NotImplemented`, hashed and compared by identity.

use crate::exception::ExceptionKind;
use crate::header::{IMMORTAL, NOT_IMPLEMENTED_TYPE_ID, ObjectHeader};
use crate::message::MessageBuffer;
use crate::string::StrObject;
use crate::type_object::{OBJECT, TypeObject};
use crate::{ExceptionObject, Raising};

/// The type `NotImplementedType`.
#[unsafe(export_name = "typy_rt_type_not_implemented")]
pub(crate) static NOT_IMPLEMENTED_TYPE: TypeObject = TypeObject {
    hash: Some(crate::instance::typy_rt_identity_hash),
    repr: Some(not_implemented_repr),
    ..TypeObject::new("NotImplementedType", &raw const OBJECT)
};

/// The one `NotImplemented` object.
static NOT_IMPLEMENTED: ObjectHeader = ObjectHeader {
    reference_count: IMMORTAL,
    type_id: NOT_IMPLEMENTED_TYPE_ID,
};

/// Returns `NotImplemented`, which the runtime keeps for the whole run.
#[unsafe(no_mangle)]
pub extern "C" fn typy_rt_not_implemented() -> *const ObjectHeader {
    &raw const NOT_IMPLEMENTED
}

/// Returns the `TypeError` an arithmetic operator raises when every method it calls gave way:
/// `unsupported operand type(s) for +: 'A' and 'B'`, named by the classes of the operands.
///
/// # Safety
///
/// `symbol` points to a live `str`, and `left` and `right` to live type objects.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn typy_rt_operands_unsupported(
    symbol: *const StrObject,
    left: *const TypeObject,
    right: *const TypeObject,
) -> *mut ExceptionObject {
    let mut message = MessageBuffer::new();
    message.push_str("unsupported operand type(s) for ");
    // SAFETY: the caller's guarantee.
    unsafe {
        message.push_text(symbol);
        message.push_str(": '");
        message.push_str((*left).name());
        message.push_str("' and '");
        message.push_str((*right).name());
    }
    message.push_str("'");
    message.raise(ExceptionKind::TypeError)
}

/// Returns the `TypeError` of a checked `cast` whose value is not of its type:
/// `cast to int found str`, named by the cast's type as TyPy writes it and the value's class
/// (TYP-24, ADR-0091).
///
/// # Safety
///
/// `target` points to a live `str`, and `found` to a live type object.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn typy_rt_cast_failed(
    target: *const StrObject,
    found: *const TypeObject,
) -> *mut ExceptionObject {
    let mut message = MessageBuffer::new();
    message.push_str("cast to ");
    // SAFETY: the caller's guarantee.
    unsafe {
        message.push_text(target);
        message.push_str(" found ");
        message.push_str((*found).name());
    }
    message.raise(ExceptionKind::TypeError)
}

/// Returns the `TypeError` an ordering raises when every method it calls gave way:
/// `'<' not supported between instances of 'A' and 'B'`.
///
/// # Safety
///
/// `symbol` points to a live `str`, and `left` and `right` to live type objects.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn typy_rt_ordering_unsupported(
    symbol: *const StrObject,
    left: *const TypeObject,
    right: *const TypeObject,
) -> *mut ExceptionObject {
    let mut message = MessageBuffer::new();
    message.push_str("'");
    // SAFETY: the caller's guarantee.
    unsafe {
        message.push_text(symbol);
        message.push_str("' not supported between instances of '");
        message.push_str((*left).name());
        message.push_str("' and '");
        message.push_str((*right).name());
    }
    message.push_str("'");
    message.raise(ExceptionKind::TypeError)
}

/// Returns `repr` of `NotImplemented`.
unsafe extern "C" fn not_implemented_repr(_object: *mut ObjectHeader) -> Raising<*mut StrObject> {
    Raising::ok(crate::string::from_str("NotImplemented"))
}

#[cfg(test)]
mod tests;
