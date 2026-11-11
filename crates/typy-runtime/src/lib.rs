//! The library linked into every TyPy program (ADR-0006): memory, integers, floats, strings, exceptions and output.
//!
//! The library has no standard library and no `alloc` crate; it calls only `malloc`, `realloc`, `free`, `write`, `abort`,
//! `getrlimit` and `floor`, `fmod`, `pow` and `ldexp` from `libm`. Unit tests build it with the standard library, so its logic is tested like any other crate.
#![cfg_attr(not(test), no_std)]
#![warn(clippy::undocumented_unsafe_blocks)]
#![allow(
    unreachable_pub,
    reason = "the exported functions form the C interface and are reached by symbol, not by path"
)]

mod allocator;
mod bytes;
mod cell;
mod codec;
mod compare;
mod dict;
mod entry;
mod exception;
mod float;
mod format;
mod hash;
mod header;
mod instance;
mod int;
mod io;
mod iterator;
mod libc;
mod libm;
mod list;
mod message;
mod module;
mod range;
mod refcount;
mod set;
mod slice;
mod stack;
mod string;
mod tuple;
mod type_object;
mod version;
mod word;
mod zip;

pub use exception::{ExceptionKind, ExceptionObject};
pub use header::{
    BIG_INT_TYPE_ID, BYTES_TYPE_ID, EXCEPTION_TYPE_ID, IMMORTAL, LIST_TYPE_ID, ObjectHeader,
    STR_TYPE_ID, TUPLE_TYPE_ID,
};
pub use int::BigIntObject;
pub use list::ListObject;
pub use string::{StrKind, StrObject};

/// A result that may carry an exception, as a function that may raise returns it (detailed design section 10.2).
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct Raising<T> {
    /// The result, meaningful only when `exception` is null.
    pub value: T,
    /// The exception, or null.
    pub exception: *mut ExceptionObject,
}

impl<T> Raising<T> {
    /// Returns a result without an exception.
    pub const fn ok(value: T) -> Self {
        Self {
            value,
            exception: core::ptr::null_mut(),
        }
    }

    /// Returns an exception, with `placeholder` in place of the result.
    pub const fn error(placeholder: T, exception: *mut ExceptionObject) -> Self {
        Self {
            value: placeholder,
            exception,
        }
    }
}

/// The unwinding personality that the precompiled `core` and `compiler_builtins` refer to from
/// their unwind tables. The runtime aborts on panic, so nothing ever unwinds into it.
#[cfg(not(test))]
#[unsafe(no_mangle)]
extern "C" fn rust_eh_personality() {}

#[cfg(not(test))]
#[panic_handler]
fn panic(_info: &core::panic::PanicInfo<'_>) -> ! {
    libc::abort_with(b"typy runtime: internal error\n")
}
