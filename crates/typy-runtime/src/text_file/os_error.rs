//! `OSError` and its subclasses for an error number, as CPython raises them (ADR-0019), and
//! as a program builds them with two to five arguments, with their attributes `errno`,
//! `strerror`, `filename` and `filename2` (EXC-14).

use super::errno_text::ERRNO_TEXTS;
use crate::exception::ExceptionObject;
use crate::exception::classes::{
    BLOCKING_IO_ERROR, BROKEN_PIPE_ERROR, CHILD_PROCESS_ERROR, CONNECTION_ABORTED_ERROR,
    CONNECTION_REFUSED_ERROR, CONNECTION_RESET_ERROR, FILE_EXISTS_ERROR, FILE_NOT_FOUND_ERROR,
    INTERRUPTED_ERROR, IS_A_DIRECTORY_ERROR, NOT_A_DIRECTORY_ERROR, OS_ERROR, PERMISSION_ERROR,
    PROCESS_LOOKUP_ERROR, TIMEOUT_ERROR,
};
use crate::list::ItemKind;
use crate::message::MessageBuffer;
use crate::string::StrObject;
use crate::tuple::{Field, TupleType};
use crate::type_object::TypeObject;

/// The layout of the arguments of an `OSError` the runtime raises: `errno` and `strerror`.
static ERROR_ARGUMENTS: TupleType = TupleType {
    size: 16,
    length: 2,
    fields: ERROR_FIELDS.as_ptr(),
};

static ERROR_FIELDS: [Field; 3] = [
    Field::plain(ItemKind::Int, 0),
    Field::plain(ItemKind::Object, 8),
    Field::plain(ItemKind::Object, 16),
];

/// The layout of the arguments of an `OSError` the runtime raises for a file: `errno`,
/// `strerror` and `filename`.
static NAMED_ERROR_ARGUMENTS: TupleType = TupleType {
    size: 24,
    length: 3,
    fields: ERROR_FIELDS.as_ptr(),
};

/// Returns the class CPython gives an error number: the subclass of `OSError` its
/// `errno` table names, or `OSError` itself.
pub(crate) fn class_of(errno: i32) -> &'static TypeObject {
    match errno {
        1 | 13 => &PERMISSION_ERROR,
        2 => &FILE_NOT_FOUND_ERROR,
        3 => &PROCESS_LOOKUP_ERROR,
        4 => &INTERRUPTED_ERROR,
        10 => &CHILD_PROCESS_ERROR,
        11 | 114 | 115 => &BLOCKING_IO_ERROR,
        17 => &FILE_EXISTS_ERROR,
        20 => &NOT_A_DIRECTORY_ERROR,
        21 => &IS_A_DIRECTORY_ERROR,
        32 | 108 => &BROKEN_PIPE_ERROR,
        103 => &CONNECTION_ABORTED_ERROR,
        104 => &CONNECTION_RESET_ERROR,
        110 => &TIMEOUT_ERROR,
        111 => &CONNECTION_REFUSED_ERROR,
        _ => &OS_ERROR,
    }
}

/// Returns glibc's text of an error number, `Unknown error 200` for one it does not name.
pub(crate) fn text_of(errno: i32) -> *mut StrObject {
    let known = usize::try_from(errno)
        .ok()
        .and_then(|number| number.checked_sub(1))
        .and_then(|index| ERRNO_TEXTS.get(index));
    if let Some(text) = known {
        return crate::string::from_str(text);
    }
    let mut text = MessageBuffer::new();
    text.push_str("Unknown error ");
    text.push_number(i64::from(errno));
    text.text()
}

/// Returns a new exception of the class for `errno`, written as
/// `[Errno 2] No such file or directory: 'name'` with the `repr` of `filename`, or without
/// it when `filename` is null, and holding the arguments `(errno, strerror)`.
///
/// # Safety
///
/// `filename` is null or points to a live `str`.
pub(crate) unsafe fn os_error(errno: i32, filename: *const StrObject) -> *mut ExceptionObject {
    let strerror = text_of(errno);
    let mut message = MessageBuffer::new();
    message.push_str("[Errno ");
    message.push_number(i64::from(errno));
    message.push_str("] ");
    // SAFETY: the text is a live `str`, and the caller passes a live file name or null.
    unsafe {
        message.push_text(strerror);
        if !filename.is_null() {
            message.push_str(": ");
            message.push_repr(filename);
        }
    }
    let exception = crate::exception::raise_class(class_of(errno), message.text());
    let number = crate::word::inline(i64::from(errno))
        .unwrap_or(1)
        .cast_unsigned();
    // SAFETY: the exception is new and has no arguments; it takes the text's reference, and
    // one of its own to the file name.
    unsafe {
        let text = strerror.expose_provenance() as u64;
        if filename.is_null() {
            crate::exception::set_arguments(exception, &ERROR_ARGUMENTS, &[number, text]);
        } else {
            crate::refcount::typy_rt_increment(filename.cast_mut().cast());
            let name = filename.expose_provenance() as u64;
            crate::exception::set_arguments(
                exception,
                &NAMED_ERROR_ARGUMENTS,
                &[number, text, name],
            );
        }
    }
    exception
}

/// Returns a new `OSError` of `class` built with the arguments `items` of the layout
/// `arguments_type`: two to five, `errno`, `strerror`, `filename`, `winerror` and
/// `filename2`, the checker having checked their types. `OSError` itself gives the subclass
/// for `errno`, and the exception is written as `[Errno 2] text: 'name' -> 'other'`.
///
/// # Safety
///
/// `class` is `OSError` or a class derived from it, and `items` a live tuple of
/// `arguments_type`, whose first two fields are an `int` and a `str`, and whose third and
/// fifth, when present, are a `str` or `None`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn typy_rt_os_error_new(
    class: *const TypeObject,
    arguments_type: *const TupleType,
    items: *const u8,
) -> *mut ExceptionObject {
    let address = items.expose_provenance() as u64;
    // SAFETY: the caller's guarantee.
    unsafe {
        let errno = crate::tuple::field(arguments_type, address, 0).cast_signed();
        let class = if core::ptr::eq(class, &raw const OS_ERROR) && crate::word::is_inline(errno) {
            let number = crate::word::inline_value(errno);
            i32::try_from(number).map_or(&OS_ERROR, class_of)
        } else {
            &*class
        };
        let exception = crate::exception::typy_rt_exception_new(class, arguments_type, items);
        let strerror = crate::tuple::field(arguments_type, address, 1);
        let mut message = MessageBuffer::new();
        message.push_str("[Errno ");
        let written = crate::int::typy_rt_int_to_str(errno).value;
        message.push_text(written);
        crate::refcount::typy_rt_decrement(written.cast());
        message.push_str("] ");
        message.push_text(core::ptr::with_exposed_provenance(
            usize::try_from(strerror).unwrap_or(0),
        ));
        if let Some(first) = name_at(arguments_type, address, 2) {
            message.push_str(": ");
            message.push_repr(first);
            if let Some(second) = name_at(arguments_type, address, 4) {
                message.push_str(" -> ");
                message.push_repr(second);
            }
        }
        (*exception).message = message.text();
        exception
    }
}

/// Returns the file name at `index` of an `OSError`'s arguments: a `str`, or `None` when it
/// is `None` or not given.
///
/// # Safety
///
/// `address` is a live tuple of `layout`.
unsafe fn name_at(
    layout: *const TupleType,
    address: u64,
    index: usize,
) -> Option<*const StrObject> {
    // SAFETY: the caller's guarantee.
    unsafe {
        if index >= usize::try_from((*layout).length).unwrap_or(0) {
            return None;
        }
        let (ty, value) = crate::tuple::field_type(layout, index)
            .held_value(crate::tuple::field(layout, address, index))?;
        if ty.kind() != ItemKind::Object {
            return None;
        }
        // A class's own `__init__` may have given the exception arguments of any type.
        let text: *const StrObject =
            core::ptr::with_exposed_provenance(usize::try_from(value).unwrap_or(0));
        ((*text).header.type_id == crate::header::STR_TYPE_ID).then_some(text)
    }
}

/// Returns `errno` of an `OSError`: its first argument when it was built with two or more,
/// as an `int` word with a reference of its own, or 0 for `None`.
///
/// # Safety
///
/// `exception` points to a live `OSError` or an exception of a class derived from it.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn typy_rt_os_error_errno(exception: *const ExceptionObject) -> i64 {
    // SAFETY: the caller's guarantee.
    unsafe {
        let exception = &*exception;
        if exception.arguments.is_null() || (*exception.arguments_type).length < 2 {
            return 0;
        }
        let address = exception.arguments.expose_provenance() as u64;
        // A class's own `__init__` may have given the exception arguments of any type.
        let Some((ty, errno)) = crate::tuple::field_type(exception.arguments_type, 0)
            .held_value(crate::tuple::field(exception.arguments_type, address, 0))
        else {
            return 0;
        };
        if ty.kind() != ItemKind::Int {
            return 0;
        }
        crate::refcount::typy_rt_int_increment(errno.cast_signed());
        errno.cast_signed()
    }
}

/// Returns `strerror`, `filename` or `filename2` of an `OSError`, for `index` the `int` word of
/// 1, 2 or 4: that argument, with a reference of its own, or null for `None`.
///
/// # Safety
///
/// `exception` points to a live `OSError` or an exception of a class derived from it.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn typy_rt_os_error_text(
    exception: *const ExceptionObject,
    index: i64,
) -> *mut StrObject {
    let index = usize::try_from(crate::word::inline_value(index)).unwrap_or(usize::MAX);
    // SAFETY: the caller's guarantee.
    unsafe {
        let exception = &*exception;
        if exception.arguments.is_null() || (*exception.arguments_type).length < 2 {
            return core::ptr::null_mut();
        }
        let address = exception.arguments.expose_provenance() as u64;
        match name_at(exception.arguments_type, address, index) {
            Some(text) => {
                crate::refcount::typy_rt_increment(text.cast_mut().cast());
                text.cast_mut()
            }
            None => core::ptr::null_mut(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{typy_rt_os_error_errno, typy_rt_os_error_new, typy_rt_os_error_text};
    use crate::exception::classes::{FILE_NOT_FOUND_ERROR, OS_ERROR};
    use crate::list::ItemKind;
    use crate::refcount::typy_rt_decrement;
    use crate::string::{StrObject, code_point, from_str, length};
    use crate::tuple::{Field, TupleType};
    use crate::word::{inline, inline_value};

    fn text(string: *const StrObject) -> String {
        // SAFETY: the string is live.
        unsafe {
            (0..length(string))
                .map(|index| char::from_u32(code_point(string, index)).unwrap())
                .collect()
        }
    }

    static FIELDS: [Field; 3] = [
        Field::plain(ItemKind::Int, 0),
        Field::plain(ItemKind::Object, 8),
        Field::plain(ItemKind::Object, 16),
    ];
    static NAMED: TupleType = TupleType {
        size: 24,
        length: 3,
        fields: FIELDS.as_ptr(),
    };

    #[test]
    fn os_error_reads_its_arguments_as_errno_strerror_and_a_file_name() {
        let (message, name) = (from_str("gone"), from_str("a.txt"));
        let items = [
            inline(2).unwrap().cast_unsigned(),
            message.expose_provenance() as u64,
            name.expose_provenance() as u64,
        ];
        // SAFETY: the items are live values of the layout, which the exception copies.
        unsafe {
            let error =
                typy_rt_os_error_new(&raw const OS_ERROR, &raw const NAMED, items.as_ptr().cast());
            assert!(core::ptr::eq(
                (*error).class(),
                &raw const FILE_NOT_FOUND_ERROR
            ));
            assert_eq!(text((*error).message), "[Errno 2] gone: 'a.txt'");
            assert_eq!(inline_value(typy_rt_os_error_errno(error)), 2);
            for (index, expected) in [(1, Some("gone")), (2, Some("a.txt")), (4, None)] {
                let found = typy_rt_os_error_text(error, inline(index).unwrap());
                assert_eq!((!found.is_null()).then(|| text(found)).as_deref(), expected);
                if !found.is_null() {
                    typy_rt_decrement(found.cast());
                }
            }
            typy_rt_decrement(error.cast());
            typy_rt_decrement(message.cast());
            typy_rt_decrement(name.cast());
        }
    }

    #[test]
    fn an_os_error_of_one_argument_has_no_errno() {
        let raised = crate::exception::raise_class(&OS_ERROR, from_str("plain"));
        // SAFETY: the exception is live and owned here.
        unsafe {
            assert_eq!(typy_rt_os_error_errno(raised), 0);
            assert!(typy_rt_os_error_text(raised, inline(1).unwrap()).is_null());
            typy_rt_decrement(raised.cast());
        }
    }
}
