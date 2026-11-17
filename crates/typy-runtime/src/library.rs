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
    let ending = unsafe { ending(exception) };
    if ending.reported {
        // SAFETY: the caller's guarantee.
        unsafe { crate::exception::report_uncaught(exception) };
    }
    match ending.status {
        // The destructor writes the rest of the output and closes the files.
        Some(status) => crate::libc::exit_process(status),
        None => crate::libc::abort_process(),
    }
}

/// How the process ends for an exception that leaves a library.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Ending {
    /// The status it exits with, or `None` to abort.
    status: Option<i32>,
    /// Whether the exception is reported first.
    reported: bool,
}

/// Returns how the process ends for `exception`: a `SystemExit` exits with its status,
/// reported only when its code is not an `int` or `None`, and any other exception is
/// reported and aborts.
///
/// # Safety
///
/// `exception` points to a live exception.
unsafe fn ending(exception: *mut ExceptionObject) -> Ending {
    // SAFETY: the caller's guarantee.
    if unsafe { crate::sys::is_system_exit((*exception).class()) } {
        // SAFETY: the caller's guarantee.
        let (status, reported) = unsafe { crate::sys::exit_status(exception) };
        return Ending {
            status: Some(status),
            reported,
        };
    }
    Ending {
        status: None,
        reported: true,
    }
}

#[cfg(test)]
mod tests {
    use super::{
        Ending, ending, typy_rt_library_enter, typy_rt_library_leave, typy_rt_library_load,
        typy_rt_library_unload,
    };
    use crate::exception::ExceptionObject;

    unsafe extern "C" fn module_that_runs() -> *mut ExceptionObject {
        core::ptr::null_mut()
    }

    #[test]
    #[cfg_attr(
        miri,
        ignore = "Miri cannot run the inline assembly that reads the stack pointer"
    )]
    fn a_library_runs_its_module_when_it_loads_and_sys_argv_is_one_empty_string() {
        // SAFETY: the module takes nothing and returns no exception.
        unsafe { typy_rt_library_load(module_that_runs) };
        let argv = crate::sys::typy_rt_sys_argv();
        // SAFETY: `sys.argv` is a live list of strings.
        unsafe {
            assert_eq!((*argv).length, 1);
            let first = (*argv).item(0) as *const crate::string::StrObject;
            assert_eq!(crate::string::length(first), 0);
        }
        typy_rt_library_unload();
    }

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

    #[test]
    fn an_exception_aborts_and_system_exit_exits_with_its_status() {
        let error = crate::exception::raise(crate::ExceptionKind::ValueError, "refused");
        let exit = crate::exception::raise_class(
            &crate::exception::classes::SYSTEM_EXIT,
            crate::string::from_str("stopped"),
        );
        // SAFETY: the exceptions are live and dropped once.
        unsafe {
            assert_eq!(
                ending(error),
                Ending {
                    status: None,
                    reported: true
                }
            );
            // An exception the runtime raises keeps its message apart from `args`, so this
            // `SystemExit` has none and exits with status 0, as `sys.exit()` does.
            assert_eq!(
                ending(exit),
                Ending {
                    status: Some(0),
                    reported: false
                }
            );
            crate::refcount::typy_rt_decrement(error.cast());
            crate::refcount::typy_rt_decrement(exit.cast());
        }
    }
}
