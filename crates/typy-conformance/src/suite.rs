//! Finds the tests under a directory, judges each one, and reports the results and which
//! requirements they cover (NFR-10).

use std::collections::BTreeMap;
use std::sync::Mutex;
use std::sync::atomic::{AtomicUsize, Ordering};

use camino::{Utf8Path, Utf8PathBuf};
use typy_driver::Pipeline;
use typy_source::PythonFloor;

use crate::expectation::Expectation;
use crate::oracle::run_oracles;
use crate::subject::{judge_accepted, judge_rejected, judge_sanitized};
use crate::test_case::TestCase;
use crate::verdict::Verdict;

/// How many accepted and rejected tests name one requirement.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Coverage {
    /// Tests of programs TyPy accepts.
    pub accepted: usize,
    /// Tests of programs TyPy rejects.
    pub rejected: usize,
}

/// The verdict of every test, and the coverage of every requirement a test names.
#[derive(Clone, Debug, Default)]
pub struct Report {
    /// Each test file and its verdict, sorted by path.
    pub results: Vec<(Utf8PathBuf, Verdict)>,
    /// For each requirement ID, the tests that name it.
    pub coverage: BTreeMap<String, Coverage>,
}

impl Report {
    /// Returns the tests that did not pass, with their verdicts.
    #[must_use]
    pub fn problems(&self) -> Vec<&(Utf8PathBuf, Verdict)> {
        self.results
            .iter()
            .filter(|(_, verdict)| *verdict != Verdict::Pass)
            .collect()
    }
}

/// Runs conformance tests with the compiler at one path.
#[derive(Clone, Debug)]
pub struct Suite {
    compiler: Utf8PathBuf,
    pipeline: Pipeline,
    sanitizers: bool,
}

impl Suite {
    /// Returns a suite that builds with the compiler at `compiler`, whose runtime archive is
    /// found beside it.
    #[must_use]
    pub fn new(compiler: impl Into<Utf8PathBuf>) -> Self {
        let compiler = compiler.into();
        Self {
            pipeline: Pipeline::new(compiler.clone()),
            compiler,
            sanitizers: false,
        }
    }

    /// Also builds each accepted test with AddressSanitizer and LeakSanitizer, which must show
    /// the same results with no report (NFR-07, NFR-08).
    #[must_use]
    pub fn with_sanitizers(mut self) -> Self {
        self.sanitizers = true;
        self
    }

    /// Judges every `.py` file under `root`, several at a time, and returns the report.
    pub fn run(&self, root: &Utf8Path) -> std::io::Result<Report> {
        let mut files = Vec::new();
        collect(root, &mut files)?;
        files.sort();
        let next = AtomicUsize::new(0);
        let results = Mutex::new(Vec::new());
        let workers = std::thread::available_parallelism().map_or(4, std::num::NonZero::get);
        std::thread::scope(|scope| {
            for _ in 0..workers {
                scope.spawn(|| {
                    while let Some(file) = files.get(next.fetch_add(1, Ordering::Relaxed)) {
                        let judged = self.judge(file);
                        if let Ok(mut results) = results.lock() {
                            results.push(judged);
                        }
                    }
                });
            }
        });
        let mut report = Report {
            results: Vec::new(),
            coverage: BTreeMap::new(),
        };
        let mut judged = results.into_inner().unwrap_or_default();
        judged.sort_by(|left, right| left.0.cmp(&right.0));
        for (path, case, verdict) in judged {
            if let Some(case) = case {
                for id in &case.requirements {
                    let entry = report.coverage.entry(id.clone()).or_default();
                    match case.expectation {
                        Expectation::Accepted => entry.accepted += 1,
                        Expectation::Rejected { .. } => entry.rejected += 1,
                    }
                }
            }
            report.results.push((path, verdict));
        }
        Ok(report)
    }

    /// Judges one file.
    fn judge(&self, file: &Utf8Path) -> (Utf8PathBuf, Option<TestCase>, Verdict) {
        let text = match std::fs::read_to_string(file) {
            Ok(text) => text,
            Err(error) => return (file.to_owned(), None, Verdict::Invalid(error.to_string())),
        };
        let case = match TestCase::parse(file, &text) {
            Ok(case) => case,
            Err(error) => return (file.to_owned(), None, Verdict::Invalid(error.to_string())),
        };
        let verdict = self.judge_case(&case);
        (file.to_owned(), Some(case), verdict)
    }

    fn judge_case(&self, case: &TestCase) -> Verdict {
        match case.expectation {
            Expectation::Rejected { code, line, column } => {
                judge_rejected(&self.pipeline, case, code, line, column)
            }
            Expectation::Accepted if case.floor != PythonFloor::default() => Verdict::Invalid(
                "a Python floor other than 3.12 needs `python-floor` in typy.toml (CLI-12, 0.3)"
                    .to_owned(),
            ),
            Expectation::Accepted => {
                let runs = run_oracles(&case.path, case.floor);
                let mut shown = Vec::new();
                for (version, run) in runs {
                    match run {
                        Ok(observation) => shown.push((version, observation)),
                        Err(reason) => {
                            return Verdict::Invalid(format!(
                                "CPython {} {reason}",
                                version.as_str()
                            ));
                        }
                    }
                }
                let Some((first_version, expected)) = shown.first() else {
                    return Verdict::Invalid("no oracle version runs this test".to_owned());
                };
                if let Some((version, other)) = shown
                    .iter()
                    .find(|(_, observation)| observation != expected)
                {
                    return Verdict::Invalid(format!(
                        "the oracles disagree: CPython {} shows {expected}; CPython {} shows {other}",
                        first_version.as_str(),
                        version.as_str()
                    ));
                }
                let verdict = judge_accepted(&self.pipeline, case, expected);
                if verdict != Verdict::Pass || !self.sanitizers {
                    return verdict;
                }
                judge_sanitized(&self.pipeline, &self.compiler, case, expected)
            }
        }
    }
}

/// Adds every `.py` file under `directory` to `files`.
fn collect(directory: &Utf8Path, files: &mut Vec<Utf8PathBuf>) -> std::io::Result<()> {
    for entry in directory.read_dir_utf8()? {
        let path = entry?.into_path();
        if path.is_dir() {
            collect(&path, files)?;
        } else if path.extension() == Some("py") {
            files.push(path);
        }
    }
    Ok(())
}
