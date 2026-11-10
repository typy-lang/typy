//! Exception objects: their class, arguments, traceback, cause and context (ADR-0004,
//! ADR-0013, EXC-01 to EXC-04).

pub(crate) mod classes;
mod handled;
mod kind;
mod report;
mod text;
mod traceback;

use crate::allocator::{allocate, free};
use crate::header::{EXCEPTION_TYPE_ID, ObjectHeader};
use crate::refcount::{typy_rt_decrement, typy_rt_increment};
use crate::string::{StrObject, from_str};
use crate::tuple::TupleType;
use crate::type_object::TypeObject;

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

/// An exception: a heap object with the exception type ID. A class's own attributes follow
/// it, laid out as the tuple its type object describes.
#[repr(C)]
#[derive(Debug)]
pub struct ExceptionObject {
    /// The object header.
    pub header: ObjectHeader,
    /// The exception's class, at the offset an instance keeps its class.
    pub class: *const TypeObject,
    /// The text the runtime gave an exception it raised, an owned `str`; null for an
    /// exception the program built from arguments.
    pub message: *mut StrObject,
    /// The layout of the arguments the program built the exception with; null for an
    /// exception the runtime raised.
    pub arguments_type: *const TupleType,
    /// The arguments, owned, laid out as `arguments_type` says; null when there are none.
    pub arguments: *mut u8,
    traceback: Traceback,
    /// The exception this one was raised from, or null.
    pub cause: *mut ExceptionObject,
    /// The exception being handled when this one was raised, or null.
    pub context: *mut ExceptionObject,
    /// Whether the context is left out of the report, as `raise ... from` sets it.
    pub suppress_context: u64,
}

/// The bytes before a class's own attributes.
pub const PREFIX: usize = core::mem::size_of::<ExceptionObject>();

impl ExceptionObject {
    /// Returns the class.
    pub(crate) fn class(&self) -> &'static TypeObject {
        // SAFETY: a class is a live, immortal type object.
        unsafe { &*self.class }
    }

    /// Returns the kind of an exception the runtime raised.
    #[cfg(test)]
    pub(crate) fn kind(&self) -> ExceptionKind {
        ExceptionKind::ALL
            .into_iter()
            .find(|kind| core::ptr::eq(kind.class(), self.class))
            .expect("an exception of a runtime class")
    }
}

/// Returns a new exception of `kind` with `message`, owned by the caller.
pub(crate) fn raise(kind: ExceptionKind, message: &str) -> *mut ExceptionObject {
    raise_with_message(kind, from_str(message))
}

/// Returns a new exception of `kind` that takes over the reference to `message`, with the
/// exception being handled as its context.
pub(crate) fn raise_with_message(
    kind: ExceptionKind,
    message: *mut StrObject,
) -> *mut ExceptionObject {
    let exception = new(
        kind.class(),
        message,
        core::ptr::null(),
        core::ptr::null_mut(),
    );
    // SAFETY: the exception is new and live.
    unsafe { typy_rt_exception_raise(exception) };
    exception
}

/// Returns a new exception of `class`, with every attribute of the class zero.
#[allow(
    clippy::cast_ptr_alignment,
    reason = "malloc returns memory aligned for every type"
)]
fn new(
    class: &'static TypeObject,
    message: *mut StrObject,
    arguments_type: *const TupleType,
    arguments: *mut u8,
) -> *mut ExceptionObject {
    let own = if class.fields.is_null() {
        0
    } else {
        // SAFETY: a class's descriptor lives as long as the class.
        usize::try_from(unsafe { (*class.fields).size }).unwrap_or(0)
    };
    let object = allocate(PREFIX + own).cast::<ExceptionObject>();
    let exception = ExceptionObject {
        header: ObjectHeader::new(EXCEPTION_TYPE_ID),
        class,
        message,
        arguments_type,
        arguments,
        traceback: Traceback::new(),
        cause: core::ptr::null_mut(),
        context: core::ptr::null_mut(),
        suppress_context: 0,
    };
    // SAFETY: `object` points to `PREFIX + own` writable bytes; zero attributes hold no
    // reference.
    unsafe {
        object.write(exception);
        core::ptr::write_bytes(object.cast::<u8>().add(PREFIX), 0, own);
    }
    object
}

/// Returns a new exception of `class`, not yet raised, that takes over the arguments the
/// program built it with: `size` bytes at `arguments`, laid out as `arguments_type` says.
///
/// # Safety
///
/// `class` is an exception class, `arguments_type` a descriptor the compiler emitted, and
/// `arguments` points to a live tuple of it whose references pass to the exception.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn typy_rt_exception_new(
    class: *const TypeObject,
    arguments_type: *const TupleType,
    arguments: *const u8,
) -> *mut ExceptionObject {
    // SAFETY: the caller passes a descriptor the compiler emitted.
    let (size, length) = unsafe { ((*arguments_type).size, (*arguments_type).length) };
    let size = usize::try_from(size).unwrap_or(0);
    let owned = if length == 0 {
        core::ptr::null_mut()
    } else {
        let owned = allocate(size);
        // SAFETY: both blocks hold `size` bytes and do not overlap.
        unsafe { core::ptr::copy_nonoverlapping(arguments, owned, size) };
        owned
    };
    // SAFETY: the caller passes a live, immortal class.
    new(
        unsafe { &*class },
        core::ptr::null_mut(),
        arguments_type,
        owned,
    )
}

/// Gives an exception the runtime raised the arguments its `repr` shows, as eight-byte words
/// laid out by `layout`, whose references pass to the exception.
///
/// # Safety
///
/// `exception` is live and has no arguments, and `words` holds one live item per field of
/// `layout`, at offsets that are multiples of eight.
pub(crate) unsafe fn set_arguments(
    exception: *mut ExceptionObject,
    layout: &'static TupleType,
    words: &[u64],
) {
    let owned = allocate(words.len() * 8);
    // SAFETY: the block holds one word per item, aligned for every type.
    unsafe {
        #[allow(
            clippy::cast_ptr_alignment,
            reason = "malloc returns memory aligned for every type"
        )]
        core::ptr::copy_nonoverlapping(words.as_ptr(), owned.cast::<u64>(), words.len());
        (*exception).arguments_type = layout;
        (*exception).arguments = owned;
    }
}

/// Gives a raised exception the exception being handled as its context, unless it is that
/// exception, and cuts any link back to it from the context's own chain, as CPython does, so
/// the chain never forms a cycle.
///
/// # Safety
///
/// `exception` points to a live exception.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn typy_rt_exception_raise(exception: *mut ExceptionObject) {
    let handled = handled::get();
    if handled.is_null() || core::ptr::eq(handled, exception) {
        return;
    }
    // SAFETY: the handled exception and every exception in its chain are live.
    unsafe {
        let mut link = handled;
        while !(*link).context.is_null() {
            let next = (*link).context;
            if core::ptr::eq(next, exception) {
                (*link).context = core::ptr::null_mut();
                typy_rt_decrement(next.cast());
                break;
            }
            link = next;
        }
        typy_rt_increment(handled.cast());
        let old = core::mem::replace(&mut (*exception).context, handled);
        if !old.is_null() {
            typy_rt_decrement(old.cast());
        }
    }
}

/// Sets the cause of `exception` to `cause`, or to none when `cause` is null, and leaves the
/// context out of the report, as `raise exception from cause` does (EXC-04).
///
/// # Safety
///
/// `exception` points to a live exception, and `cause` is null or a live exception whose
/// reference passes to `exception`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn typy_rt_exception_set_cause(
    exception: *mut ExceptionObject,
    cause: *mut ExceptionObject,
) {
    // SAFETY: the caller's guarantee.
    unsafe {
        let old = core::mem::replace(&mut (*exception).cause, cause);
        if !old.is_null() {
            typy_rt_decrement(old.cast());
        }
        (*exception).suppress_context = 1;
    }
}

/// Starts handling `exception` in an `except` clause, and returns the exception handled
/// before, which `typy_rt_exception_leave` restores when the clause ends.
///
/// # Safety
///
/// `exception` points to a live exception that outlives the clause.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn typy_rt_exception_enter(
    exception: *mut ExceptionObject,
) -> *mut ExceptionObject {
    handled::replace(exception)
}

/// Ends an `except` clause, restoring the exception handled before it.
///
/// # Safety
///
/// `previous` is the value `typy_rt_exception_enter` returned for the clause.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn typy_rt_exception_leave(previous: *mut ExceptionObject) {
    handled::replace(previous);
}

/// Returns a new reference to the exception being handled, for a bare `raise` (EXC-07).
///
/// # Safety
///
/// An `except` clause is running, so an exception is being handled.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn typy_rt_exception_handled() -> *mut ExceptionObject {
    let handled = handled::get();
    // SAFETY: the caller's guarantee; the clause keeps the exception alive.
    unsafe { typy_rt_increment(handled.cast()) };
    handled
}

/// Returns whether `exception` is of `class` or of a class derived from it, as `except class`
/// matches (EXC-01).
///
/// # Safety
///
/// `exception` points to a live exception and `class` to a live type object.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn typy_rt_exception_matches(
    exception: *const ExceptionObject,
    class: *const TypeObject,
) -> bool {
    // SAFETY: the caller's guarantee.
    unsafe { classes::derives_from((*exception).class(), &*class) }
}

/// Returns the class of an exception.
///
/// # Safety
///
/// `exception` points to a live exception.
pub(crate) unsafe fn class_of(exception: *const ExceptionObject) -> &'static TypeObject {
    // SAFETY: the caller's guarantee.
    unsafe { (*exception).class() }
}

/// Frees an exception whose last reference is gone, with its message, arguments, traceback,
/// cause, context and attributes.
///
/// # Safety
///
/// `exception` points to a live exception that nothing else refers to.
pub(crate) unsafe fn destroy(exception: *mut ExceptionObject) {
    // SAFETY: the exception is live and owns everything it points to.
    unsafe {
        let object = &mut *exception;
        if !object.message.is_null() {
            typy_rt_decrement(object.message.cast());
        }
        if !object.arguments.is_null() {
            crate::tuple::release(
                object.arguments_type,
                object.arguments.expose_provenance() as u64,
            );
            free(object.arguments);
        }
        object.traceback.release();
        for link in [object.cause, object.context] {
            if !link.is_null() {
                typy_rt_decrement(link.cast());
            }
        }
        let fields = object.class().fields;
        if !fields.is_null() {
            crate::tuple::release(fields, (exception.expose_provenance() + PREFIX) as u64);
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
mod tests;
