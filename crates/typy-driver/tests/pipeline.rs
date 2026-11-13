//! Tests of the pipeline through its public API: checks, every `--emit` stage, and full builds.

use camino::{Utf8Path, Utf8PathBuf};
use typy_build::{ArtifactKind, BuildRequest, EmitStage};
use typy_diagnostics::SourceLookup;
use typy_driver::{Emission, Failure, Outcome, Pipeline, Sources};

/// A directory holding `app.py` with `source`.
fn program(source: &str) -> (tempfile::TempDir, Utf8PathBuf) {
    let directory = tempfile::tempdir().unwrap();
    let root = Utf8PathBuf::from_path_buf(directory.path().to_owned()).unwrap();
    std::fs::write(root.join("app.py"), source).unwrap();
    (directory, root.join("app.py"))
}

/// A pipeline whose compiler sits in Cargo's target directory, where `make runtime` stages the
/// archive.
fn pipeline() -> Pipeline {
    let target = option_env!("CARGO_TARGET_DIR").map_or_else(
        || Utf8PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../target"),
        Utf8PathBuf::from,
    );
    Pipeline::new(target.join("debug/typy"))
}

fn codes(failure: &Failure) -> Vec<String> {
    failure
        .diagnostics
        .iter()
        .map(|diagnostic| diagnostic.code().to_string())
        .collect()
}

fn emit(main: &Utf8Path, stage: EmitStage) -> Emission {
    let request = BuildRequest::new(main).unwrap().with_emit(stage);
    match pipeline().run(&request).map(|success| success.outcome) {
        Ok(Outcome::Emitted(emission)) => emission,
        other => panic!("{other:?}"),
    }
}

fn text(emission: Emission) -> String {
    match emission {
        Emission::Text(text) => text,
        Emission::Bytes(_) => panic!("bytes"),
    }
}

const SOURCE: &str = "s = \"a\"\ns = s + \"b\"\nprint(s, 1 + 2)\n";

#[test]
fn a_correct_program_checks() {
    let (_guard, main) = program(SOURCE);
    match pipeline().check(&main).map(|success| success.outcome) {
        Ok(Outcome::Checked(Sources::Project(project))) => {
            assert_eq!(project.path(typy_diagnostics::FileId(0)), "app.py");
        }
        other => panic!("{other:?}"),
    }
}

#[test]
fn a_wrong_program_fails_with_its_diagnostics_and_files() {
    let (_guard, main) = program("x: int = \"a\"\n");
    let failure = pipeline().check(&main).unwrap_err();
    assert_eq!(codes(&failure), ["E0402"]);
    assert!(matches!(failure.sources, Sources::Project(_)));
}

#[test]
fn a_missing_main_file_fails_with_the_files_it_tried() {
    let (_guard, main) = program("");
    let failure = pipeline()
        .check(&main.with_file_name("absent.py"))
        .unwrap_err();
    assert_eq!(codes(&failure), ["E0105"]);
    assert!(matches!(failure.sources, Sources::Unloaded(_)));
}

#[test]
fn every_stage_emits_the_same_output_twice() {
    let (_guard, main) = program(SOURCE);
    for stage in EmitStage::ALL {
        let first = emit(&main, stage);
        assert!(!first.as_bytes().is_empty(), "{stage:?}");
        assert_eq!(first, emit(&main, stage), "{stage:?}");
    }
}

#[test]
fn each_stage_writes_its_own_form() {
    let (_guard, main) = program(SOURCE);
    assert!(text(emit(&main, EmitStage::Ast)).starts_with("Module\n"));
    assert!(text(emit(&main, EmitStage::TypedAst)).starts_with("Program entry fn0\n"));
    let tyir = text(emit(&main, EmitStage::Tyir));
    let tyir_mem = text(emit(&main, EmitStage::TyirMem));
    assert!(
        !tyir.contains("dec v") && tyir_mem.contains("dec v"),
        "{tyir_mem}"
    );
    assert!(text(emit(&main, EmitStage::LlvmIr)).contains("define"));
    assert!(text(emit(&main, EmitStage::Asm)).contains("__typy_main"));
    assert!(
        emit(&main, EmitStage::Obj)
            .as_bytes()
            .starts_with(b"\x7fELF")
    );
}

#[test]
fn a_stage_after_checking_needs_a_correct_program() {
    let (_guard, main) = program("x: int = \"a\"\n");
    let syntax = emit(&main, EmitStage::Ast);
    assert!(syntax.as_bytes().starts_with(b"Module\n"));
    let request = BuildRequest::new(&main).unwrap().with_emit(EmitStage::Tyir);
    assert_eq!(codes(&pipeline().run(&request).unwrap_err()), ["E0402"]);
}

#[test]
fn a_build_links_an_executable_that_runs() {
    let (_guard, main) = program(SOURCE);
    let request = BuildRequest::new(&main).unwrap();
    let Ok(Outcome::Executable(artifact)) = pipeline().run(&request).map(|success| success.outcome)
    else {
        panic!("no executable; run `make runtime`");
    };
    assert_eq!(
        (artifact.kind, artifact.path.as_str()),
        (ArtifactKind::Executable, main.with_extension("").as_str())
    );
    let output = std::process::Command::new(&artifact.path).output().unwrap();
    assert_eq!(String::from_utf8(output.stdout).unwrap(), "ab 3\n");
}

#[test]
fn a_missing_runtime_archive_fails_before_the_program_is_read() {
    let (guard, main) = program("");
    let compiler = Utf8PathBuf::from_path_buf(guard.path().join("bin/typy")).unwrap();
    let request = BuildRequest::new(main.with_file_name("absent.py")).unwrap();
    let failure = Pipeline::new(compiler).run(&request).unwrap_err();
    assert_eq!(codes(&failure), ["E0703"]);
    assert!(matches!(failure.sources, Sources::None));
}
