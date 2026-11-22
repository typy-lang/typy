//! Tests of `guard`.

use crate::exit_status::ExitStatus;
use crate::output::{ErrorFormat, Reporter};

fn guarded(command: impl FnOnce() -> ExitStatus) -> ExitStatus {
    super::guarded(Reporter::new(ErrorFormat::Text), command)
}

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
