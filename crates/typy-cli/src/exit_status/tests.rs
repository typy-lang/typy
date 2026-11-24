//! Tests of `exit_status`.

use super::ExitStatus;

#[test]
fn each_status_has_the_number_of_cli_06() {
    let cases = [
        (ExitStatus::Success, 0),
        (ExitStatus::ProgramErrors, 1),
        (ExitStatus::Usage, 2),
        (ExitStatus::Internal, 101),
        (ExitStatus::Program(7), 7),
    ];
    for (status, code) in cases {
        assert_eq!(status.code(), code, "{status:?}");
    }
}
