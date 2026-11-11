//! Module objects, and the errors of reading module variables and of importing their names,
//! raised with CPython's messages (SMT-08, MOD-10, ADR-0015).

use crate::Raising;
use crate::exception::{ExceptionKind, ExceptionObject};
use crate::header::ObjectHeader;
use crate::message::MessageBuffer;
use crate::string::StrObject;
use crate::type_object::{OBJECT, TypeObject};
use crate::word::inline_value;

/// A module object: an immortal static object the compiler emits, with the module's
/// qualified name and the path of its file from the project root.
#[repr(C)]
#[derive(Debug)]
pub(crate) struct ModuleObject {
    header: ObjectHeader,
    name: *const StrObject,
    path: *const StrObject,
}

/// The type `module`: its objects are written `<module 'm' from 'm.py'>` (OI-20).
#[unsafe(export_name = "typy_rt_type_module")]
pub(crate) static MODULE: TypeObject = TypeObject {
    hash: Some(crate::instance::typy_rt_identity_hash),
    repr: Some(module_repr),
    ..TypeObject::new("module", &raw const OBJECT)
};

/// Returns `repr` of a module object, as CPython's `module_repr` writes it.
#[allow(
    clippy::cast_ptr_alignment,
    reason = "a module object is a static the compiler aligns for its pointers"
)]
unsafe extern "C" fn module_repr(object: *mut ObjectHeader) -> Raising<*mut StrObject> {
    // SAFETY: the type of the object says it is a module object, whose strings are static.
    unsafe {
        let module = &*object.cast::<ModuleObject>();
        let mut text = MessageBuffer::new();
        text.push_str("<module ");
        text.push_repr(module.name);
        text.push_str(" from ");
        text.push_repr(module.path);
        text.push_str(">");
        Raising::ok(text.text())
    }
}

/// Appends what CPython 3.13 to 3.15 add to an error about a module still initialising,
/// whose file lies in the program's own directory and so may hide a library (OI-14).
///
/// # Safety
///
/// `module` points to a live module object.
unsafe fn push_initialising(text: &mut MessageBuffer, module: &ModuleObject) {
    text.push_str(" (consider renaming '");
    // SAFETY: the caller's guarantee.
    unsafe { text.push_text(module.path) };
    text.push_str("' if it has the same name as a library you intended to import)");
}

/// Returns `AttributeError: module 'm' has no attribute 'x'` for a module variable read
/// through its module while it is not bound; `initialising`, an `int` word of 1 while the
/// module's code runs, adds CPython's note.
///
/// # Safety
///
/// `module` points to a live module object and `name` to a live `str`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn typy_rt_module_attribute_error(
    module: *const ModuleObject,
    name: *const StrObject,
    initialising: i64,
) -> *mut ExceptionObject {
    let mut text = MessageBuffer::new();
    // SAFETY: the caller's guarantee.
    unsafe {
        let module = &*module;
        text.push_str("module '");
        text.push_text(module.name);
        text.push_str("' has no attribute '");
        text.push_text(name);
        text.push_str("'");
        if inline_value(initialising) == 1 {
            push_initialising(&mut text, module);
        }
    }
    text.raise(ExceptionKind::AttributeError)
}

/// Returns `ImportError: cannot import name 'x' from 'm' (m.py)` for `from m import x` while
/// `x` is not bound; `initialising` is as for `typy_rt_module_attribute_error`.
///
/// # Safety
///
/// `module` points to a live module object and `name` to a live `str`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn typy_rt_module_import_error(
    module: *const ModuleObject,
    name: *const StrObject,
    initialising: i64,
) -> *mut ExceptionObject {
    let mut text = MessageBuffer::new();
    // SAFETY: the caller's guarantee.
    unsafe {
        let module = &*module;
        text.push_str("cannot import name '");
        text.push_text(name);
        text.push_str("' from '");
        text.push_text(module.name);
        text.push_str("'");
        if inline_value(initialising) == 1 {
            push_initialising(&mut text, module);
        } else {
            text.push_str(" (");
            text.push_text(module.path);
            text.push_str(")");
        }
    }
    text.raise(ExceptionKind::ImportError)
}

/// Returns `NameError: name 'x' is not defined`, raised by a function that reads the module
/// variable `name` while it is not bound.
///
/// # Safety
///
/// `name` points to a live `str` object.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn typy_rt_name_error(name: *const StrObject) -> *mut ExceptionObject {
    let mut message = MessageBuffer::new();
    message.push_str("name '");
    // SAFETY: the caller's guarantee.
    unsafe { message.push_text(name) };
    message.push_str("' is not defined");
    message.raise(ExceptionKind::NameError)
}

#[cfg(test)]
mod tests {
    use super::typy_rt_name_error;
    use crate::string::from_str;

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
    fn module_objects_and_their_errors_read_as_cpython_3_15_writes_them() {
        let name = from_str("shapes");
        let path = from_str("pkg/shapes.py");
        let module = super::ModuleObject {
            header: crate::header::ObjectHeader {
                reference_count: crate::header::IMMORTAL,
                type_id: crate::header::MODULE_TYPE_ID,
            },
            name,
            path,
        };
        let attribute = from_str("area");
        let word = |value| crate::word::inline(value).unwrap();
        // SAFETY: the module and the strings are live for the whole test.
        unsafe {
            let object = core::ptr::from_ref(&module).cast_mut();
            let text = super::module_repr(object.cast()).value;
            let repr: String = (0..crate::string::length(text))
                .map(|index| char::from_u32(crate::string::code_point(text, index)).unwrap())
                .collect();
            crate::refcount::typy_rt_decrement(text.cast());
            assert_eq!(repr, "<module 'shapes' from 'pkg/shapes.py'>");
            let cases = [
                (
                    super::typy_rt_module_attribute_error(object, attribute, word(0)),
                    "module 'shapes' has no attribute 'area'",
                ),
                (
                    super::typy_rt_module_attribute_error(object, attribute, word(1)),
                    "module 'shapes' has no attribute 'area' (consider renaming 'pkg/shapes.py' \
                     if it has the same name as a library you intended to import)",
                ),
                (
                    super::typy_rt_module_import_error(object, attribute, word(0)),
                    "cannot import name 'area' from 'shapes' (pkg/shapes.py)",
                ),
                (
                    super::typy_rt_module_import_error(object, attribute, word(1)),
                    "cannot import name 'area' from 'shapes' (consider renaming 'pkg/shapes.py' \
                     if it has the same name as a library you intended to import)",
                ),
            ];
            for (exception, expected) in cases {
                assert_eq!(shown(exception), expected);
            }
            for text in [name, path, attribute] {
                crate::refcount::typy_rt_decrement(text.cast());
            }
        }
    }

    #[test]
    fn an_unbound_module_variable_raises_name_error_with_cpythons_message() {
        let name = from_str("count");
        // SAFETY: the name is live, and the exception is owned here.
        unsafe {
            let exception = typy_rt_name_error(name);
            assert_eq!((*exception).kind(), crate::ExceptionKind::NameError);
            let text = crate::exception::classes::BASE_EXCEPTION.str.unwrap()(exception.cast());
            let shown: String = (0..crate::string::length(text.value))
                .map(|index| char::from_u32(crate::string::code_point(text.value, index)).unwrap())
                .collect();
            assert_eq!(shown, "name 'count' is not defined");
            crate::refcount::typy_rt_decrement(text.value.cast());
            crate::refcount::typy_rt_decrement(exception.cast());
            crate::refcount::typy_rt_decrement(name.cast());
        }
    }
}
