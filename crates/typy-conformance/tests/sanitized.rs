//! Runs every accepted conformance test once more, linked with AddressSanitizer and
//! LeakSanitizer: each must show CPython's results with no sanitizer report (NFR-07, NFR-08).

use camino::Utf8PathBuf;
use typy_conformance::Suite;

#[test]
fn every_accepted_test_runs_clean_under_the_sanitizers() {
    let workspace = Utf8PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let target =
        option_env!("CARGO_TARGET_DIR").map_or_else(|| workspace.join("target"), Utf8PathBuf::from);
    let report = Suite::new(target.join("debug/typy"))
        .with_sanitizers()
        .run(&workspace.join("tests/conformance"))
        .unwrap();
    let problems: Vec<String> = report
        .problems()
        .iter()
        .map(|(path, verdict)| format!("{path}: {verdict}"))
        .collect();
    assert!(problems.is_empty(), "{}", problems.join("\n"));
}
