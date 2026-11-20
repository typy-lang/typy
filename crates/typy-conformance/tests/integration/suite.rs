//! Runs the conformance suite in `tests/conformance` and checks that every requirement the test
//! plan gives to it has an accepted and a rejected test (UC-09, NFR-10).

use std::collections::BTreeSet;

use camino::Utf8PathBuf;
use typy_conformance::{Report, Suite};

/// Requirements whose rejected case cannot be a file in the corpus. A file that is not UTF-8
/// breaks the repository's ASCII rule, and a file over 16 MiB is too large to commit, so unit
/// tests in `typy-source` cover LEX-01 and LEX-09. Memory management asks nothing of a
/// program, so MEM-01, MEM-02, MEM-03, MEM-04 and MEM-09 have no rule a program can break;
/// nor have recursion (FUN-02), a surrogate escape (LEX-10), the order expressions evaluate in (EXP-01), the order
/// modules initialise in (MOD-03), the report of an exception nothing caught (STD-05) and the
/// class and text of an error the C library reports (EXC-19), which describe what an accepted
/// program does, nor a warning of a `match` that leaves members unmatched (CST-07), which
/// accepts the program.
const NO_REJECTED_FILE: [&str; 14] = [
    "LEX-01", "LEX-09", "LEX-10", "EXP-01", "FUN-02", "CST-07", "MEM-01", "MEM-02", "MEM-03",
    "MEM-04", "MEM-09", "MOD-03", "STD-05", "EXC-19",
];

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

/// The releases whose requirements the suite must cover: every one up to the version being
/// built.
const RELEASES: [&str; 9] = [
    "0.1", "0.2", "0.3", "0.4", "0.5", "0.6", "0.7", "0.8", "0.9",
];

/// Returns the requirements the test plan's matrix gives to the conformance suite for the
/// releases up to this one.
fn planned_requirements() -> BTreeSet<String> {
    let plan = std::fs::read_to_string(workspace().join("docs/testing/test-plan.md")).unwrap();
    plan.lines()
        .filter(|line| {
            RELEASES.iter().any(|release| {
                line.contains(&format!("| {release} | IT | `tests/conformance/"))
                    || line.contains(&format!("| {release} | UT, IT | "))
                        && line.contains("`tests/conformance/")
            })
        })
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
