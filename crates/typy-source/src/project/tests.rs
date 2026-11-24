//! Tests of `project`.

use super::Project;
use crate::settings::ProjectSettings;
use crate::syntax::Stmt;
use camino::Utf8PathBuf;
use typy_diagnostics::{SourceLookup, render_text};

fn project_with(source: &str) -> (tempfile::TempDir, Utf8PathBuf) {
    let directory = tempfile::tempdir().unwrap();
    std::fs::write(directory.path().join("app.py"), source).unwrap();
    let root = Utf8PathBuf::from_path_buf(directory.path().to_path_buf()).unwrap();
    (directory, root)
}

#[test]
fn valid_program_loads_as_the_main_module() {
    let (_directory, root) = project_with("print(1 + 2)\n");
    let project = Project::load(&root.join("app.py"), ProjectSettings::new(root.clone())).unwrap();
    let main = project.main_module();
    assert_eq!(main.name.as_str(), "__main__");
    assert!(matches!(main.syntax.body.as_slice(), [Stmt::Expr { .. }]));
    assert_eq!(project.modules().count(), 1);
    assert_eq!(project.path(main.file), "app.py");
    assert_eq!(project.line_text(main.file, 1), "print(1 + 2)");
    assert_eq!(project.settings().root, root);
    assert!(project.file(main.file).is_some());
}

#[test]
fn syntax_errors_are_returned_with_the_files_to_render_them() {
    let (_directory, root) = project_with("x = (1 +\n");
    let failure = Project::load(&root.join("app.py"), ProjectSettings::new(root)).unwrap_err();
    assert!(failure.diagnostics.has_errors());
    let first = failure.diagnostics.clone().into_sorted(&failure).remove(0);
    let text = render_text(&first, &failure);
    assert!(text.starts_with("error[E0103]: "), "{text}");
    assert!(text.contains("--> app.py:"), "{text}");
}

#[test]
fn missing_main_file_fails_with_e0105() {
    let directory = tempfile::tempdir().unwrap();
    let root = Utf8PathBuf::from_path_buf(directory.path().to_path_buf()).unwrap();
    let failure = Project::load(&root.join("absent.py"), ProjectSettings::new(root)).unwrap_err();
    let diagnostics = failure.diagnostics.clone().into_sorted(&failure);
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].code().to_string(), "E0105");
    assert_eq!(
        failure.path(diagnostics[0].span().unwrap().file()),
        "absent.py"
    );
}

#[test]
fn unknown_file_ids_render_without_panicking() {
    let (_directory, root) = project_with("pass\n");
    let project = Project::load(&root.join("app.py"), ProjectSettings::new(root)).unwrap();
    let unknown = typy_diagnostics::FileId(99);
    assert_eq!(project.path(unknown), "<unknown>");
    assert_eq!(project.line_text(unknown, 1), "");
    assert_eq!(
        project
            .position(unknown, typy_diagnostics::TextOffset(0))
            .line,
        1
    );
}

#[test]
fn a_standard_library_module_typy_gives_in_python_loads_from_its_source() {
    let (_directory, root) = project_with("import bisect\nprint(bisect.bisect([1, 3], 2))\n");
    let project = Project::load(&root.join("app.py"), ProjectSettings::new(root.clone())).unwrap();
    let module = project
        .modules()
        .find(|module| module.name.as_str() == "bisect")
        .expect("the program gets `bisect`");
    assert_eq!(project.path(module.file), "<typy-stdlib>/bisect.py");
    assert!(
        module.syntax.body.len() > 4,
        "the module defines its functions"
    );
    // `bisect` is a module of the program now, though its own imports name standard ones.
    assert!(
        project
            .standard_modules()
            .all(|standard| standard.as_str() != "bisect")
    );
}

#[test]
fn a_module_of_the_program_hides_the_standard_library_module_of_its_name() {
    let (directory, root) = project_with("import bisect\nprint(bisect.mine)\n");
    std::fs::write(directory.path().join("bisect.py"), "mine = 1\n").unwrap();
    let project = Project::load(&root.join("app.py"), ProjectSettings::new(root.clone())).unwrap();
    let module = project
        .modules()
        .find(|module| module.name.as_str() == "bisect")
        .expect("the program's own `bisect`");
    assert_eq!(project.path(module.file), "bisect.py");
}
