//! Tests of `mach_o`.

use camino::{Utf8Path, Utf8PathBuf};
use typy_codegen::{ObjectFile, Target, TargetTriple};

use super::{Work, exports_list};
use crate::linker::Linker;
use crate::runtime_archive::RuntimeArchive;

fn linker(directory: &Utf8Path, strip_debug: bool) -> Linker {
    let path = directory.join("libtypy_runtime.a");
    std::fs::write(
        &path,
        format!("typy-runtime-version:{}\n", env!("CARGO_PKG_VERSION")),
    )
    .unwrap();
    Linker::new(
        TargetTriple::Aarch64AppleDarwin,
        RuntimeArchive::open(&path).unwrap(),
    )
    .with_strip_debug(strip_debug)
    .with_libraries(vec!["-ltools".to_owned()])
    .with_library_paths(vec![Utf8PathBuf::from("/project/lib")])
}

#[test]
fn a_debug_link_records_the_object_from_the_work_directory() {
    let directory = tempfile::tempdir().unwrap();
    let root = Utf8PathBuf::from_path_buf(directory.path().to_owned()).unwrap();
    let linker = linker(&root, false);
    let arguments = linker.mach_o_arguments(Utf8Path::new("/w/program.o"), Utf8Path::new("/w"));
    let archive = root.join("libtypy_runtime.a").to_string();
    assert_eq!(
        arguments,
        [
            "-arch",
            "arm64",
            "-mmacosx-version-min=11.0",
            "-L/project/lib",
            "/w/program.o",
            "-lm",
            archive.as_str(),
            "-ltools",
            "-Wl,-dead_strip",
            "-Wl,-oso_prefix,/w/",
        ]
    );
}

#[test]
fn a_release_link_leaves_out_the_debug_map() {
    let directory = tempfile::tempdir().unwrap();
    let root = Utf8PathBuf::from_path_buf(directory.path().to_owned()).unwrap();
    let arguments =
        linker(&root, true).mach_o_arguments(Utf8Path::new("/w/program.o"), Utf8Path::new("/w"));
    assert_eq!(arguments.last().map(String::as_str), Some("-Wl,-S"));
    assert!(
        !arguments
            .iter()
            .any(|argument| argument.contains("oso_prefix"))
    );
    assert!(
        !arguments
            .iter()
            .any(|argument| argument.starts_with("-Wl,--"))
    );
}

#[test]
fn the_work_directory_has_its_real_path_and_the_object_a_fixed_time() {
    let directory = tempfile::tempdir().unwrap();
    let root = Utf8PathBuf::from_path_buf(directory.path().to_owned()).unwrap();
    let output = root.join(".").join("app");
    let object = ObjectFile {
        target: Target {
            triple: TargetTriple::Aarch64AppleDarwin,
        },
        bytes: vec![1, 2, 3],
    };
    let work = Work::beside(&output, &object, "the executable").unwrap();
    let real = std::fs::canonicalize(&work.path).unwrap();
    assert_eq!(work.path.as_std_path(), real);
    assert!(!work.path.as_str().contains("/./"), "{}", work.path);
    let modified = std::fs::metadata(work.object())
        .unwrap()
        .modified()
        .unwrap();
    assert_eq!(modified, std::time::SystemTime::UNIX_EPOCH);
    assert_eq!(std::fs::read(work.object()).unwrap(), [1, 2, 3]);
}

#[test]
fn the_exports_list_names_each_symbol_as_mach_o_spells_it() {
    let directory = tempfile::tempdir().unwrap();
    let root = Utf8PathBuf::from_path_buf(directory.path().to_owned()).unwrap();
    let path = exports_list(&root, &["add".to_owned(), "scale".to_owned()]).unwrap();
    assert_eq!(std::fs::read_to_string(path).unwrap(), "_add\n_scale\n");
    let empty = exports_list(&root, &[]).unwrap();
    assert_eq!(std::fs::read_to_string(empty).unwrap(), "");
}
