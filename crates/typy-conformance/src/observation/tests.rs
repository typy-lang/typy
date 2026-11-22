//! Tests of `observation`.

use super::Observation;
use std::process::Command;

#[test]
fn output_status_and_the_last_error_line_are_kept() {
    let observation = Observation::of(
        Command::new("sh").args(["-c", "echo out; echo first >&2; echo last >&2; exit 3"]),
    )
    .unwrap();
    assert_eq!(observation.stdout, "out\n");
    assert_eq!(
        (observation.status, observation.last_error_line.as_str()),
        (Some(3), "last")
    );
    assert_eq!(
        observation.to_string(),
        "status 3, stdout \"out\\n\", last error line \"last\""
    );
}

#[test]
fn a_program_that_cannot_start_is_reported() {
    let error = Observation::of(&mut Command::new("typy-no-such-program")).unwrap_err();
    assert!(error.starts_with("cannot start"), "{error}");
}

#[test]
fn a_signal_is_shown_as_such() {
    let observation = Observation::of(Command::new("sh").args(["-c", "kill -9 $$"])).unwrap();
    assert_eq!(observation.status, None);
    assert!(observation.to_string().starts_with("status a signal"));
}
