//! The program's `main`: it runs the module body and reports an exception nothing caught (STD-05).

#[cfg(not(test))]
mod program {
    use crate::exception::{ExceptionObject, report_uncaught};

    unsafe extern "C" {
        /// The body of `__main__`, which generated code defines.
        fn __typy_main() -> *mut ExceptionObject;
    }

    /// Runs the program; the exit status is 0, or 1 when an exception was not caught.
    #[unsafe(no_mangle)]
    pub extern "C" fn main(_argument_count: i32, _arguments: *const *const u8) -> i32 {
        crate::stack::record_limit();
        // SAFETY: `__typy_main` is the generated module body, which takes nothing and returns an exception or null.
        let exception = unsafe { __typy_main() };
        crate::io::flush_stdout();
        if exception.is_null() {
            0
        } else {
            // SAFETY: a non-null result is a live exception.
            unsafe { report_uncaught(exception) };
            1
        }
    }
}
