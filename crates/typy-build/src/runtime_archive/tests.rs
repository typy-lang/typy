//! Tests of `runtime_archive`.

use super::{RuntimeArchive, VERSION};
use crate::BuildMode;
use camino::Utf8PathBuf;
use typy_codegen::TargetTriple;

fn directory() -> (tempfile::TempDir, Utf8PathBuf) {
    let directory = tempfile::tempdir().unwrap();
    let path = Utf8PathBuf::from_path_buf(directory.path().to_owned()).unwrap();
    (directory, path)
}

fn message(result: Result<RuntimeArchive, typy_diagnostics::Diagnostics>) -> String {
    let diagnostics = result.unwrap_err();
    let diagnostic = diagnostics.iter().next().unwrap();
    assert_eq!(diagnostic.code().to_string(), "E0703");
    diagnostic.message().to_owned()
}

#[test]
fn the_archive_beside_the_compiler_is_found() {
    let (_guard, root) = directory();
    let archive = root.join("lib/typy/x86_64-unknown-linux-gnu/libtypy_runtime.a");
    std::fs::create_dir_all(archive.parent().unwrap()).unwrap();
    std::fs::write(
        &archive,
        format!("!<arch>\ntypy-runtime-version:{VERSION}\nrest"),
    )
    .unwrap();
    let found = RuntimeArchive::beside(
        &root.join("bin/typy"),
        TargetTriple::X86_64LinuxGnu,
        BuildMode::Release,
    )
    .unwrap();
    assert_eq!(found.path(), archive);
}

#[test]
fn a_build_for_size_finds_the_archive_optimised_for_size() {
    let (_guard, root) = directory();
    let small = root.join("lib/typy-small/x86_64-unknown-linux-gnu/libtypy_runtime.a");
    std::fs::create_dir_all(small.parent().unwrap()).unwrap();
    std::fs::write(
        &small,
        format!("!<arch>\ntypy-runtime-version:{VERSION}\nrest"),
    )
    .unwrap();
    let found = RuntimeArchive::beside(
        &root.join("bin/typy"),
        TargetTriple::X86_64LinuxGnu,
        BuildMode::ReleaseSmall,
    )
    .unwrap();
    assert_eq!(found.path(), small);
    assert!(
        RuntimeArchive::beside(
            &root.join("bin/typy"),
            TargetTriple::X86_64LinuxGnu,
            BuildMode::Debug
        )
        .is_err()
    );
}

#[test]
fn a_missing_or_foreign_archive_is_refused() {
    let (_guard, root) = directory();
    let path = root.join("libtypy_runtime.a");
    assert!(message(RuntimeArchive::open(&path)).starts_with("cannot read the runtime archive"));
    std::fs::write(&path, "!<arch>\nnothing").unwrap();
    assert_eq!(
        message(RuntimeArchive::open(&path)),
        format!("{path} is not a TyPy runtime archive")
    );
    std::fs::write(&path, "typy-runtime-version:9.9.9\n").unwrap();
    assert_eq!(
        message(RuntimeArchive::open(&path)),
        format!("the runtime archive {path} is from TyPy 9.9.9, not {VERSION}")
    );
    let missing = RuntimeArchive::beside(
        &root.join("typy"),
        TargetTriple::X86_64LinuxMusl,
        BuildMode::Debug,
    );
    assert!(message(missing).contains("x86_64-unknown-linux-musl"));
}
