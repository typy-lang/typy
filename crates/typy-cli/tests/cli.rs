//! End-to-end tests: the `typy` executable runs as a subprocess, as a developer runs it.

use std::process::{Command, Output};

use camino::{Utf8Path, Utf8PathBuf};

/// A directory with the program `app.py`, and a private `TYPY_HOME`.
struct Workspace {
    _directory: tempfile::TempDir,
    root: Utf8PathBuf,
}

impl Workspace {
    fn new(source: &str) -> Self {
        let directory = tempfile::tempdir().unwrap();
        let root = Utf8PathBuf::from_path_buf(directory.path().to_owned()).unwrap();
        std::fs::write(root.join("app.py"), source).unwrap();
        Self {
            _directory: directory,
            root,
        }
    }

    fn typy(&self, arguments: &[&str]) -> Output {
        self.typy_at(Utf8Path::new(env!("CARGO_BIN_EXE_typy")), arguments)
    }

    fn typy_at(&self, compiler: &Utf8Path, arguments: &[&str]) -> Output {
        Command::new(compiler)
            .args(arguments)
            .current_dir(&self.root)
            .env("TYPY_HOME", self.root.join("home"))
            .output()
            .unwrap()
    }
}

fn status(output: &Output) -> i32 {
    output.status.code().unwrap_or(-1)
}

fn stdout(output: &Output) -> String {
    String::from_utf8_lossy(&output.stdout).into_owned()
}

fn stderr(output: &Output) -> String {
    String::from_utf8_lossy(&output.stderr).into_owned()
}

const HELLO: &str = "print(\"hello\", 1 + 2)\n";

#[test]
fn wrong_usage_exits_with_status_2() {
    let workspace = Workspace::new(HELLO);
    for arguments in [
        &[][..],
        &["build"],
        &["build", "app.py", "--emit=exe"],
        &["launch"],
    ] {
        assert_eq!(status(&workspace.typy(arguments)), 2, "{arguments:?}");
    }
    let output = workspace.typy(&["build", "app.txt"]);
    assert_eq!(
        (status(&output), stderr(&output).as_str()),
        (2, "error: app.txt is not a `.py` file\n")
    );
    let version = workspace.typy(&["--version"]);
    assert_eq!(
        (status(&version), stdout(&version)),
        (0, format!("typy {}\n", env!("CARGO_PKG_VERSION")))
    );
}

#[test]
fn check_reports_each_diagnostic_with_its_place() {
    let workspace = Workspace::new(HELLO);
    let clean = workspace.typy(&["check", "app.py"]);
    assert_eq!((status(&clean), stderr(&clean).as_str()), (0, ""));
    std::fs::write(workspace.root.join("app.py"), "x: int = 1\nx = \"a\"\n").unwrap();
    let wrong = workspace.typy(&["check", "app.py"]);
    assert_eq!(status(&wrong), 1);
    let text = stderr(&wrong);
    assert!(text.starts_with("error[E0402]: "), "{text}");
    assert!(text.contains(" --> app.py:2:"), "{text}");
    let missing = workspace.typy(&["check", "absent.py"]);
    assert_eq!(status(&missing), 1);
    assert!(
        stderr(&missing).starts_with("error[E0105]"),
        "{}",
        stderr(&missing)
    );
}

#[test]
fn build_writes_an_executable_beside_the_main_file_or_at_the_output() {
    let workspace = Workspace::new(HELLO);
    for (arguments, path) in [
        (&["build", "app.py"][..], "app"),
        (
            &["build", "app.py", "-o", "small", "--release-small"],
            "small",
        ),
        (
            &["build", "app.py", "--release", "--output", "fast"],
            "fast",
        ),
    ] {
        let output = workspace.typy(arguments);
        assert_eq!(
            (status(&output), stderr(&output).as_str()),
            (0, ""),
            "{arguments:?}"
        );
        let run = Command::new(workspace.root.join(path)).output().unwrap();
        assert_eq!(stdout(&run), "hello 3\n");
    }
}

#[test]
fn the_same_build_gives_the_same_bytes() {
    let workspace = Workspace::new(HELLO);
    for output in ["first", "second"] {
        assert_eq!(
            status(&workspace.typy(&["build", "app.py", "-o", output])),
            0
        );
    }
    let first = std::fs::read(workspace.root.join("first")).unwrap();
    assert_eq!(first, std::fs::read(workspace.root.join("second")).unwrap());
}

#[test]
fn emit_writes_one_stage_to_standard_output_or_a_file() {
    let workspace = Workspace::new(HELLO);
    let tyir = workspace.typy(&["build", "app.py", "--emit=tyir"]);
    assert_eq!(status(&tyir), 0);
    assert!(
        stdout(&tyir).contains("fn __typy_main() -> unit raises"),
        "{}",
        stdout(&tyir)
    );
    let object = workspace.typy(&["build", "app.py", "--emit=obj", "-o", "app.o"]);
    assert_eq!((status(&object), stdout(&object).as_str()), (0, ""));
    assert!(
        std::fs::read(workspace.root.join("app.o"))
            .unwrap()
            .starts_with(b"\x7fELF")
    );
    assert!(
        !workspace.root.join("app").exists(),
        "no executable is written"
    );
    let unwritable = workspace.typy(&["build", "app.py", "--emit=ast", "-o", "absent/out.txt"]);
    assert_eq!(status(&unwritable), 1);
    assert!(stderr(&unwritable).starts_with("error[E0106]: cannot write absent/out.txt"));
}

#[test]
fn run_builds_once_and_passes_on_the_program_status() {
    let workspace = Workspace::new(HELLO);
    let first = workspace.typy(&["run", "app.py", "--", "an", "argument"]);
    assert_eq!((status(&first), stdout(&first).as_str()), (0, "hello 3\n"));
    let builds = || {
        std::fs::read_dir(workspace.root.join("home/cache"))
            .unwrap()
            .count()
    };
    assert_eq!(builds(), 1);
    assert_eq!(stdout(&workspace.typy(&["run", "app.py"])), "hello 3\n");
    assert_eq!(builds(), 1, "the second run reuses the build");
    std::fs::write(workspace.root.join("app.py"), "print(1 // 0)\n").unwrap();
    let failing = workspace.typy(&["run", "app.py"]);
    assert_eq!(status(&failing), 1);
    assert!(
        stderr(&failing).ends_with("ZeroDivisionError: division by zero\n"),
        "{}",
        stderr(&failing)
    );
    assert_eq!(builds(), 2);
}

#[test]
fn run_reports_a_program_that_does_not_build() {
    let workspace = Workspace::new("print(undefined)\n");
    let output = workspace.typy(&["run", "app.py"]);
    assert_eq!(status(&output), 1);
    assert!(
        stderr(&output).starts_with("error[E0205]"),
        "{}",
        stderr(&output)
    );
    let missing = workspace.typy(&["run", "absent.py"]);
    assert!(
        stderr(&missing).starts_with("error[E0105]"),
        "{}",
        stderr(&missing)
    );
    assert_eq!(status(&workspace.typy(&["run", "app.txt"])), 2);
}

#[test]
fn a_compiler_without_its_runtime_archive_says_so() {
    let workspace = Workspace::new(HELLO);
    let compiler = workspace.root.join("bin/typy");
    std::fs::create_dir_all(compiler.parent().unwrap()).unwrap();
    std::fs::copy(env!("CARGO_BIN_EXE_typy"), &compiler).unwrap();
    let output = workspace.typy_at(&compiler, &["build", "app.py"]);
    assert_eq!(status(&output), 1);
    assert!(
        stderr(&output).starts_with("error[E0703]: cannot read the runtime archive"),
        "{}",
        stderr(&output)
    );
    let check = workspace.typy_at(&compiler, &["check", "app.py"]);
    assert_eq!(status(&check), 0, "checking needs no archive");
}
