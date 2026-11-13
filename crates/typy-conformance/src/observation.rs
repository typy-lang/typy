//! What one run of a program showed: the parts STD-02 and STD-05 compare.

use std::fmt;
use std::io::Read;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

/// A program longer than this has hung. Every conformance program finishes in well under a
/// second, but the gate runs several suites at once, each with a worker per CPU, and a starved
/// oracle can take many times longer; the limit catches a hang without failing a slow run.
const TIME_LIMIT: Duration = Duration::from_secs(60);

/// Standard output, exit status and the last line of standard error of one run.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Observation {
    /// Everything written to standard output.
    pub stdout: String,
    /// The exit status, or `None` when a signal ended the program.
    pub status: Option<i32>,
    /// The last line of standard error, empty when it wrote nothing.
    pub last_error_line: String,
}

impl Observation {
    /// Returns what was shown with each `prefix` removed: CPython names a module's file by
    /// its absolute path, where TyPy names it from the project root (OI-20).
    #[must_use]
    pub fn without_prefix(self, prefix: &str) -> Self {
        Self {
            stdout: self.stdout.replace(prefix, ""),
            last_error_line: self.last_error_line.replace(prefix, ""),
            ..self
        }
    }

    /// Runs `command` with no standard input and returns what it showed, or a reason it could not
    /// be observed: it did not start, or it ran past the time limit and was stopped.
    pub fn of(command: &mut Command) -> Result<Self, String> {
        let mut child = command
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|error| {
                format!("cannot start {}: {error}", command.get_program().display())
            })?;
        let stdout = child.stdout.take();
        let stderr = child.stderr.take();
        // Each pipe is read on its own thread, so a program that fills one is never blocked.
        let (out, error, status) = std::thread::scope(|scope| {
            let out = scope.spawn(move || read_all(stdout));
            let error = scope.spawn(move || read_all(stderr));
            let started = Instant::now();
            let status = loop {
                match child.try_wait() {
                    Ok(Some(status)) => break Ok(status),
                    Ok(None) if started.elapsed() > TIME_LIMIT => {
                        let _ = child.kill();
                        let _ = child.wait();
                        break Err(format!("ran longer than {} seconds", TIME_LIMIT.as_secs()));
                    }
                    Ok(None) => std::thread::sleep(Duration::from_millis(5)),
                    Err(failure) => break Err(format!("cannot wait for the program: {failure}")),
                }
            };
            (
                out.join().unwrap_or_default(),
                error.join().unwrap_or_default(),
                status,
            )
        });
        let status = status?;
        Ok(Self {
            stdout: out,
            status: status.code(),
            last_error_line: error.lines().last().unwrap_or("").to_owned(),
        })
    }
}

/// Returns everything a pipe delivers, as text; invalid UTF-8 is replaced, not refused.
fn read_all(pipe: Option<impl Read>) -> String {
    let mut bytes = Vec::new();
    if let Some(mut pipe) = pipe {
        let _ = pipe.read_to_end(&mut bytes);
    }
    String::from_utf8_lossy(&bytes).into_owned()
}

impl fmt::Display for Observation {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let status = self
            .status
            .map_or_else(|| "a signal".to_owned(), |code| code.to_string());
        write!(
            formatter,
            "status {status}, stdout {:?}, last error line {:?}",
            self.stdout, self.last_error_line
        )
    }
}

#[cfg(test)]
mod tests {
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
}
