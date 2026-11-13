//! Feeds any bytes to the source, checking and lowering stages in turn. A stage may reject the
//! input with diagnostics; a panic, or an internal error E9001 such as a TyIR verifier failure,
//! is a defect (NFR-09).
#![no_main]

use camino::Utf8PathBuf;
use libfuzzer_sys::fuzz_target;
use typy_source::{Project, ProjectSettings};

/// The stack the stages run on, as large as the one `typy` gives the compiler, since its passes
/// recurse over trees nested up to `typy_source::MAX_DEPTH` levels.
const COMPILER_STACK: usize = 512 * 1024 * 1024;

fuzz_target!(|bytes: &[u8]| {
    let bytes = bytes.to_vec();
    let stages = std::thread::Builder::new()
        .stack_size(COMPILER_STACK)
        .spawn(move || run_stages(&bytes))
        .expect("the stages' thread starts");
    if let Err(panic) = stages.join() {
        std::panic::resume_unwind(panic);
    }
});

/// Runs the source, checking and lowering stages on `bytes` as a main file.
fn run_stages(bytes: &[u8]) {
    let Ok(directory) = tempfile::tempdir() else {
        return;
    };
    let Ok(root) = Utf8PathBuf::from_path_buf(directory.path().to_owned()) else {
        return;
    };
    let main = root.join("app.py");
    if std::fs::write(&main, bytes).is_err() {
        return;
    }
    let Ok(project) = Project::load(&main, ProjectSettings::new(root)) else {
        return;
    };
    let Ok(program) = typy_check::TypeChecker::check(&project) else {
        return;
    };
    if let Err(diagnostics) = typy_lower::lower(&program) {
        let internal = diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code().to_string() == "E9001");
        assert!(!internal, "lowering failed internally: {diagnostics:?}");
    }
}
