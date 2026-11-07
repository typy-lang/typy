//! Feeds any bytes to the source, checking and lowering stages in turn. A stage may reject the
//! input with diagnostics; a panic, or an internal error E9001 such as a TyIR verifier failure,
//! is a defect (NFR-09).
#![no_main]

use camino::Utf8PathBuf;
use libfuzzer_sys::fuzz_target;
use typy_source::{Project, ProjectSettings};

fuzz_target!(|bytes: &[u8]| {
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
});
