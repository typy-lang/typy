//! Tests of `linker`.

use super::Linker;
use crate::runtime_archive::RuntimeArchive;
use camino::{Utf8Path, Utf8PathBuf};
use typy_codegen::{ObjectFile, Target, TargetTriple};

fn archive(directory: &Utf8Path) -> RuntimeArchive {
    let path = directory.join("libtypy_runtime.a");
    std::fs::write(
        &path,
        format!("typy-runtime-version:{}\n", env!("CARGO_PKG_VERSION")),
    )
    .unwrap();
    RuntimeArchive::open(&path).unwrap()
}

fn directory() -> (tempfile::TempDir, Utf8PathBuf) {
    let directory = tempfile::tempdir().unwrap();
    let path = Utf8PathBuf::from_path_buf(directory.path().to_owned()).unwrap();
    (directory, path)
}

fn object() -> ObjectFile {
    ObjectFile {
        target: Target::host(),
        bytes: b"not an object".to_vec(),
    }
}

#[test]
fn each_target_links_with_its_toolchain() {
    let (_guard, root) = directory();
    // The host's own target links with `cc`; `toolchain.rs` tests the tools of the others.
    let host = Target::host().triple;
    let gnu = Linker::new(host, archive(&root));
    let arguments = gnu.arguments(Utf8Path::new("a.o"), Utf8Path::new("out"));
    let expected_archive = root.join("libtypy_runtime.a").to_string();
    assert_eq!(gnu.tools.linker, "cc");
    assert_eq!(
        arguments,
        [
            "-o",
            "out",
            "a.o",
            "-Wl,--push-state,--no-as-needed",
            "-lm",
            "-Wl,--pop-state",
            expected_archive.as_str(),
            "-lm",
            "-Wl,--gc-sections"
        ]
    );
    let stripped = Linker::new(host, archive(&root)).with_strip_debug(true);
    let arguments = stripped.arguments(Utf8Path::new("a.o"), Utf8Path::new("out"));
    assert_eq!(
        arguments.last().map(String::as_str),
        Some("-Wl,--strip-debug")
    );
    let pooled = Linker::new(host, archive(&root)).with_libraries(vec!["-lmimalloc".to_owned()]);
    let arguments = pooled.arguments(Utf8Path::new("a.o"), Utf8Path::new("out"));
    assert_eq!(
        arguments[6..],
        [
            expected_archive.as_str(),
            "-lmimalloc",
            "-lm",
            "-Wl,--gc-sections"
        ]
    );
    let musl = Linker::new(TargetTriple::X86_64LinuxMusl, archive(&root));
    assert_eq!(
        musl.arguments(Utf8Path::new("a.o"), Utf8Path::new("out"))[0],
        "-static"
    );
}

#[test]
fn a_missing_toolchain_program_is_named() {
    let (_guard, root) = directory();
    let mut linker = Linker::new(Target::host().triple, archive(&root));
    linker.tools.linker = "typy-no-such-linker".to_owned();
    let diagnostics = linker.link(&object(), &root.join("app")).unwrap_err();
    let diagnostic = diagnostics.iter().next().unwrap();
    assert_eq!(diagnostic.code().to_string(), "E0701");
    assert!(diagnostic.message().contains("`typy-no-such-linker`"));
}

#[test]
fn a_failed_link_leaves_no_file_behind() {
    let (_guard, root) = directory();
    // The host's own target links with `cc`.
    let linker = Linker::new(Target::host().triple, archive(&root));
    let output = root.join("app");
    let diagnostics = linker.link(&object(), &output).unwrap_err();
    let diagnostic = diagnostics.iter().next().unwrap();
    assert_eq!(diagnostic.code().to_string(), "E0702");
    assert!(
        diagnostic
            .help()
            .unwrap()
            .contains("the command was `cc -o")
    );
    let left: Vec<String> = std::fs::read_dir(&root)
        .unwrap()
        .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    assert_eq!(left, ["libtypy_runtime.a"]);
}

#[test]
fn an_output_in_a_missing_directory_is_refused() {
    let (_guard, root) = directory();
    let linker = Linker::new(Target::host().triple, archive(&root));
    let diagnostics = linker
        .link(&object(), &root.join("absent/app"))
        .unwrap_err();
    let diagnostic = diagnostics.iter().next().unwrap();
    assert_eq!(diagnostic.code().to_string(), "E0106");
    assert!(
        diagnostic
            .message()
            .starts_with("cannot write the executable in")
    );
}
