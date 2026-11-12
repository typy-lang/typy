//! Tests of loading the modules a program imports, through the public API (MOD-01, MOD-02,
//! MOD-05).

use camino::Utf8PathBuf;
use typy_diagnostics::{Diagnostic, SourceLookup};
use typy_source::{LoadFailure, ModuleName, Project, ProjectSettings};

/// Writes the files of a project and returns its directory, its root and the main file.
fn project(files: &[(&str, &str)]) -> (tempfile::TempDir, Utf8PathBuf) {
    let directory = tempfile::tempdir().unwrap();
    for (path, text) in files {
        let path = directory.path().join(path);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, text).unwrap();
    }
    let root = Utf8PathBuf::from_path_buf(directory.path().to_path_buf()).unwrap();
    (directory, root)
}

fn load(files: &[(&str, &str)]) -> Result<Project, LoadFailure> {
    let (_directory, root) = project(files);
    Project::load(&root.join("app.py"), ProjectSettings::new(root))
}

fn names(project: &Project) -> Vec<&str> {
    project.order().iter().map(ModuleName::as_str).collect()
}

/// Returns each diagnostic as its code, its message and the line of its first span.
fn errors(failure: &LoadFailure) -> Vec<(String, String, String)> {
    failure
        .diagnostics
        .clone()
        .into_sorted(failure)
        .iter()
        .map(|diagnostic: &Diagnostic| {
            let span = diagnostic.span().unwrap();
            (
                diagnostic.code().to_string(),
                diagnostic.message().to_owned(),
                format!(
                    "{}:{}",
                    failure.path(span.file()),
                    failure.position(span.file(), span.start()).line
                ),
            )
        })
        .collect()
}

#[test]
fn imported_modules_load_after_the_modules_they_import() {
    let project = load(&[
        ("app.py", "import shapes\nfrom tools.text import shout\n"),
        ("shapes.py", "import tools\n"),
        ("tools/__init__.py", ""),
        ("tools/text.py", "def shout() -> None:\n    pass\n"),
    ])
    .unwrap();
    assert_eq!(
        names(&project),
        ["tools", "shapes", "tools.text", "__main__"]
    );
    let tools = project.module(&ModuleName::new("tools").unwrap()).unwrap();
    assert!(tools.is_package);
    assert_eq!(project.path(tools.file), "tools/__init__.py");
    let imports: Vec<&str> = project
        .imports(&ModuleName::main())
        .iter()
        .map(|import| import.module.as_str())
        .collect();
    assert_eq!(
        imports,
        ["shapes", "tools", "tools.text", "tools.text.shout"]
    );
}

#[test]
fn a_package_comes_before_a_module_of_the_same_name() {
    let project = load(&[
        ("app.py", "import shapes\n"),
        ("shapes.py", ""),
        ("shapes/__init__.py", ""),
    ])
    .unwrap();
    let shapes = project.module(&ModuleName::new("shapes").unwrap()).unwrap();
    assert_eq!(project.path(shapes.file), "shapes/__init__.py");
}

#[test]
fn relative_imports_start_from_the_importing_modules_package() {
    let project = load(&[
        ("app.py", "import pkg.inner.leaf\n"),
        ("pkg/__init__.py", "from . import helper\n"),
        ("pkg/helper.py", ""),
        ("pkg/inner/__init__.py", "from .. import helper\n"),
        (
            "pkg/inner/leaf.py",
            "from .sibling import value\nfrom ..helper import name\n",
        ),
        ("pkg/inner/sibling.py", "value = 1\n"),
    ])
    .unwrap();
    assert_eq!(
        names(&project),
        [
            "pkg.helper",
            "pkg",
            "pkg.inner",
            "pkg.inner.sibling",
            "pkg.inner.leaf",
            "__main__"
        ]
    );
}

#[test]
fn the_main_file_imported_by_name_is_a_second_module_of_the_same_file() {
    let project = load(&[("app.py", "def f() -> None:\n    import app\n")]).unwrap();
    let app = project.module(&ModuleName::new("app").unwrap()).unwrap();
    assert_eq!(app.file, project.main_module().file);
    // An import inside a function orders its module first.
    assert_eq!(names(&project), ["app", "__main__"]);
}

#[test]
fn typing_collections_abc_and_types_need_no_file() {
    let project = load(&[(
        "app.py",
        "from collections.abc import Iterator\nimport typing\nfrom types import TracebackType\n",
    )])
    .unwrap();
    for name in ["collections", "collections.abc", "typing", "types"] {
        assert!(
            project.imports_standard(&ModuleName::new(name).unwrap()),
            "{name}"
        );
    }
    assert_eq!(names(&project), ["__main__"]);
}

#[test]
fn standard_library_modules_are_left_to_the_checker() {
    let project = load(&[(
        "app.py",
        "import math
from os.path import join
",
    )])
    .unwrap();
    for name in ["math", "os", "os.path"] {
        assert!(
            project.imports_standard(&ModuleName::new(name).unwrap()),
            "{name}"
        );
    }
}

#[test]
fn a_user_module_hides_a_standard_module_of_the_same_name() {
    let project = load(&[("app.py", "import typing\n"), ("typing.py", "")]).unwrap();
    assert!(
        project
            .module(&ModuleName::new("typing").unwrap())
            .is_some()
    );
    assert!(!project.imports_standard(&ModuleName::new("typing").unwrap()));
}

#[test]
fn modules_that_cannot_be_found_or_read_are_reported_at_their_import() {
    let failure = load(&[
        (
            "app.py",
            "import missing\nimport plain.part\nfrom . import nothing\n",
        ),
        ("plain.py", ""),
        ("broken.py", "x = (\n"),
    ])
    .unwrap_err();
    assert_eq!(
        errors(&failure),
        [
            (
                "E0206".to_owned(),
                "no module named `missing`".to_owned(),
                "app.py:1".to_owned()
            ),
            (
                "E0206".to_owned(),
                "no module named `plain.part`; `plain` is not a package".to_owned(),
                "app.py:2".to_owned()
            ),
            (
                "E0206".to_owned(),
                "attempted relative import with no known parent package".to_owned(),
                "app.py:3".to_owned()
            ),
        ]
    );
    let failure = load(&[("app.py", "import broken\n"), ("broken.py", "x = (\n")]).unwrap_err();
    let found = errors(&failure);
    assert_eq!(found[0].0, "E0103");
    assert!(found[0].2.starts_with("broken.py:"), "{found:?}");
}

#[test]
fn a_relative_import_beyond_the_top_package_is_reported() {
    let failure = load(&[
        ("app.py", "import pkg\n"),
        ("pkg/__init__.py", "from ... import x\n"),
    ])
    .unwrap_err();
    assert_eq!(
        errors(&failure)[0].1,
        "attempted relative import beyond top-level package"
    );
}

#[test]
fn a_cycle_of_module_level_imports_is_reported_with_each_import() {
    let failure = load(&[
        ("app.py", "import first\n"),
        ("first.py", "import second\n"),
        ("second.py", "from first import value\n"),
    ])
    .unwrap_err();
    let found = errors(&failure);
    assert_eq!(found.len(), 1, "{found:?}");
    assert_eq!(found[0].0, "E0422");
    assert_eq!(
        found[0].1,
        "circular import: `first` imports `second`, which imports `first`"
    );
    assert_eq!(found[0].2, "first.py:1");
}

#[test]
fn an_import_inside_a_function_makes_no_cycle() {
    let project = load(&[
        ("app.py", "import first\n"),
        ("first.py", "import second\n"),
        ("second.py", "def later() -> None:\n    import first\n"),
        ("pkg/__init__.py", "from . import part\n"),
    ])
    .unwrap();
    assert_eq!(names(&project), ["second", "first", "__main__"]);
}

#[test]
fn a_missing_submodule_of_a_package_and_a_missing_main_file_are_reported() {
    let failure = load(&[
        ("app.py", "import tools.missing\n"),
        ("tools/__init__.py", ""),
    ])
    .unwrap_err();
    assert_eq!(errors(&failure)[0].1, "no module named `tools.missing`");
    let failure = load(&[("other.py", "")]).unwrap_err();
    assert_eq!(errors(&failure)[0].0, "E0105");
}

#[test]
fn imports_in_every_kind_of_block_are_loaded() {
    let block_imports = concat!(
        "if True:\n    import in_if\nelse:\n    import in_else\n",
        "for _ in []:\n    import in_for\nelse:\n    import in_for_else\n",
        "while False:\n    import in_while\n",
        "class Holder:\n    import in_class\n",
        "try:\n    import in_try\nexcept ValueError:\n    import in_except\nelse:\n",
        "    import in_try_else\nfinally:\n    import in_finally\n",
        "def later() -> None:\n    import in_function\n",
    );
    let modules = [
        "in_if",
        "in_else",
        "in_for",
        "in_for_else",
        "in_while",
        "in_class",
        "in_try",
        "in_except",
        "in_try_else",
        "in_finally",
        "in_function",
    ];
    let mut files: Vec<(String, String)> = vec![("app.py".to_owned(), block_imports.to_owned())];
    files.extend(
        modules
            .iter()
            .map(|name| (format!("{name}.py"), String::new())),
    );
    let borrowed: Vec<(&str, &str)> = files
        .iter()
        .map(|(path, text)| (path.as_str(), text.as_str()))
        .collect();
    let project = load(&borrowed).unwrap();
    let loaded = names(&project);
    for name in modules {
        assert!(loaded.contains(&name), "{name} in {loaded:?}");
    }
    let in_order: Vec<&str> = project
        .modules_in_order()
        .map(|module| module.name.as_str())
        .collect();
    assert_eq!(in_order, loaded);
}

#[test]
fn a_lazy_import_an_import_in_a_function_and_one_of_the_module_itself_make_no_cycle() {
    for files in [
        [
            ("app.py", "import a\n"),
            ("a.py", "lazy import b\n"),
            ("b.py", "import a\n"),
        ],
        [
            ("app.py", "import a\n"),
            ("a.py", "__lazy_modules__ = [\"b\"]\nimport b\n"),
            ("b.py", "import a\n"),
        ],
        [
            ("app.py", "import a\n"),
            ("a.py", "import a\n"),
            ("b.py", ""),
        ],
        [
            ("app.py", "import a\n"),
            ("a.py", "def use() -> None:\n    import b\n"),
            ("b.py", "import a\n"),
        ],
    ] {
        let (_directory, root) = project(&files);
        let mut settings = ProjectSettings::new(root.clone());
        settings.python_floor = typy_source::PythonFloor::V3_15;
        let loaded = Project::load(&root.join("app.py"), settings)
            .unwrap_or_else(|failure| panic!("{files:?}: {:?}", errors(&failure)));
        assert_eq!(names(&loaded).last(), Some(&"__main__"), "{files:?}");
    }
}

#[test]
fn the_standard_modules_a_program_imports_are_listed() {
    let project = load(&[
        ("app.py", "import math\nimport sys\nimport helper\n"),
        ("helper.py", ""),
    ])
    .unwrap();
    let standard: Vec<&str> = project.standard_modules().map(ModuleName::as_str).collect();
    assert_eq!(standard, ["math", "sys"]);
}

#[test]
fn a_load_failure_gives_the_text_of_each_line() {
    let failure = load(&[("app.py", "x = 1\nimport missing\n")]).unwrap_err();
    let span = failure.diagnostics.iter().next().unwrap().span().unwrap();
    assert_eq!(failure.line_text(span.file(), 2), "import missing");
    assert_eq!(failure.line_text(span.file(), 1), "x = 1");
}

#[test]
fn the_largest_source_file_is_16_mib() {
    assert_eq!(typy_source::MAX_SOURCE_BYTES, 16_777_216);
}
