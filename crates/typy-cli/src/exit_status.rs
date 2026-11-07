//! The exit status of `typy` (CLI-06).

use std::process::ExitCode;

/// How a command ended; `typy run` passes on the program's own status instead.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ExitStatus {
    /// The command did what was asked.
    Success,
    /// The program has errors.
    ProgramErrors,
    /// The command line was wrong.
    Usage,
    /// The compiler failed internally (NFR-09).
    Internal,
    /// The status of the program `typy run` started.
    Program(u8),
}

impl ExitStatus {
    /// Returns the number the process exits with.
    pub(crate) const fn code(self) -> u8 {
        match self {
            Self::Success => 0,
            Self::ProgramErrors => 1,
            Self::Usage => 2,
            Self::Internal => 101,
            Self::Program(code) => code,
        }
    }
}

impl From<ExitStatus> for ExitCode {
    fn from(status: ExitStatus) -> Self {
        Self::from(status.code())
    }
}

#[cfg(test)]
mod tests {
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
}
