//! Builds every accepted conformance program twice, in debug and in release mode, and checks
//! that the two executables of each are byte for byte the same (NFR-06).

use std::process::Command;
use std::sync::Mutex;
use std::sync::atomic::{AtomicUsize, Ordering};

use camino::{Utf8Path, Utf8PathBuf};

fn workspace() -> Utf8PathBuf {
    Utf8PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// Returns every accepted conformance program under `directory`.
fn accepted(directory: &Utf8Path, found: &mut Vec<Utf8PathBuf>) {
    for entry in directory.read_dir_utf8().unwrap() {
        let path = entry.unwrap().into_path();
        if path.is_dir() {
            accepted(&path, found);
        } else if path.extension() == Some("py")
            && std::fs::read_to_string(&path)
                .unwrap()
                .lines()
                .any(|line| line == "# typy-expect: accepted")
        {
            found.push(path);
        }
    }
}

/// Builds `program` with `flags`, at the Python floor its header names, into `output`, and
/// returns the executable's bytes.
fn build(compiler: &Utf8Path, program: &Utf8Path, flags: &[&str], output: &Utf8Path) -> Vec<u8> {
    let text = std::fs::read_to_string(program).unwrap();
    let floor = text
        .lines()
        .find_map(|line| line.strip_prefix("# typy-python: "))
        .unwrap_or("3.12");
    let status = Command::new(compiler)
        .arg("build")
        .args(flags)
        .args(["--python-floor", floor])
        .arg(program)
        .arg("-o")
        .arg(output)
        .status()
        .unwrap();
    assert!(status.success(), "{program} does not build with {flags:?}");
    std::fs::read(output).unwrap()
}

#[test]
fn every_program_builds_to_the_same_bytes_twice() {
    let target = option_env!("CARGO_TARGET_DIR")
        .map_or_else(|| workspace().join("target"), Utf8PathBuf::from);
    let compiler = target.join("debug/typy");
    let mut programs = Vec::new();
    accepted(&workspace().join("tests/conformance"), &mut programs);
    programs.sort();
    assert_ne!(programs.len(), 0, "the corpus has accepted programs");
    let scratch = tempfile::tempdir().unwrap();
    let scratch = Utf8PathBuf::from_path_buf(scratch.path().to_path_buf()).unwrap();
    let next = AtomicUsize::new(0);
    let differing = Mutex::new(Vec::new());
    let workers = std::thread::available_parallelism().map_or(4, std::num::NonZero::get);
    std::thread::scope(|scope| {
        for _ in 0..workers {
            scope.spawn(|| {
                while let Some(program) = programs.get(next.fetch_add(1, Ordering::Relaxed)) {
                    let index = programs
                        .iter()
                        .position(|each| each == program)
                        .unwrap_or(0);
                    for (mode, flags) in [("debug", &[][..]), ("release", &["--release"][..])] {
                        let first = scratch.join(format!("{index}-{mode}-1"));
                        let second = scratch.join(format!("{index}-{mode}-2"));
                        if build(&compiler, program, flags, &first)
                            != build(&compiler, program, flags, &second)
                        {
                            differing
                                .lock()
                                .unwrap()
                                .push(format!("{program} ({mode})"));
                        }
                    }
                }
            });
        }
    });
    let differing = differing.into_inner().unwrap();
    assert!(
        differing.is_empty(),
        "built differently twice:\n{}",
        differing.join("\n")
    );
}
