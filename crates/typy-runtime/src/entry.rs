//! The program's `main`: it runs the module body and reports an exception nothing caught (STD-05).

#[cfg(not(test))]
mod program {
    use crate::exception::{ExceptionObject, report_uncaught};

    unsafe extern "C" {
        /// The body of `__main__`, which generated code defines.
        fn __typy_main() -> *mut ExceptionObject;
        /// The end of the program, which generated code defines: it releases the module
        /// variables of a program with generators, so those they hold close (ADR-0071).
        fn __typy_finish();
    }

    /// Runs the program; the exit status is 0, or the one an exception nothing caught gives:
    /// 1, or the status of a `SystemExit` (LIB-04).
    #[unsafe(no_mangle)]
    pub extern "C" fn main(argument_count: i32, arguments: *const *const u8) -> i32 {
        crate::stack::record_limit();
        crate::sys::keep_arguments(argument_count, arguments);
        // SAFETY: `__typy_main` is the generated module body, which takes nothing and returns an exception or null.
        let exception = unsafe { __typy_main() };
        crate::io::flush_stdout();
        if exception.is_null() {
            // SAFETY: `__typy_finish` is the generated end, which takes and returns nothing.
            unsafe { __typy_finish() };
            crate::text_file::close_all();
            crate::io::flush_stdout();
            0
        } else {
            // SAFETY: a non-null result is a live exception that the program no longer holds;
            // it is released after the report, so the program ends with nothing allocated
            // (NFR-08).
            unsafe {
                let (status, reported) = crate::sys::exit_status(exception);
                if reported {
                    report_uncaught(exception);
                }
                crate::refcount::typy_rt_decrement(exception.cast());
                // The variables the exception kept go with it, so generators among them
                // close now, after the report, as CPython closes them (FUN-11, ADR-0070).
                crate::generator::close_dropped_at_exit();
                __typy_finish();
                crate::text_file::close_all();
                crate::io::flush_stdout();
                status
            }
        }
    }
}
