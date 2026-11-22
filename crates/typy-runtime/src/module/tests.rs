//! Tests of module objects and the errors of reading their variables.

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
fn a_module_without_a_file_reads_as_built_in() {
    let (name, path) = (from_str("math"), from_str(""));
    let module = super::ModuleObject {
        header: crate::header::ObjectHeader {
            reference_count: crate::header::IMMORTAL,
            type_id: crate::header::MODULE_TYPE_ID,
        },
        name,
        path,
    };
    // SAFETY: the module and the strings are live for the whole test.
    unsafe {
        let object = core::ptr::from_ref(&module).cast_mut();
        let text = super::module_repr(object.cast()).value;
        let repr: String = (0..crate::string::length(text))
            .map(|index| char::from_u32(crate::string::code_point(text, index)).unwrap())
            .collect();
        crate::refcount::typy_rt_decrement(text.cast());
        assert_eq!(repr, "<module 'math' (built-in)>");
        for text in [name, path] {
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
