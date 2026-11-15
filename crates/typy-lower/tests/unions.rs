//! Tests of lowering unions and generics through the public API: every accepted union and
//! generic program of the conformance suite is checked, lowered and verified in process
//! (TYP-14, GEN-03, ADR-0023, ADR-0027).

use camino::Utf8PathBuf;
use typy_check::TypeChecker;
use typy_source::{Project, ProjectSettings};

/// Returns the accepted one-file conformance programs of a requirement in an area's
/// directory, each with its path.
fn accepted_programs(area: &str, requirement: &str) -> Vec<(Utf8PathBuf, String)> {
    let directory = Utf8PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/conformance")
        .join(area);
    let mut programs: Vec<(Utf8PathBuf, String)> = std::fs::read_dir(&directory)
        .unwrap()
        .map(|entry| Utf8PathBuf::from_path_buf(entry.unwrap().path()).unwrap())
        .filter(|path| {
            path.is_file()
                && path
                    .file_name()
                    .is_some_and(|name| name.starts_with(requirement))
        })
        .map(|path| {
            let text = std::fs::read_to_string(&path).unwrap();
            (path, text)
        })
        .filter(|(_, text)| text.contains("# typy-expect: accepted"))
        .collect();
    programs.sort();
    programs
}

#[test]
fn every_accepted_union_program_lowers_and_verifies() {
    let programs = accepted_programs("typ", "TYP-14_");
    assert!(programs.len() >= 8, "{programs:?}");
    lowers(programs);
}

#[test]
fn every_accepted_generic_program_lowers_and_verifies() {
    let programs = accepted_programs("gen", "GEN-0");
    assert!(programs.len() >= 2, "{programs:?}");
    lowers(programs);
}

/// Checks, lowers and verifies each program.
fn lowers(programs: Vec<(Utf8PathBuf, String)>) {
    for (path, _) in programs {
        let project = Project::load(
            &path,
            ProjectSettings::new(path.parent().unwrap().to_owned()),
        )
        .unwrap_or_else(|failure| panic!("{path}: {:?}", failure.diagnostics));
        let program = TypeChecker::check(&project)
            .unwrap_or_else(|diagnostics| panic!("{path}: rejected: {diagnostics:?}"));
        typy_lower::lower(&program).unwrap_or_else(|diagnostics| panic!("{path}: {diagnostics:?}"));
    }
}
