//! Runs a command so that a panic becomes internal error E9001 and exit status 101 (NFR-09).

use std::cell::RefCell;
use std::panic::{self, AssertUnwindSafe};

use crate::exit_status::ExitStatus;
use crate::output::report_internal;

thread_local! {
    /// Where the last panic on this thread happened, recorded by the panic hook.
    static LOCATION: RefCell<Option<String>> = const { RefCell::new(None) };
}

/// Returns the command's status, or `Internal` after reporting E9001 when it panics.
pub(crate) fn guarded(command: impl FnOnce() -> ExitStatus) -> ExitStatus {
    let previous = panic::take_hook();
    panic::set_hook(Box::new(|info| {
        let location = info
            .location()
            .map(|location| format!("{}:{}", location.file(), location.line()));
        LOCATION.with(|cell| *cell.borrow_mut() = location);
    }));
    let result = panic::catch_unwind(AssertUnwindSafe(command));
    panic::set_hook(previous);
    match result {
        Ok(status) => status,
        Err(payload) => {
            let message = payload
                .downcast_ref::<&str>()
                .map(|text| (*text).to_owned())
                .or_else(|| payload.downcast_ref::<String>().cloned())
                .unwrap_or_else(|| "no message".to_owned());
            let location = LOCATION
                .with(|cell| cell.borrow_mut().take())
                .unwrap_or_else(|| "an unknown place".to_owned());
            report_internal(&format!("the compiler panicked at {location}: {message}"));
            ExitStatus::Internal
        }
    }
}

#[cfg(test)]
mod tests {
    use super::guarded;
    use crate::exit_status::ExitStatus;

    #[test]
    fn a_command_that_returns_keeps_its_status() {
        assert_eq!(
            guarded(|| ExitStatus::ProgramErrors),
            ExitStatus::ProgramErrors
        );
    }

    #[test]
    fn a_panic_is_an_internal_error() {
        assert_eq!(guarded(|| panic!("broken invariant")), ExitStatus::Internal);
        let text = String::from("formatted");
        assert_eq!(
            guarded(|| std::panic::panic_any(text)),
            ExitStatus::Internal
        );
        assert_eq!(
            guarded(|| std::panic::panic_any(7_u8)),
            ExitStatus::Internal
        );
    }
}
