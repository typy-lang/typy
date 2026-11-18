//! A conformance test: a Python file whose first comment lines name the requirements, the
//! expectation and the Python floor (detailed design section 15).

use std::fmt;

use camino::{Utf8Path, Utf8PathBuf};
use typy_source::PythonFloor;

use crate::expectation::Expectation;

/// Why a test file is not a valid conformance test.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CaseError {
    /// A header line is missing.
    Missing(&'static str),
    /// A header line has a value that does not parse.
    Malformed(&'static str, String),
    /// The file name does not start with the first requirement ID.
    Name(String),
}

impl fmt::Display for CaseError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Missing(field) => write!(formatter, "no `# {field}:` line"),
            Self::Malformed(field, value) => {
                write!(formatter, "`# {field}: {value}` does not parse")
            }
            Self::Name(id) => write!(formatter, "the file name does not start with {id}"),
        }
    }
}

impl std::error::Error for CaseError {}

/// One conformance test.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TestCase {
    /// The file.
    pub path: Utf8PathBuf,
    /// The requirement IDs it tests, the first one naming the file.
    pub requirements: Vec<String>,
    /// What TyPy shall do with it.
    pub expectation: Expectation,
    /// The oldest oracle version it runs under.
    pub floor: PythonFloor,
    /// Whether its output depends on the machine's architecture, as CPython's own does, so it
    /// is compared only with CPython on the architecture it runs on (NFR-16).
    pub architecture_dependent: bool,
    /// The one operating system it runs on, as `# typy-system: linux` or `macos` names it,
    /// for a test that names a library only that system has (NFR-17).
    pub system: Option<typy_source::Platform>,
}

/// Returns whether a directory holds one test of several modules: its name starts with a
/// requirement ID, as `MOD-01_shapes`, and its `main.py` holds the header.
pub(crate) fn is_case_directory(path: &Utf8Path) -> bool {
    path.file_name()
        .and_then(|name| name.split_once('_'))
        .is_some_and(|(id, _)| is_requirement_id(id))
}

/// Returns the name a test is known by: the file's, or for the `main.py` of a test of several
/// modules, its directory's.
fn case_name(path: &Utf8Path) -> Option<&str> {
    match path.file_name() {
        Some("main.py") if path.parent().is_some_and(is_case_directory) => {
            path.parent().and_then(Utf8Path::file_name)
        }
        name => name,
    }
}

fn is_requirement_id(text: &str) -> bool {
    text.split_once('-').is_some_and(|(area, number)| {
        (2..=4).contains(&area.len())
            && area.bytes().all(|byte| byte.is_ascii_uppercase())
            && number.len() == 2
            && number.bytes().all(|byte| byte.is_ascii_digit())
    })
}

impl TestCase {
    /// Reads the header of `text`, the contents of `path`: the comment lines at the top.
    pub fn parse(path: &Utf8Path, text: &str) -> Result<Self, CaseError> {
        let mut requirements = None;
        let mut expectation = None;
        let mut floor = PythonFloor::default();
        let mut architecture_dependent = false;
        let mut system = None;
        for line in text.lines().take_while(|line| line.starts_with('#')) {
            let Some((field, value)) = line
                .strip_prefix("# ")
                .and_then(|rest| rest.split_once(": "))
            else {
                continue;
            };
            let value = value.trim();
            match field {
                "typy-test" => {
                    let ids: Vec<String> =
                        value.split(',').map(|id| id.trim().to_owned()).collect();
                    if !ids.iter().all(|id| is_requirement_id(id)) {
                        return Err(CaseError::Malformed("typy-test", value.to_owned()));
                    }
                    requirements = Some(ids);
                }
                "typy-expect" => {
                    expectation =
                        Some(Expectation::parse(value).ok_or_else(|| {
                            CaseError::Malformed("typy-expect", value.to_owned())
                        })?);
                }
                "typy-python" => {
                    floor = PythonFloor::parse(value)
                        .ok_or_else(|| CaseError::Malformed("typy-python", value.to_owned()))?;
                }
                "typy-output" => {
                    if value != "architecture-dependent" {
                        return Err(CaseError::Malformed("typy-output", value.to_owned()));
                    }
                    architecture_dependent = true;
                }
                "typy-system" => {
                    system = Some(match value {
                        "linux" => typy_source::Platform::Linux,
                        "macos" => typy_source::Platform::MacOs,
                        _ => return Err(CaseError::Malformed("typy-system", value.to_owned())),
                    });
                }
                _ => {}
            }
        }
        let requirements = requirements.ok_or(CaseError::Missing("typy-test"))?;
        let expectation = expectation.ok_or(CaseError::Missing("typy-expect"))?;
        let first = &requirements[0];
        if !case_name(path).is_some_and(|name| name.starts_with(&format!("{first}_"))) {
            return Err(CaseError::Name(first.clone()));
        }
        Ok(Self {
            path: path.to_owned(),
            requirements,
            expectation,
            floor,
            architecture_dependent,
            system,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::{CaseError, TestCase};
    use crate::Expectation;
    use camino::Utf8Path;
    use typy_source::PythonFloor;

    #[test]
    fn the_header_gives_requirements_expectation_and_floor() {
        let text =
            "# typy-test: EXP-03, DAT-02\n# typy-expect: accepted\n# typy-python: 3.13\nprint(1)\n";
        let case = TestCase::parse(Utf8Path::new("exp/EXP-03_division.py"), text).unwrap();
        assert_eq!(case.requirements, ["EXP-03", "DAT-02"]);
        assert_eq!(case.expectation, Expectation::Accepted);
        assert_eq!(case.floor, PythonFloor::V3_13);
    }

    #[test]
    fn the_floor_defaults_to_3_12_and_the_header_ends_at_the_first_code_line() {
        let text = "# typy-test: EXP-04\n# typy-expect: rejected E0302 at 4:7\nn: int = 3\n# typy-python: 3.15\n";
        let case = TestCase::parse(Utf8Path::new("EXP-04_power.py"), text).unwrap();
        assert_eq!(case.floor, PythonFloor::V3_12);
        assert_eq!(case.expectation.to_string(), "rejected E0302 at 4:7");
    }

    #[test]
    fn a_system_header_names_the_one_system_a_test_runs_on() {
        for (value, system) in [
            ("linux", typy_source::Platform::Linux),
            ("macos", typy_source::Platform::MacOs),
        ] {
            let text =
                format!("# typy-test: FFI-01\n# typy-expect: accepted\n# typy-system: {value}\n");
            let case = TestCase::parse(Utf8Path::new("FFI-01_a.py"), &text).unwrap();
            assert_eq!(case.system, Some(system), "{value}");
        }
        let plain = TestCase::parse(
            Utf8Path::new("FFI-01_a.py"),
            "# typy-test: FFI-01\n# typy-expect: accepted\n",
        )
        .unwrap();
        assert_eq!(plain.system, None);
        assert_eq!(
            TestCase::parse(
                Utf8Path::new("FFI-01_a.py"),
                "# typy-test: FFI-01\n# typy-expect: accepted\n# typy-system: windows\n",
            ),
            Err(CaseError::Malformed("typy-system", "windows".into()))
        );
    }

    #[test]
    fn a_wrong_header_or_name_is_refused() {
        let cases = [
            (
                "EXP-03_a.py",
                "# typy-expect: accepted\n",
                CaseError::Missing("typy-test"),
            ),
            (
                "EXP-03_a.py",
                "# typy-test: EXP-03\n",
                CaseError::Missing("typy-expect"),
            ),
            (
                "EXP-03_a.py",
                "# typy-test: exp-3\n",
                CaseError::Malformed("typy-test", "exp-3".into()),
            ),
            (
                "EXP-03_a.py",
                "# typy-test: E-03\n",
                CaseError::Malformed("typy-test", "E-03".into()),
            ),
            (
                "EXP-03_a.py",
                "# typy-test: ABCDE-03\n",
                CaseError::Malformed("typy-test", "ABCDE-03".into()),
            ),
            (
                "EXP-03_a.py",
                "# typy-test: EXP-3\n",
                CaseError::Malformed("typy-test", "EXP-3".into()),
            ),
            (
                "EXP-03_a.py",
                "# typy-test: EXP-003\n",
                CaseError::Malformed("typy-test", "EXP-003".into()),
            ),
            (
                "EXP-03_a.py",
                "# typy-test: EXP03\n",
                CaseError::Malformed("typy-test", "EXP03".into()),
            ),
            (
                "EXP-03_a.py",
                "# typy-test: EXP-03\n# typy-expect: maybe\n",
                CaseError::Malformed("typy-expect", "maybe".into()),
            ),
            (
                "EXP-03_a.py",
                "# typy-test: EXP-03\n# typy-expect: accepted\n# typy-python: 3.11\n",
                CaseError::Malformed("typy-python", "3.11".into()),
            ),
            (
                "EXP-04_a.py",
                "# typy-test: EXP-03\n# typy-expect: accepted\n",
                CaseError::Name("EXP-03".into()),
            ),
        ];
        for (name, text, error) in cases {
            assert_eq!(
                TestCase::parse(Utf8Path::new(name), text),
                Err(error.clone()),
                "{text:?}"
            );
            assert_ne!(error.to_string(), "");
        }
    }
}
