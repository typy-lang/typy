//! Runs every accepted conformance test built for the other architecture, under its user-mode
//! emulator, as a developer's machine of one architecture builds for the other: each must show
//! CPython's results (NFR-16, ADR-0051). On the arm64 CI runner the suite's own run covers
//! AArch64 natively, and this one covers x86-64 under emulation.

use camino::Utf8PathBuf;
use typy_codegen::{Target, TargetTriple};
use typy_conformance::Suite;

#[test]
fn every_accepted_test_runs_alike_built_for_the_other_architecture() {
    let workspace = Utf8PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let target =
        option_env!("CARGO_TARGET_DIR").map_or_else(|| workspace.join("target"), Utf8PathBuf::from);
    let other = if Target::host().triple.architecture() == "aarch64" {
        TargetTriple::X86_64LinuxGnu
    } else {
        TargetTriple::Aarch64LinuxGnu
    };
    let report = Suite::new(target.join("debug/typy"))
        .with_target(other)
        .run(&workspace.join("tests/conformance"))
        .unwrap();
    let problems: Vec<String> = report
        .problems()
        .iter()
        .map(|(path, verdict)| format!("{path}: {verdict}"))
        .collect();
    assert!(problems.is_empty(), "{}", problems.join("\n"));
}
