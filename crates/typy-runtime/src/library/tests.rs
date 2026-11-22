//! Tests of what a library asks of the runtime.

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
