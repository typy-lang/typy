//! Golden tests: the text of each `--emit` stage for each program in `tests/golden/programs/`,
//! and the rendered diagnostics of each program in `tests/golden/diagnostics/` (CLI-04,
//! DIA-01 to DIA-04). An expected file changes only through `make bless`, which sets
//! `TYPY_BLESS=1`, and the change is reviewed in the same commit.

use camino::{Utf8Path, Utf8PathBuf};
use typy_build::{BuildRequest, EmitStage};
use typy_diagnostics::render_text;
use typy_driver::{Emission, Outcome, Pipeline};

const TEXT_STAGES: [EmitStage; 6] = [
    EmitStage::Ast,
    EmitStage::TypedAst,
    EmitStage::Tyir,
    EmitStage::TyirMem,
    EmitStage::LlvmIr,
    EmitStage::Asm,
];

fn golden() -> Utf8PathBuf {
    Utf8PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../tests/golden")
}

fn blessing() -> bool {
    std::env::var("TYPY_BLESS").is_ok_and(|value| value == "1")
}

/// The `.py` files directly in `directory`, sorted.
fn programs(directory: &Utf8Path) -> Vec<Utf8PathBuf> {
    let mut files: Vec<Utf8PathBuf> = directory
        .read_dir_utf8()
        .unwrap()
        .map(|entry| entry.unwrap().into_path())
        .filter(|path| path.extension() == Some("py"))
        .collect();
    files.sort();
    assert!(!files.is_empty(), "no programs in {directory}");
    files
}

/// Compares `actual` with the file at `expected`, or writes it when blessing; returns a
/// description of the difference.
fn compare(expected: &Utf8Path, actual: &str) -> Option<String> {
    if blessing() {
        std::fs::create_dir_all(expected.parent().unwrap()).unwrap();
        std::fs::write(expected, actual).unwrap();
        return None;
    }
    let Ok(wanted) = std::fs::read_to_string(expected) else {
        return Some(format!("{expected} is missing; run `make bless`"));
    };
    if wanted == actual {
        return None;
    }
    let line = wanted
        .lines()
        .zip(actual.lines())
        .position(|(left, right)| left != right)
        .unwrap_or_else(|| wanted.lines().count().min(actual.lines().count()));
    Some(format!(
        "{expected} differs at line {}:\n  expected: {:?}\n  actual:   {:?}",
        line + 1,
        wanted.lines().nth(line).unwrap_or("<end>"),
        actual.lines().nth(line).unwrap_or("<end>")
    ))
}

fn pipeline() -> Pipeline {
    Pipeline::new("typy")
}

#[test]
fn every_text_stage_of_every_program_matches_its_golden_file() {
    let mut problems = Vec::new();
    for program in programs(&golden().join("programs")) {
        let name = program.file_stem().unwrap();
        for stage in TEXT_STAGES {
            let request = BuildRequest::new(&program).unwrap().with_emit(stage);
            let text = match pipeline().run(&request).map(|success| success.outcome) {
                Ok(Outcome::Emitted(Emission::Text(text))) => text,
                other => panic!("{program} --emit={}: {other:?}", stage.name()),
            };
            let expected = golden().join(stage.name()).join(format!("{name}.txt"));
            problems.extend(compare(&expected, &text));
        }
    }
    assert!(problems.is_empty(), "{}", problems.join("\n"));
}

#[test]
fn every_diagnostic_program_renders_as_its_golden_file() {
    let mut problems = Vec::new();
    for program in programs(&golden().join("diagnostics")) {
        let Err(failure) = pipeline().check(&program) else {
            panic!("{program} has no diagnostic");
        };
        let sorted = failure.diagnostics.into_sorted(&failure.sources);
        let text: Vec<String> = sorted
            .iter()
            .map(|diagnostic| render_text(diagnostic, &failure.sources))
            .collect();
        problems.extend(compare(&program.with_extension("txt"), &text.join("\n")));
    }
    assert!(problems.is_empty(), "{}", problems.join("\n"));
}

#[test]
fn every_golden_file_belongs_to_a_program() {
    let names: Vec<String> = programs(&golden().join("programs"))
        .iter()
        .map(|path| path.file_stem().unwrap().to_owned())
        .collect();
    let mut orphans = Vec::new();
    for stage in TEXT_STAGES {
        let directory = golden().join(stage.name());
        let Ok(entries) = directory.read_dir_utf8() else {
            continue;
        };
        for entry in entries {
            let path = entry.unwrap().into_path();
            if !path
                .file_stem()
                .is_some_and(|stem| names.iter().any(|name| name == stem))
            {
                orphans.push(path.to_string());
            }
        }
    }
    for entry in golden().join("diagnostics").read_dir_utf8().unwrap() {
        let path = entry.unwrap().into_path();
        if path.extension() == Some("txt") && !path.with_extension("py").exists() {
            orphans.push(path.to_string());
        }
    }
    assert!(
        orphans.is_empty(),
        "golden files without a program: {orphans:?}"
    );
}
