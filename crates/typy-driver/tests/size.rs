//! NFR-01: a hello world built with `--release-small` and linked statically with musl is
//! smaller than 100 KB.

use camino::Utf8PathBuf;
use typy_build::{BuildMode, BuildRequest};
use typy_codegen::{Target, TargetTriple};
use typy_driver::{Outcome, Pipeline};

const LIMIT: u64 = 100 * 1024;

#[test]
fn a_static_musl_hello_world_is_smaller_than_100_kb() {
    let directory = tempfile::tempdir().unwrap();
    let root = Utf8PathBuf::from_path_buf(directory.path().to_owned()).unwrap();
    std::fs::write(root.join("hello.py"), "print(\"Hello, world!\")\n").unwrap();
    let target = option_env!("CARGO_TARGET_DIR").map_or_else(
        || Utf8PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../target"),
        Utf8PathBuf::from,
    );
    let request = BuildRequest::new(root.join("hello.py"))
        .unwrap()
        .with_mode(BuildMode::ReleaseSmall)
        .with_target(Target {
            triple: TargetTriple::X86_64LinuxMusl,
        });
    let outcome = Pipeline::new(target.join("debug/typy")).run(&request);
    let Ok(Outcome::Executable(artifact)) = outcome else {
        panic!("{outcome:?}; run `make runtime`");
    };
    let output = std::process::Command::new(&artifact.path).output().unwrap();
    assert_eq!(output.stdout, b"Hello, world!\n");
    let size = std::fs::metadata(&artifact.path).unwrap().len();
    assert!(size < LIMIT, "{size} bytes, over the {LIMIT} of NFR-01");
}
