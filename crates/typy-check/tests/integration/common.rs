//! Checks programs written to a temporary directory, for the tests of this directory.

use camino::Utf8PathBuf;
use typy_check::TypeChecker;
use typy_source::{Project, ProjectSettings};

/// Returns the code of each diagnostic of the program `files`, whose first file is `app.py`,
/// or nothing when it is accepted.
pub(crate) fn codes(files: &[(&str, &str)]) -> Vec<String> {
    found(files, typy_source::PythonFloor::V3_12)
        .into_iter()
        .map(|(code, _, _)| code)
        .collect()
}

/// Returns the code, the message and the rule of each diagnostic of the one-module program
/// `source`, in the order the checker gives them, or nothing when it is accepted.
pub(crate) fn messages(source: &str) -> Vec<(String, String, String)> {
    found(&[("app.py", source)], typy_source::PythonFloor::V3_12)
}

/// Returns the code, message and rule of each diagnostic of `source` at the Python floor
/// `floor`.
pub(crate) fn messages_at(
    source: &str,
    floor: typy_source::PythonFloor,
) -> Vec<(String, String, String)> {
    found(&[("app.py", source)], floor)
}

fn found(files: &[(&str, &str)], floor: typy_source::PythonFloor) -> Vec<(String, String, String)> {
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
    let mut settings = ProjectSettings::new(root.clone());
    settings.python_floor = floor;
    let project = Project::load(&root.join("app.py"), settings).unwrap();
    let found = match TypeChecker::check(&project) {
        Ok(_) => Vec::new(),
        Err(diagnostics) => diagnostics
            .iter()
            .map(|diagnostic| {
                let rule = diagnostic
                    .rule()
                    .map(|rule| rule.to_string())
                    .unwrap_or_default();
                (
                    diagnostic.code().to_string(),
                    diagnostic.message().to_owned(),
                    rule,
                )
            })
            .collect(),
    };
    let _ = std::fs::remove_dir_all(&root);
    found
}

/// Returns the code of each diagnostic of the one-module program `source`.
pub(crate) fn app(source: &str) -> Vec<String> {
    codes(&[("app.py", source)])
}

/// Returns the typed tree of the accepted one-module program `source`.
pub(crate) fn dump(source: &str) -> String {
    use std::sync::atomic::{AtomicU32, Ordering};
    static NEXT: AtomicU32 = AtomicU32::new(0);
    let base = Utf8PathBuf::from_path_buf(std::env::temp_dir()).unwrap();
    let root = base.join(format!(
        "typy-dump-{}-{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    ));
    std::fs::create_dir_all(&root).unwrap();
    std::fs::write(root.join("app.py"), source).unwrap();
    let project = Project::load(&root.join("app.py"), ProjectSettings::new(root.clone())).unwrap();
    let program = TypeChecker::check(&project)
        .unwrap_or_else(|found| panic!("rejected: {found:?}\n{source}"));
    let text = typy_check::dump(&program, &project);
    let _ = std::fs::remove_dir_all(&root);
    text
}
