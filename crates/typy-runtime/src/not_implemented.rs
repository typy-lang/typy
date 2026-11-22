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
mod tests {
    #[test]
    fn not_implemented_is_written_by_its_name_and_its_type_named_as_cpython_names_it() {
        // SAFETY: the object is the runtime's static.
        unsafe {
            let object = super::typy_rt_not_implemented().cast_mut();
            let text = super::not_implemented_repr(object).value;
            let written: String = (0..crate::string::length(text))
                .map(|index| char::from_u32(crate::string::code_point(text, index)).unwrap())
                .collect();
            assert_eq!(written, "NotImplemented");
            crate::refcount::typy_rt_decrement(text.cast());
            let ty = crate::type_object::typy_rt_type_text(&raw const super::NOT_IMPLEMENTED_TYPE);
            let named: String = (0..crate::string::length(ty))
                .map(|index| char::from_u32(crate::string::code_point(ty, index)).unwrap())
                .collect();
            assert_eq!(named, "<class 'NotImplementedType'>");
            crate::refcount::typy_rt_decrement(ty.cast());
        }
    }

    /// Returns the text of `str` of an exception, and releases it.
    fn shown(exception: *mut crate::ExceptionObject) -> String {
        // SAFETY: the exception is live and owned here.
        unsafe {
            let text = crate::exception::classes::BASE_EXCEPTION.str.unwrap()(exception.cast());
            let shown = (0..crate::string::length(text.value))
                .map(|index| char::from_u32(crate::string::code_point(text.value, index)).unwrap())
                .collect();
            crate::refcount::typy_rt_decrement(text.value.cast());
            crate::refcount::typy_rt_decrement(exception.cast());
            shown
        }
    }

    #[test]
    fn operands_no_method_takes_raise_type_error_as_cpython_words_it() {
        use crate::type_object::{FLOAT, INT};
        let plus = crate::string::from_str("+");
        let less = crate::string::from_str("<=");
        // SAFETY: the strings are live, and the type objects are the runtime's statics.
        unsafe {
            assert_eq!(
                shown(super::typy_rt_operands_unsupported(
                    plus,
                    &raw const INT,
                    &raw const FLOAT
                )),
                "unsupported operand type(s) for +: 'int' and 'float'"
            );
            assert_eq!(
                shown(super::typy_rt_ordering_unsupported(
                    less,
                    &raw const FLOAT,
                    &raw const INT
                )),
                "'<=' not supported between instances of 'float' and 'int'"
            );
            crate::refcount::typy_rt_decrement(plus.cast());
            crate::refcount::typy_rt_decrement(less.cast());
        }
    }

    #[test]
    fn typ_24_a_wrong_cast_raises_type_error_naming_the_type_and_the_class() {
        use crate::type_object::STR;
        let target = crate::string::from_str("int | None");
        // SAFETY: the string is live, and the type object is the runtime's static.
        unsafe {
            assert_eq!(
                shown(super::typy_rt_cast_failed(target, &raw const STR)),
                "cast to int | None found str"
            );
            crate::refcount::typy_rt_decrement(target.cast());
        }
    }
}
