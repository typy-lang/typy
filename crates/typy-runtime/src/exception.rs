//! Exception objects: their kind, message, traceback and cause (ADR-0004, EXC-02).

mod kind;
mod report;
mod traceback;

use crate::allocator::{allocate, free};
use crate::header::{EXCEPTION_TYPE_ID, ObjectHeader};
use crate::string::{StrObject, from_str};

pub use kind::ExceptionKind;
#[cfg_attr(
    test,
    allow(
        unused_imports,
        reason = "only `main` reports uncaught exceptions, and tests have no `main`"
    )
)]
pub(crate) use report::report_uncaught;
use traceback::Traceback;

/// An exception: a heap object with the exception type ID.
#[repr(C)]
#[derive(Debug)]
pub struct ExceptionObject {
    /// The object header.
    pub header: ObjectHeader,
    /// The exception's class.
    pub kind: ExceptionKind,
    padding: u32,
    /// The message, an owned `str`.
    pub message: *mut StrObject,
    traceback: Traceback,
    /// The exception this one was raised from, or null.
    pub cause: *mut ExceptionObject,
}

/// Returns a new exception of `kind` with `message`, owned by the caller.
pub(crate) fn raise(kind: ExceptionKind, message: &str) -> *mut ExceptionObject {
    raise_with_message(kind, from_str(message))
}

/// Returns a new exception of `kind` that takes over the reference to `message`.
pub(crate) fn raise_with_message(
    kind: ExceptionKind,
    message: *mut StrObject,
) -> *mut ExceptionObject {
    #[allow(
        clippy::cast_ptr_alignment,
        reason = "malloc returns memory aligned for every type"
    )]
    let object = allocate(core::mem::size_of::<ExceptionObject>()).cast::<ExceptionObject>();
    let exception = ExceptionObject {
        header: ObjectHeader::new(EXCEPTION_TYPE_ID),
        kind,
        padding: 0,
        message,
        traceback: Traceback::new(),
        cause: core::ptr::null_mut(),
    };
    // SAFETY: `object` points to enough writable bytes for an exception.
    unsafe { object.write(exception) };
    object
}

/// Frees an exception whose last reference is gone, with its message, traceback and cause.
///
/// # Safety
///
/// `exception` points to a live exception that nothing else refers to.
pub(crate) unsafe fn destroy(exception: *mut ExceptionObject) {
    // SAFETY: the exception is live and owns its message, traceback and cause.
    unsafe {
        crate::refcount::typy_rt_decrement((*exception).message.cast());
        (*exception).traceback.release();
        if !(*exception).cause.is_null() {
            crate::refcount::typy_rt_decrement((*exception).cause.cast());
        }
    }
    free(exception.cast());
}

/// Adds the frame a raising call is in to the exception's traceback: function name, file name and line, as C strings.
///
/// # Safety
///
/// `exception` points to a live exception; `function` and `file` point to static, null-terminated strings.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn typy_rt_traceback_add(
    exception: *mut ExceptionObject,
    function: *const u8,
    file: *const u8,
    line: u32,
) {
    // SAFETY: the caller passes a live exception and static strings.
    unsafe { (*exception).traceback.push(function, file, line) };
}

#[cfg(test)]
mod tests {
    use super::{ExceptionKind, raise, typy_rt_traceback_add};

    #[test]
    fn an_exception_keeps_its_kind_message_and_frames() {
        let exception = raise(ExceptionKind::ValueError, "bad");
        // SAFETY: the exception is live, the names are static and null-terminated, and the last reference is dropped at the end.
        unsafe {
            typy_rt_traceback_add(
                exception,
                c"f".as_ptr().cast(),
                c"app.py".as_ptr().cast(),
                3,
            );
            typy_rt_traceback_add(
                exception,
                c"<module>".as_ptr().cast(),
                c"app.py".as_ptr().cast(),
                7,
            );
            assert_eq!((*exception).kind, ExceptionKind::ValueError);
            assert_eq!((*exception).traceback.len(), 2);
            crate::refcount::typy_rt_decrement(exception.cast());
        }
    }
}
