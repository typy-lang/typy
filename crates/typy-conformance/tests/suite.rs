//! Runs the conformance suite in `tests/conformance` and checks that every requirement the test
//! plan gives to it has an accepted and a rejected test (UC-09, NFR-10).

use std::collections::BTreeSet;

use camino::Utf8PathBuf;
use typy_conformance::{Report, Suite};

/// Requirements whose rejected case cannot be a file in the corpus, and the unit tests in
/// `typy-source` that cover it instead: a file that is not UTF-8 breaks the repository's ASCII
/// rule, and a file over 16 MiB is too large to commit.
const NO_REJECTED_FILE: [&str; 2] = ["LEX-01", "LEX-09"];

fn workspace() -> Utf8PathBuf {
    Utf8PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn run() -> Report {
    let target = option_env!("CARGO_TARGET_DIR")
        .map_or_else(|| workspace().join("target"), Utf8PathBuf::from);
    Suite::new(target.join("debug/typy"))
        .run(&workspace().join("tests/conformance"))
        .unwrap()
}

/// Returns the requirements the test plan's matrix gives to the conformance suite for 0.1.
fn planned_requirements() -> BTreeSet<String> {
    let plan = std::fs::read_to_string(workspace().join("docs/testing/test-plan.md")).unwrap();
    plan.lines()
        .filter(|line| line.contains("| 0.1 | IT | `tests/conformance/"))
        .filter_map(|line| line.split('|').nth(1).map(|id| id.trim().to_owned()))
        .collect()
}

#[test]
fn every_conformance_test_passes_and_every_planned_requirement_has_both_kinds() {
    let report = run();
    let problems: Vec<String> = report
        .problems()
        .iter()
        .map(|(path, verdict)| format!("{path}: {verdict}"))
        .collect();
    assert!(problems.is_empty(), "{}", problems.join("\n"));
    let mut missing = Vec::new();
    for id in planned_requirements() {
        let coverage = report.coverage.get(&id).copied().unwrap_or_default();
        if coverage.accepted == 0 {
            missing.push(format!("{id} has no accepted test"));
        }
        if coverage.rejected == 0 && !NO_REJECTED_FILE.contains(&id.as_str()) {
            missing.push(format!("{id} has no rejected test"));
        }
    }
    assert!(missing.is_empty(), "{}", missing.join("\n"));
}
