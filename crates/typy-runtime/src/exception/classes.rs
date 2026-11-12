//! The built-in exception classes, with CPython's bases (EXC-02, EXC-10, ADR-0013). Generated
//! code names them by their symbols, as `raise` and `except` operands and as the bases of a
//! program's own exception classes.

use crate::type_object::{OBJECT, TypeObject};

/// Returns an exception class named `name` with `base`: its objects are hashed by identity and
/// written from their arguments, as CPython writes `BaseException`.
const fn class(name: &'static str, base: &'static TypeObject) -> TypeObject {
    TypeObject {
        hash: Some(crate::instance::typy_rt_identity_hash),
        repr: Some(super::text::typy_rt_exception_repr),
        str: Some(super::text::typy_rt_exception_str),
        ..TypeObject::new(name, base)
    }
}

/// The root of every exception class.
#[unsafe(export_name = "typy_rt_exception_BaseException")]
pub(crate) static BASE_EXCEPTION: TypeObject = class("BaseException", &OBJECT);
/// The base of every exception a program is meant to catch.
#[unsafe(export_name = "typy_rt_exception_Exception")]
pub(crate) static EXCEPTION: TypeObject = class("Exception", &BASE_EXCEPTION);
/// `sys.exit`: the program ends, with the status its argument gives (LIB-04).
#[unsafe(export_name = "typy_rt_exception_SystemExit")]
pub(crate) static SYSTEM_EXIT: TypeObject = class("SystemExit", &BASE_EXCEPTION);
/// `input` at the end of standard input.
#[unsafe(export_name = "typy_rt_exception_EOFError")]
pub(crate) static EOF_ERROR: TypeObject = class("EOFError", &EXCEPTION);
/// The base of the errors of arithmetic.
#[unsafe(export_name = "typy_rt_exception_ArithmeticError")]
pub(crate) static ARITHMETIC_ERROR: TypeObject = class("ArithmeticError", &EXCEPTION);
/// `assert` with a false test.
#[unsafe(export_name = "typy_rt_exception_AssertionError")]
pub(crate) static ASSERTION_ERROR: TypeObject = class("AssertionError", &EXCEPTION);
/// An attribute that is missing.
#[unsafe(export_name = "typy_rt_exception_AttributeError")]
pub(crate) static ATTRIBUTE_ERROR: TypeObject = class("AttributeError", &EXCEPTION);
/// An import that cannot find the module or the name it imports.
#[unsafe(export_name = "typy_rt_exception_ImportError")]
pub(crate) static IMPORT_ERROR: TypeObject = class("ImportError", &EXCEPTION);
/// A name read while it is not bound.
#[unsafe(export_name = "typy_rt_exception_NameError")]
pub(crate) static NAME_ERROR: TypeObject = class("NameError", &EXCEPTION);
/// The base of the errors of a missing index or key.
#[unsafe(export_name = "typy_rt_exception_LookupError")]
pub(crate) static LOOKUP_ERROR: TypeObject = class("LookupError", &EXCEPTION);
/// An index out of range.
#[unsafe(export_name = "typy_rt_exception_IndexError")]
pub(crate) static INDEX_ERROR: TypeObject = class("IndexError", &LOOKUP_ERROR);
/// A key that is missing.
#[unsafe(export_name = "typy_rt_exception_KeyError")]
pub(crate) static KEY_ERROR: TypeObject = class("KeyError", &LOOKUP_ERROR);
/// A result too large to hold.
#[unsafe(export_name = "typy_rt_exception_OverflowError")]
pub(crate) static OVERFLOW_ERROR: TypeObject = class("OverflowError", &ARITHMETIC_ERROR);
/// An error that fits no other class.
#[unsafe(export_name = "typy_rt_exception_RuntimeError")]
pub(crate) static RUNTIME_ERROR: TypeObject = class("RuntimeError", &EXCEPTION);
/// A method a subclass is meant to define.
#[unsafe(export_name = "typy_rt_exception_NotImplementedError")]
pub(crate) static NOT_IMPLEMENTED_ERROR: TypeObject = class("NotImplementedError", &RUNTIME_ERROR);
/// Calls nested deeper than the limit.
#[unsafe(export_name = "typy_rt_exception_RecursionError")]
pub(crate) static RECURSION_ERROR: TypeObject = class("RecursionError", &RUNTIME_ERROR);
/// The end of an iterator.
#[unsafe(export_name = "typy_rt_exception_StopIteration")]
pub(crate) static STOP_ITERATION: TypeObject = class("StopIteration", &EXCEPTION);
/// An operation on a value of the wrong type.
#[unsafe(export_name = "typy_rt_exception_TypeError")]
pub(crate) static TYPE_ERROR: TypeObject = class("TypeError", &EXCEPTION);
/// A value of the right type that is not allowed.
#[unsafe(export_name = "typy_rt_exception_ValueError")]
pub(crate) static VALUE_ERROR: TypeObject = class("ValueError", &EXCEPTION);
/// The base of the errors of encoding and decoding text.
#[unsafe(export_name = "typy_rt_exception_UnicodeError")]
pub(crate) static UNICODE_ERROR: TypeObject = class("UnicodeError", &VALUE_ERROR);
/// Bytes that a codec cannot decode.
#[unsafe(export_name = "typy_rt_exception_UnicodeDecodeError")]
pub(crate) static UNICODE_DECODE_ERROR: TypeObject = class("UnicodeDecodeError", &UNICODE_ERROR);
/// Text that a codec cannot encode.
#[unsafe(export_name = "typy_rt_exception_UnicodeEncodeError")]
pub(crate) static UNICODE_ENCODE_ERROR: TypeObject = class("UnicodeEncodeError", &UNICODE_ERROR);
/// Division or modulo by zero.
#[unsafe(export_name = "typy_rt_exception_ZeroDivisionError")]
pub(crate) static ZERO_DIVISION_ERROR: TypeObject = class("ZeroDivisionError", &ARITHMETIC_ERROR);

/// An error the operating system reports, with its `errno` (ADR-0019).
#[unsafe(export_name = "typy_rt_exception_OSError")]
pub(crate) static OS_ERROR: TypeObject = class("OSError", &EXCEPTION);
/// An operation that would block.
#[unsafe(export_name = "typy_rt_exception_BlockingIOError")]
pub(crate) static BLOCKING_IO_ERROR: TypeObject = class("BlockingIOError", &OS_ERROR);
/// An operation on a child process that failed.
#[unsafe(export_name = "typy_rt_exception_ChildProcessError")]
pub(crate) static CHILD_PROCESS_ERROR: TypeObject = class("ChildProcessError", &OS_ERROR);
/// The base of the errors of a connection.
#[unsafe(export_name = "typy_rt_exception_ConnectionError")]
pub(crate) static CONNECTION_ERROR: TypeObject = class("ConnectionError", &OS_ERROR);
/// A write to a pipe or socket whose other end is closed.
#[unsafe(export_name = "typy_rt_exception_BrokenPipeError")]
pub(crate) static BROKEN_PIPE_ERROR: TypeObject = class("BrokenPipeError", &CONNECTION_ERROR);
/// A connection the other end aborted.
#[unsafe(export_name = "typy_rt_exception_ConnectionAbortedError")]
pub(crate) static CONNECTION_ABORTED_ERROR: TypeObject =
    class("ConnectionAbortedError", &CONNECTION_ERROR);
/// A connection the other end refused.
#[unsafe(export_name = "typy_rt_exception_ConnectionRefusedError")]
pub(crate) static CONNECTION_REFUSED_ERROR: TypeObject =
    class("ConnectionRefusedError", &CONNECTION_ERROR);
/// A connection the other end reset.
#[unsafe(export_name = "typy_rt_exception_ConnectionResetError")]
pub(crate) static CONNECTION_RESET_ERROR: TypeObject =
    class("ConnectionResetError", &CONNECTION_ERROR);
/// A file created that exists already.
#[unsafe(export_name = "typy_rt_exception_FileExistsError")]
pub(crate) static FILE_EXISTS_ERROR: TypeObject = class("FileExistsError", &OS_ERROR);
/// A file or directory that does not exist.
#[unsafe(export_name = "typy_rt_exception_FileNotFoundError")]
pub(crate) static FILE_NOT_FOUND_ERROR: TypeObject = class("FileNotFoundError", &OS_ERROR);
/// A call a signal interrupted.
#[unsafe(export_name = "typy_rt_exception_InterruptedError")]
pub(crate) static INTERRUPTED_ERROR: TypeObject = class("InterruptedError", &OS_ERROR);
/// A file operation on a directory.
#[unsafe(export_name = "typy_rt_exception_IsADirectoryError")]
pub(crate) static IS_A_DIRECTORY_ERROR: TypeObject = class("IsADirectoryError", &OS_ERROR);
/// A directory operation on something that is not a directory.
#[unsafe(export_name = "typy_rt_exception_NotADirectoryError")]
pub(crate) static NOT_A_DIRECTORY_ERROR: TypeObject = class("NotADirectoryError", &OS_ERROR);
/// An operation without the access rights it needs.
#[unsafe(export_name = "typy_rt_exception_PermissionError")]
pub(crate) static PERMISSION_ERROR: TypeObject = class("PermissionError", &OS_ERROR);
/// A process that does not exist.
#[unsafe(export_name = "typy_rt_exception_ProcessLookupError")]
pub(crate) static PROCESS_LOOKUP_ERROR: TypeObject = class("ProcessLookupError", &OS_ERROR);
/// An operation that timed out.
#[unsafe(export_name = "typy_rt_exception_TimeoutError")]
pub(crate) static TIMEOUT_ERROR: TypeObject = class("TimeoutError", &OS_ERROR);
/// `io.UnsupportedOperation`: reading a file opened for writing, or the other way round. Its
/// bases in CPython are `OSError` and `ValueError`; `derives_from` adds the second.
pub(crate) static UNSUPPORTED_OPERATION: TypeObject = TypeObject {
    module: "io".as_ptr(),
    module_length: 2,
    ..class("UnsupportedOperation", &OS_ERROR)
};

/// Every built-in exception class.
#[cfg(test)]
pub(crate) const EVERY_CLASS: [&TypeObject; 40] = [
    &BASE_EXCEPTION,
    &EXCEPTION,
    &SYSTEM_EXIT,
    &EOF_ERROR,
    &OS_ERROR,
    &BLOCKING_IO_ERROR,
    &CHILD_PROCESS_ERROR,
    &CONNECTION_ERROR,
    &BROKEN_PIPE_ERROR,
    &CONNECTION_ABORTED_ERROR,
    &CONNECTION_REFUSED_ERROR,
    &CONNECTION_RESET_ERROR,
    &FILE_EXISTS_ERROR,
    &FILE_NOT_FOUND_ERROR,
    &INTERRUPTED_ERROR,
    &IS_A_DIRECTORY_ERROR,
    &NOT_A_DIRECTORY_ERROR,
    &PERMISSION_ERROR,
    &PROCESS_LOOKUP_ERROR,
    &TIMEOUT_ERROR,
    &UNSUPPORTED_OPERATION,
    &ARITHMETIC_ERROR,
    &ASSERTION_ERROR,
    &ATTRIBUTE_ERROR,
    &IMPORT_ERROR,
    &NAME_ERROR,
    &LOOKUP_ERROR,
    &INDEX_ERROR,
    &KEY_ERROR,
    &OVERFLOW_ERROR,
    &RUNTIME_ERROR,
    &NOT_IMPLEMENTED_ERROR,
    &RECURSION_ERROR,
    &STOP_ITERATION,
    &TYPE_ERROR,
    &VALUE_ERROR,
    &UNICODE_ERROR,
    &UNICODE_DECODE_ERROR,
    &UNICODE_ENCODE_ERROR,
    &ZERO_DIVISION_ERROR,
];

/// Returns an exception class built when the program runs, as a test builds one.
#[cfg(test)]
pub(crate) fn class_for_test(name: &'static str) -> TypeObject {
    class(name, &EXCEPTION)
}

/// Returns whether `class` is `base` or has `base` among its bases, `ValueError` counting as
/// a base of `io.UnsupportedOperation`, as in CPython.
pub(crate) fn derives_from(class: &TypeObject, base: &TypeObject) -> bool {
    let mut current: *const TypeObject = class;
    while !current.is_null() {
        if core::ptr::eq(current, base) {
            return true;
        }
        if core::ptr::eq(current, &raw const UNSUPPORTED_OPERATION)
            && derives_from(&VALUE_ERROR, base)
        {
            return true;
        }
        // SAFETY: a base is null or a live, immortal type object.
        current = unsafe { (*current).base };
    }
    false
}

#[cfg(test)]
mod tests {
    use super::{
        ARITHMETIC_ERROR, BASE_EXCEPTION, BROKEN_PIPE_ERROR, CONNECTION_ERROR, EVERY_CLASS,
        EXCEPTION, FILE_NOT_FOUND_ERROR, KEY_ERROR, LOOKUP_ERROR, OBJECT, OS_ERROR, TYPE_ERROR,
        UNICODE_DECODE_ERROR, UNSUPPORTED_OPERATION, VALUE_ERROR, ZERO_DIVISION_ERROR,
        derives_from,
    };

    #[test]
    fn every_exception_class_derives_from_base_exception_and_object() {
        for class in EVERY_CLASS {
            assert!(derives_from(class, &BASE_EXCEPTION), "{}", class.name());
            assert!(derives_from(class, &OBJECT), "{}", class.name());
        }
    }

    #[test]
    fn a_class_derives_from_its_bases_and_not_from_its_siblings() {
        let cases = [
            (&ZERO_DIVISION_ERROR, &ARITHMETIC_ERROR, true),
            (&ZERO_DIVISION_ERROR, &EXCEPTION, true),
            (&KEY_ERROR, &LOOKUP_ERROR, true),
            (&UNICODE_DECODE_ERROR, &VALUE_ERROR, true),
            (&KEY_ERROR, &KEY_ERROR, true),
            (&KEY_ERROR, &TYPE_ERROR, false),
            (&EXCEPTION, &KEY_ERROR, false),
            (&OBJECT, &BASE_EXCEPTION, false),
            (&FILE_NOT_FOUND_ERROR, &OS_ERROR, true),
            (&BROKEN_PIPE_ERROR, &CONNECTION_ERROR, true),
            (&BROKEN_PIPE_ERROR, &OS_ERROR, true),
            (&FILE_NOT_FOUND_ERROR, &VALUE_ERROR, false),
            (&UNSUPPORTED_OPERATION, &OS_ERROR, true),
            (&UNSUPPORTED_OPERATION, &VALUE_ERROR, true),
            (&UNSUPPORTED_OPERATION, &EXCEPTION, true),
            (&UNSUPPORTED_OPERATION, &KEY_ERROR, false),
            (&UNSUPPORTED_OPERATION, &FILE_NOT_FOUND_ERROR, false),
        ];
        for (class, base, expected) in cases {
            assert_eq!(
                derives_from(class, base),
                expected,
                "{} {}",
                class.name(),
                base.name()
            );
        }
    }
}
