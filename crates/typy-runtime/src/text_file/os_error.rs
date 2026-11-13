//! `OSError` and its subclasses for an error number, as CPython raises them (ADR-0019).

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

static ERROR_FIELDS: [Field; 2] = [
    Field::plain(ItemKind::Int, 0),
    Field::plain(ItemKind::Object, 8),
];

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
    // SAFETY: the exception is new and has no arguments; it takes the text's reference.
    unsafe {
        crate::exception::set_arguments(
            exception,
            &ERROR_ARGUMENTS,
            &[number, strerror.expose_provenance() as u64],
        );
    }
    exception
}
