//! Tests of checking `sys`, `input` and `print(file=...)` through the public API (LIB-04,
//! LIB-13, MOD-09, ADR-0018): the names TyPy provides, the ones it refuses and why.

use typy_check::TypeChecker;
use typy_source::{Project, ProjectSettings, PythonFloor};

/// Returns each diagnostic of `source`, checked at `floor`, as its code and message.
fn diagnostics_at(floor: PythonFloor, source: &str) -> Vec<(String, String)> {
    use std::sync::atomic::{AtomicU32, Ordering};
    static NEXT: AtomicU32 = AtomicU32::new(0);
    let base = camino::Utf8PathBuf::from_path_buf(std::env::temp_dir()).unwrap();
    let directory = base.join(format!(
        "typy-sys-{}-{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    ));
    std::fs::create_dir_all(&directory).unwrap();
    std::fs::write(directory.join("app.py"), source).unwrap();
    let mut settings = ProjectSettings::new(directory.clone());
    settings.python_floor = floor;
    let project = Project::load(&directory.join("app.py"), settings).unwrap();
    let found = match TypeChecker::check(&project) {
        Ok(_) => Vec::new(),
        Err(diagnostics) => diagnostics
            .iter()
            .map(|diagnostic| {
                (
                    diagnostic.code().to_string(),
                    diagnostic.message().to_owned(),
                )
            })
            .collect(),
    };
    let _ = std::fs::remove_dir_all(&directory);
    found
}

fn diagnostics(source: &str) -> Vec<(String, String)> {
    diagnostics_at(PythonFloor::V3_12, source)
}

#[test]
fn the_names_typy_provides_are_accepted() {
    for source in [
        "import sys\nprint(sys.argv, sys.stdout, sys.stderr)\nsys.exit()\n",
        "import sys as s\nprint(s.stdout.write(\"x\"), s.stderr.flush())\ns.exit(None)\n",
        "from sys import argv, exit\nargv.append(\"x\")\nexit(len(argv))\n",
        "from sys import stderr as error\nprint(1, 2, sep=\"\", file=error, flush=True)\n",
        "import sys\nsys.exit(\"a message\")\n",
        "print(input(), input(\"prompt\"), input(1))\n",
        "print(1, file=None)\n",
        "def main() -> None:\n    import sys\n    sys.exit(2)\n\n\nmain()\n",
    ] {
        assert_eq!(diagnostics(source), [], "{source}");
    }
}

#[test]
fn each_switch_of_lazy_imports_is_refused() {
    for name in [
        "get_lazy_imports",
        "get_lazy_imports_filter",
        "lazy_modules",
        "set_lazy_imports",
        "set_lazy_imports_filter",
    ] {
        for source in [
            format!("import sys\nprint(sys.{name})\n"),
            format!("from sys import {name}\n"),
        ] {
            let found = diagnostics_at(PythonFloor::V3_15, &source);
            assert_eq!(found.len(), 1, "{source}");
            assert_eq!(found[0].0, "E0202", "{source}");
            assert!(found[0].1.contains("(MOD-09)"), "{source}: {}", found[0].1);
        }
    }
}

#[test]
fn names_of_sys_typy_does_not_have_give_their_codes() {
    for (source, code, message) in [
        (
            "import sys\nprint(sys.version_info)\n",
            "E0201",
            "`sys.version_info` is planned for TyPy 0.4 (LIB-13)",
        ),
        (
            "from sys import stdin\n",
            "E0201",
            "`sys.stdin` is planned for TyPy 0.4 (LIB-13)",
        ),
        (
            "import sys\nprint(sys.getrefcount(1))\n",
            "E0202",
            "TyPy does not provide `sys.getrefcount`",
        ),
        (
            "from sys import modules\n",
            "E0202",
            "TyPy does not provide `sys.modules`",
        ),
        (
            "from sys import arguments\n",
            "E0425",
            "module `sys` has no name `arguments`",
        ),
        (
            "import sys\nf = sys.exit\n",
            "E0201",
            "using `sys.exit` as a value is planned for TyPy 1.0 (FUN-05)",
        ),
    ] {
        assert_eq!(
            diagnostics(source),
            [(code.to_owned(), message.to_owned())],
            "{source}"
        );
    }
}

#[test]
fn calls_sys_exit_and_input_refuse_give_their_messages() {
    for (source, message) in [
        (
            "import sys\nsys.exit(1, 2)\n",
            "`sys.exit` takes at most 1 argument, and 2 are given",
        ),
        (
            "import sys\nsys.exit(status=1)\n",
            "`sys.exit` takes no keyword arguments",
        ),
        (
            "print(input(\"a\", \"b\"))\n",
            "`input` takes 0 to 1 arguments in TyPy 0.1, and 2 are given",
        ),
    ] {
        assert_eq!(
            diagnostics(source),
            [("E0404".to_owned(), message.to_owned())],
            "{source}"
        );
    }
}

#[test]
fn a_sys_exit_of_an_erroneous_value_reports_only_that_error() {
    let found = diagnostics("import sys\nsys.exit(missing)\n");
    assert_eq!(found.len(), 1, "{found:?}");
    assert_eq!(found[0].0, "E0205");
}

#[test]
fn print_takes_only_a_text_file_and_a_bool_flush() {
    for source in [
        "print(1, file=1)\n",
        "print(1, flush=1)\n",
        "import sys\nprint(1, file=sys.argv)\n",
    ] {
        let found = diagnostics(source);
        assert_eq!(found.len(), 1, "{source}: {found:?}");
        assert_eq!(found[0].0, "E0404", "{source}");
    }
}

#[test]
fn a_text_file_has_write_and_flush_with_their_types() {
    for source in [
        "import sys\nsys.stdout.write(1)\n",
        "import sys\nsys.stdout.flush(1)\n",
        "import sys\nn: str = sys.stdout.write(\"x\")\n",
    ] {
        assert_eq!(diagnostics(source).len(), 1, "{source}");
    }
}

#[test]
fn the_typed_dump_shows_each_sys_expression_and_its_operands() {
    let base = camino::Utf8PathBuf::from_path_buf(std::env::temp_dir()).unwrap();
    let directory = base.join(format!("typy-sys-dump-{}", std::process::id()));
    std::fs::create_dir_all(&directory).unwrap();
    std::fs::write(
        directory.join("app.py"),
        "import sys\nprint(sys.argv, file=sys.stderr, flush=True)\nprint(input(\"p\"))\nsys.exit(len(sys.argv))\n",
    )
    .unwrap();
    let project = Project::load(
        &directory.join("app.py"),
        ProjectSettings::new(directory.clone()),
    )
    .unwrap();
    let program = TypeChecker::check(&project).unwrap();
    let dump = typy_check::dump(&program, &project);
    let _ = std::fs::remove_dir_all(&directory);
    for expected in [
        "SysValue Argv : list[str] @2:7",
        "SysValue Stderr : TextIOWrapper @2:22",
        "Constant Bool(true) : bool @2:40",
        "Input : str @3:7",
        "Constant Str(\"p\") : str @3:13",
        "SysExit : None @4:1",
        "Builtin Len : int @4:10",
    ] {
        assert!(dump.contains(expected), "{expected} in\n{dump}");
    }
}

#[test]
fn assigning_a_name_of_a_built_in_module_is_refused() {
    for (source, code, message) in [
        (
            "import sys\nsys.argv = [\"x\"]\n",
            "E0402",
            "`sys.argv` is not a module variable",
        ),
        (
            "import math\nmath.pi = 3.0\n",
            "E0402",
            "`math.pi` is not a module variable",
        ),
        (
            "import sys\nsys.version_info = 1\n",
            "E0201",
            "`sys.version_info` is planned for TyPy 0.4 (LIB-13)",
        ),
    ] {
        let found = diagnostics(source);
        assert_eq!(found.len(), 1, "{source}: {found:?}");
        assert_eq!((found[0].0.as_str(), found[0].1.as_str()), (code, message));
    }
}

#[test]
fn sys_argv_changes_in_place() {
    let source =
        "import sys\nsys.argv.clear()\nsys.argv.extend([\"a\", \"b\"])\nsys.argv[0] = \"c\"\n";
    assert_eq!(diagnostics(source), []);
}
