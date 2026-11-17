//! What a library built with `typy build --lib` asks of the runtime: running its modules when
//! it loads, guarding each call of an export, and ending as a program ends (CLI-09, EXC-05,
//! ADR-0044).

use crate::exception::ExceptionObject;

/// The arguments of a library's `sys.argv`: one empty string, as in an embedded CPython.
struct NoArguments([*const u8; 1]);

// SAFETY: the one pointer is to a static string that nothing writes.
unsafe impl Sync for NoArguments {}

static NO_ARGUMENTS: NoArguments = NoArguments([c"".as_ptr().cast()]);

/// Runs the library's module bodies, as the constructor of its object asks when the library
/// is loaded, and writes what they printed.
///
/// # Safety
///
/// `module` is the generated `__typy_main`, which takes nothing and returns an exception or
/// null.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn typy_rt_library_load(
    module: unsafe extern "C" fn() -> *mut ExceptionObject,
) {
    crate::stack::record_limit();
    crate::sys::keep_arguments(1, NO_ARGUMENTS.0.as_ptr());
    // SAFETY: the caller's guarantee.
    let exception = unsafe { module() };
    if !exception.is_null() {
        // SAFETY: a non-null result is a live exception that nothing else holds.
        unsafe { typy_rt_library_raised(exception) }
    }
    crate::io::flush_stdout();
}

/// Writes what the library printed and closes the files it opened, as the process ends; the
/// destructor of the library's object calls it.
#[unsafe(no_mangle)]
pub extern "C" fn typy_rt_library_unload() {
    crate::io::flush_stdout();
    crate::text_file::close_all();
    crate::io::flush_stdout();
}

/// Starts a call of an export: the recursion check measures the calling thread's stack from
/// here.
#[unsafe(no_mangle)]
pub extern "C" fn typy_rt_library_enter() {
    crate::stack::record_limit();
}

/// Ends a call of an export: what it printed is written before its caller writes more.
#[unsafe(no_mangle)]
pub extern "C" fn typy_rt_library_leave() {
    crate::io::flush_stdout();
}

/// Ends the process for an exception that would leave an export or the library's modules,
/// since C has no exceptions (EXC-05): a `SystemExit` exits with its status, as it ends a
/// program, and any other exception is reported as an uncaught one is and aborts.
///
/// # Safety
///
/// `exception` is a live exception that nothing else holds.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn typy_rt_library_raised(exception: *mut ExceptionObject) -> ! {
    crate::io::flush_stdout();
    // SAFETY: the caller's guarantee.
    unsafe {
        if crate::sys::is_system_exit((*exception).class()) {
            let (status, reported) = crate::sys::exit_status(exception);
            if reported {
                crate::exception::report_uncaught(exception);
            }
            // The destructor writes the rest of the output and closes the files.
            crate::libc::exit_process(status)
        }
        crate::exception::report_uncaught(exception);
    }
    crate::libc::abort_process()
}

#[cfg(test)]
mod tests {
    use super::{typy_rt_library_enter, typy_rt_library_leave};

    #[test]
    #[cfg_attr(
        miri,
        ignore = "Miri cannot run the inline assembly that reads the stack pointer"
    )]
    fn a_call_of_an_export_measures_the_stack_it_runs_on() {
        typy_rt_library_enter();
        assert!(crate::stack::typy_rt_check_stack().is_null());
        typy_rt_library_leave();
    }
}
