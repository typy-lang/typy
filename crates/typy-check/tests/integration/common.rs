//! Checks programs written to a temporary directory, for the tests of this directory.

use camino::Utf8PathBuf;
use typy_check::TypeChecker;
use typy_source::{Project, ProjectSettings};

/// Returns the code of each diagnostic of the program `files`, whose first file is `app.py`,
/// or nothing when it is accepted.
pub(crate) fn codes(files: &[(&str, &str)]) -> Vec<String> {
    use std::sync::atomic::{AtomicU32, Ordering};
    static NEXT: AtomicU32 = AtomicU32::new(0);
    let base = Utf8PathBuf::from_path_buf(std::env::temp_dir()).unwrap();
    let root = base.join(format!(
        "typy-check-{}-{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    ));
    std::fs::create_dir_all(&root).unwrap();
    for (path, text) in files {
        std::fs::write(root.join(path), text).unwrap();
    }
    let project = Project::load(&root.join("app.py"), ProjectSettings::new(root.clone())).unwrap();
    let found = match TypeChecker::check(&project) {
        Ok(_) => Vec::new(),
        Err(diagnostics) => diagnostics
            .iter()
            .map(|diagnostic| diagnostic.code().to_string())
            .collect(),
    };
    let _ = std::fs::remove_dir_all(&root);
    found
}

/// Returns the code of each diagnostic of the one-module program `source`.
pub(crate) fn app(source: &str) -> Vec<String> {
    codes(&[("app.py", source)])
}
