//! Tests of `test_case`.

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
