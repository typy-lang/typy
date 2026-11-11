//! `str` and `repr` of an exception, from the text the runtime gave it or from the arguments
//! the program built it with, as `BaseException` writes them (EXC-08, ADR-0013).

use super::ExceptionObject;
use super::classes::{KEY_ERROR, derives_from};
use crate::Raising;
use crate::header::ObjectHeader;
use crate::refcount::typy_rt_increment;
use crate::string::{StrObject, from_str};

/// Returns `str(exception)`: the runtime's text; or from the arguments, nothing for none, the
/// `str` of the only one, its `repr` for a `KeyError`, and the `repr` of the tuple of several.
///
/// # Safety
///
/// `object` points to a live exception.
#[allow(
    clippy::cast_ptr_alignment,
    reason = "every object comes from malloc, aligned for every type, and starts with its header"
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn typy_rt_exception_str(
    object: *mut ObjectHeader,
) -> Raising<*mut StrObject> {
    // SAFETY: the caller's guarantee.
    let exception = unsafe { &*object.cast::<ExceptionObject>() };
    if !exception.message.is_null() {
        // SAFETY: the message is a live `str` the exception owns.
        unsafe { typy_rt_increment(exception.message.cast()) };
        return Raising::ok(exception.message);
    }
    let Some(arguments) = Arguments::of(exception) else {
        return Raising::ok(from_str(""));
    };
    // SAFETY: the arguments are live and laid out as their descriptor says.
    unsafe {
        match arguments.count() {
            1 if derives_from(exception.class(), &KEY_ERROR) => arguments.item_repr(0),
            1 => arguments.item_str(0),
            _ => crate::tuple::repr(arguments.layout, arguments.address),
        }
    }
}

/// Returns `repr(exception)`: the class name, then the arguments between parentheses, without
/// the comma after a single one. The runtime's text stands for its one argument: written as
/// it is for a `KeyError`, whose text is already the `repr` of the key, and with `repr`
/// otherwise.
///
/// # Safety
///
/// `object` points to a live exception.
#[allow(
    clippy::cast_ptr_alignment,
    reason = "every object comes from malloc, aligned for every type, and starts with its header"
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn typy_rt_exception_repr(
    object: *mut ObjectHeader,
) -> Raising<*mut StrObject> {
    // SAFETY: the caller's guarantee.
    let exception = unsafe { &*object.cast::<ExceptionObject>() };
    let message = exception.message;
    // SAFETY: the arguments are live and laid out as their descriptor says, and the message
    // is a live `str` the exception owns.
    let (open, inner, close) = unsafe {
        match Arguments::of(exception) {
            Some(arguments) if arguments.count() == 1 => ("(", arguments.item_repr(0), ")"),
            Some(arguments) => (
                "",
                crate::tuple::repr(arguments.layout, arguments.address),
                "",
            ),
            None if message.is_null() => ("(", Raising::ok(from_str("")), ")"),
            None if derives_from(exception.class(), &KEY_ERROR) => {
                typy_rt_increment(message.cast());
                ("(", Raising::ok(message), ")")
            }
            None => (
                "(",
                Raising::ok(crate::string::typy_rt_str_repr(message)),
                ")",
            ),
        }
    };
    if !inner.exception.is_null() {
        return inner;
    }
    let head = crate::string::from_parts(&[exception.class().name(), open]);
    let tail = from_str(close);
    // SAFETY: the three strings are live and owned here, and each is released once.
    unsafe {
        let first = crate::string::typy_rt_str_concat(head, inner.value);
        let result = crate::string::typy_rt_str_concat(first, tail);
        for text in [head, inner.value, first, tail] {
            crate::refcount::typy_rt_decrement(text.cast());
        }
        Raising::ok(result)
    }
}

/// The arguments of an exception: their layout and their address.
struct Arguments {
    layout: *const crate::tuple::TupleType,
    address: u64,
}

impl Arguments {
    /// Returns the arguments of an exception the program built with at least one argument.
    fn of(exception: &ExceptionObject) -> Option<Self> {
        if exception.arguments.is_null() {
            return None;
        }
        Some(Self {
            layout: exception.arguments_type,
            address: exception.arguments.expose_provenance() as u64,
        })
    }

    /// Returns the number of arguments.
    fn count(&self) -> u64 {
        // SAFETY: the descriptor lives as long as the program.
        unsafe { (*self.layout).length }
    }

    /// Returns `str` of argument `index`.
    ///
    /// # Safety
    ///
    /// The arguments are live, with more than `index` of them.
    unsafe fn item_str(&self, index: usize) -> Raising<*mut StrObject> {
        // SAFETY: the caller's guarantee.
        unsafe {
            crate::tuple::field_type(self.layout, index).str(crate::tuple::field(
                self.layout,
                self.address,
                index,
            ))
        }
    }

    /// Returns `repr` of argument `index`.
    ///
    /// # Safety
    ///
    /// The arguments are live, with more than `index` of them.
    unsafe fn item_repr(&self, index: usize) -> Raising<*mut StrObject> {
        // SAFETY: the caller's guarantee.
        unsafe {
            crate::tuple::field_type(self.layout, index).repr(crate::tuple::field(
                self.layout,
                self.address,
                index,
            ))
        }
    }
}
