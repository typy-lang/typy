//! The library linked into every TyPy program (ADR-0006): memory, integers, floats, strings, exceptions and output.
//!
//! The library has no standard library and no `alloc` crate. It allocates through the three hooks the program object
//! defines (ADR-0043), and calls only the C functions `libc.rs` declares, with the constants of the system it runs on
//! (ADR-0057), and those of `libm` that `libm.rs` declares. Unit tests build it with the standard library, so its logic
//! is tested like any other crate.
#![cfg_attr(not(test), no_std)]
#![warn(clippy::undocumented_unsafe_blocks)]
#![allow(
    unreachable_pub,
    reason = "the exported functions form the C interface and are reached by symbol, not by path"
)]

/// Exports a runtime function that may raise and gives a `float` under its C symbol, with the
/// `float` returned as its bits, so that every target returns the pair in two integer registers
/// as generated code reads it (ADR-0053).
macro_rules! export_float {
    ($name:ident = $inner:ident($($argument:ident: $ty:ty),*)) => {
        #[doc = concat!("`", stringify!($inner), "` for generated code, with the `float` as its bits (ADR-0053).")]
        #[unsafe(no_mangle)]
        pub extern "C" fn $name($($argument: $ty),*) -> $crate::RaisingFloat {
            $inner($($argument),*).into()
        }
    };
    (unsafe $name:ident = $inner:ident($($argument:ident: $ty:ty),*)) => {
        #[doc = concat!("`", stringify!($inner), "` for generated code, with the `float` as its bits (ADR-0053).")]
        ///
        /// # Safety
        ///
        #[doc = concat!("As for `", stringify!($inner), "`.")]
        #[unsafe(no_mangle)]
        pub unsafe extern "C" fn $name($($argument: $ty),*) -> $crate::RaisingFloat {
            // SAFETY: the caller's guarantee, which the wrapped function states.
            unsafe { $inner($($argument),*) }.into()
        }
    };
}
pub(crate) use export_float;

mod allocator;
mod boxed;
mod byte_buffer;
mod bytes;
mod call;
mod cdata;
mod cell;
mod codec;
mod compare;
mod dict;
mod entry;
mod exception;
mod float;
mod foreign;
mod format;
mod function;
mod generator;
mod hash;
mod header;
mod instance;
mod int;
mod io;
mod iterator;
mod libc;
mod libm;
mod library;
mod list;
mod math;
mod message;
mod method;
mod module;
#[cfg(test)]
mod moved_tests;
mod not_implemented;
mod range;
mod refcount;
mod set;
mod slice;
mod slot;
mod stack;
mod string;
mod sys;
mod text_file;
mod tuple;
mod type_alias;
mod type_object;
mod typing_value;
mod unpack;
mod variable_cell;
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

/// The result of a runtime function that may raise and gives a `float`, as generated code reads
/// it: the bits of the `float`, then the exception (ADR-0053). A C structure of a `double` and a
/// pointer returns in other registers on AArch64 than the pair LLVM returns for `{double, ptr}`,
/// while two 64-bit words return alike on every target.
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct RaisingFloat {
    /// The bits of the result, meaningful only when `exception` is null.
    pub bits: u64,
    /// The exception, or null.
    pub exception: *mut ExceptionObject,
}

impl From<Raising<f64>> for RaisingFloat {
    fn from(result: Raising<f64>) -> Self {
        Self {
            bits: result.value.to_bits(),
            exception: result.exception,
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
