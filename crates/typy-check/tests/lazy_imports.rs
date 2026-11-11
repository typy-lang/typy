//! Tests of checking lazy imports through the public API (MOD-06, MOD-07, PEP 810).

use camino::Utf8PathBuf;
use typy_source::{Project, ProjectSettings, PythonFloor};

use typy_check::TypeChecker;

/// A project of `files` in a new directory under the system's temporary directory.
struct Files {
    root: Utf8PathBuf,
}

impl Files {
    fn new(files: &[(&str, &str)]) -> Self {
        use std::sync::atomic::{AtomicU32, Ordering};
        static NEXT: AtomicU32 = AtomicU32::new(0);
        let number = NEXT.fetch_add(1, Ordering::Relaxed);
        let base = Utf8PathBuf::from_path_buf(std::env::temp_dir()).unwrap();
        let root = base.join(format!("typy-lazy-{}-{number}", std::process::id()));
        for (path, text) in files {
            let path = root.join(path);
            std::fs::create_dir_all(path.parent().unwrap()).unwrap();
            std::fs::write(path, text).unwrap();
        }
        Self { root }
    }

    /// Loads the project with `floor`, which decides whether `lazy` is a keyword.
    fn project(&self, floor: PythonFloor) -> Result<Project, Vec<String>> {
        let mut settings = ProjectSettings::new(self.root.clone());
        settings.python_floor = floor;
        Project::load(&self.root.join("app.py"), settings).map_err(|failure| {
            failure
                .diagnostics
                .iter()
                .map(|diagnostic| diagnostic.code().to_string())
                .collect()
        })
    }
}

impl Drop for Files {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.root);
    }
}

/// Returns the typed dump of a program accepted at `floor`.
fn accepted_at(floor: PythonFloor, files: &[(&str, &str)]) -> String {
    let files = Files::new(files);
    let project = files
        .project(floor)
        .unwrap_or_else(|codes| panic!("does not load: {codes:?}"));
    let program = TypeChecker::check(&project)
        .unwrap_or_else(|diagnostics| panic!("rejected: {diagnostics:?}"));
    typy_check::dump(&program, &project)
}

/// Returns the codes of a program rejected at floor 3.15, while loading or checking.
fn rejected(files: &[(&str, &str)]) -> Vec<String> {
    let files = Files::new(files);
    let project = match files.project(PythonFloor::V3_15) {
        Ok(project) => project,
        Err(codes) => return codes,
    };
    TypeChecker::check(&project)
        .unwrap_err()
        .iter()
        .map(|diagnostic| diagnostic.code().to_string())
        .collect()
}

const TOOLS: &str = "\
print('tools runs')
LIMIT = 10


class Point:
    def __init__(self, x: int) -> None:
        self.x = x


def double(n: int) -> int:
    return n * 2
";

const OTHER: &str = "def double(n: int) -> int:\n    return n + n\n";

fn assert_contains(dump: &str, nodes: &[&str]) {
    for node in nodes {
        assert!(dump.contains(node), "{node} missing from\n{dump}");
    }
}

#[test]
fn a_lazy_import_binds_each_name_through_its_own_function() {
    let dump = accepted_at(
        PythonFloor::V3_15,
        &[
            (
                "app.py",
                "\
lazy import tools
lazy from tools import Point, double, LIMIT
if LIMIT > 0:
    lazy import other
for i in range(1):
    pass


class Box:
    def __init__(self) -> None:
        self.p = Point(4)


def read() -> int:
    return tools.LIMIT + double(LIMIT)


print(Box().p.x, read(), other.double(1))
",
            ),
            ("tools.py", TOOLS),
            ("other.py", OTHER),
        ],
    );
    assert_contains(
        &dump,
        &[
            "Import tools binds tools = tools",
            "Import tools binds LIMIT = tools.LIMIT",
            "LazyImport",
            "binds tools",
            "binds LIMIT",
            "binds other",
            "Lazy ",
        ],
    );
}

#[test]
fn lazy_modules_make_the_listed_imports_lazy_at_every_floor() {
    for floor in PythonFloor::ALL {
        let dump = accepted_at(
            floor,
            &[
                (
                    "app.py",
                    "__lazy_modules__ = ['tools']\nimport tools\nimport other\n\
                     print(tools.LIMIT, other.double(1))\n",
                ),
                ("tools.py", TOOLS),
                ("other.py", OTHER),
            ],
        );
        assert_contains(&dump, &["LazyImport", "Import other binds other = other"]);
        assert!(
            !dump.contains("binds other\n"),
            "other is not listed\n{dump}"
        );
    }
}

#[test]
fn a_function_import_of_the_same_name_hides_the_lazy_one() {
    let dump = accepted_at(
        PythonFloor::V3_15,
        &[
            (
                "app.py",
                "\
lazy from tools import double


def twice(n: int) -> int:
    from other import double
    return double(n)


print(twice(2))
",
            ),
            ("tools.py", TOOLS),
            ("other.py", OTHER),
        ],
    );
    let twice = dump
        .split("Function ")
        .find(|function| function.contains("twice("))
        .unwrap();
    assert!(!twice.contains("Lazy "), "{twice}");
}

#[test]
fn lazy_imports_outside_module_level_are_rejected() {
    let cases: &[(&str, &[&str])] = &[
        ("def f() -> None:\n    lazy import tools\n", &["E0103"]),
        // A class body holds only methods and its docstring in TyPy.
        ("class C:\n    lazy import tools\n", &["E0202"]),
        (
            "try:\n    lazy import tools\nexcept ImportError:\n    pass\n",
            &["E0103"],
        ),
        ("lazy from tools import *\n", &["E0103"]),
        ("lazy from __future__ import annotations\n", &["E0103"]),
    ];
    for (source, codes) in cases {
        let found = rejected(&[("app.py", source), ("tools.py", TOOLS)]);
        assert_eq!(found, *codes, "{source}");
    }
}

#[test]
fn lazy_imports_planned_for_a_later_release_give_e0201() {
    let cases: &[&str] = &[
        "lazy import tools\nif tools.LIMIT:\n    lazy import tools\n",
        "lazy import tools\ntools.LIMIT = 3\n",
    ];
    for source in cases {
        let found = rejected(&[("app.py", source), ("tools.py", TOOLS), ("other.py", OTHER)]);
        assert_eq!(found, ["E0201"], "{source}");
    }
}

#[test]
fn a_name_another_module_imports_lazily_gives_e0201() {
    for source in [
        "from middle import double\n",
        "import middle\nprint(middle.double(1))\n",
        "import middle\nprint(middle.tools)\n",
    ] {
        let found = rejected(&[
            ("app.py", source),
            (
                "middle.py",
                "lazy import tools\nlazy from tools import double\n",
            ),
            ("tools.py", TOOLS),
        ]);
        assert_eq!(found, ["E0201"], "{source}");
    }
}

#[test]
fn lazy_is_a_name_before_floor_3_15() {
    let dump = accepted_at(PythonFloor::V3_12, &[("app.py", "lazy = 1\nprint(lazy)\n")]);
    assert!(!dump.contains("LazyImport"), "{dump}");
}
