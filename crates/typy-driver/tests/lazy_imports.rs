//! Tests of programs whose `__lazy_modules__` makes imports lazy, built and run (MOD-06 to
//! MOD-08). Each expected output is what CPython 3.15 prints for the same files; CPython
//! 3.12 to 3.14 ignore `__lazy_modules__`, so these programs are not conformance tests.

use camino::Utf8PathBuf;
use typy_build::BuildRequest;
use typy_driver::{Outcome, Pipeline};

/// Builds and runs the program of `files`, whose main file is `main.py`, and returns its
/// standard output and standard error.
fn run(files: &[(&str, &str)]) -> (String, String) {
    let directory = tempfile::tempdir().unwrap();
    let root = Utf8PathBuf::from_path_buf(directory.path().to_owned()).unwrap();
    for (path, text) in files {
        std::fs::write(root.join(path), text).unwrap();
    }
    let target = option_env!("CARGO_TARGET_DIR").map_or_else(
        || Utf8PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../target"),
        Utf8PathBuf::from,
    );
    let request = BuildRequest::new(root.join("main.py")).unwrap();
    let outcome = Pipeline::new(target.join("debug/typy")).run(&request);
    let Ok(Outcome::Executable(artifact)) = outcome else {
        panic!("no executable: {outcome:?}");
    };
    let output = std::process::Command::new(&artifact.path).output().unwrap();
    (
        String::from_utf8(output.stdout).unwrap(),
        String::from_utf8(output.stderr).unwrap(),
    )
}

const HEAVY: &str = "print('heavy runs')\nVALUE = 42\n";

#[test]
fn a_lazy_module_runs_at_the_first_use_of_its_name() {
    let (output, _) = run(&[
        (
            "main.py",
            "__lazy_modules__ = ['heavy']\nimport heavy\nprint('before use')\n\
             print(heavy.VALUE)\nprint(heavy.VALUE, type(heavy).__name__)\n",
        ),
        ("heavy.py", HEAVY),
    ]);
    assert_eq!(output, "before use\nheavy runs\n42\n42 module\n");
}

#[test]
fn a_lazy_module_that_is_never_used_never_runs() {
    let (output, _) = run(&[
        (
            "main.py",
            "__lazy_modules__ = ['heavy']\nimport heavy\nprint('done')\n",
        ),
        ("heavy.py", HEAVY),
    ]);
    assert_eq!(output, "done\n");
}

#[test]
fn an_error_while_a_lazy_module_runs_reaches_the_use_and_the_next_use_retries() {
    let (output, _) = run(&[
        (
            "main.py",
            "\
__lazy_modules__ = ['flaky']
from flaky import OK
for attempt in range(2):
    try:
        print(OK)
    except ValueError as error:
        print('error', error)
",
        ),
        (
            "flaky.py",
            "import tries\ntries.count += 1\nif tries.count < 2:\n    \
             raise ValueError('first try')\nOK = 'ok'\n",
        ),
        ("tries.py", "count = 0\n"),
    ]);
    assert_eq!(output, "error first try\nok\n");
}

#[test]
fn calls_classes_and_functions_run_the_lazy_import_first() {
    let (output, _) = run(&[
        (
            "main.py",
            "\
__lazy_modules__ = ['heavy', 'tools']
import heavy
from tools import Point, double, LIMIT


class Box:
    def __init__(self) -> None:
        self.p = Point(4)


def read() -> int:
    return heavy.VALUE + LIMIT


LIMIT = 3
print('start', LIMIT)
print(double(2))
print(Box().p.x)
print(read())
",
        ),
        ("heavy.py", HEAVY),
        (
            "tools.py",
            "\
print('tools runs')


class Point:
    def __init__(self, x: int) -> None:
        self.x = x


def double(n: int) -> int:
    return n * 2


LIMIT = 10
",
        ),
    ]);
    assert_eq!(output, "start 3\ntools runs\n4\n4\nheavy runs\n45\n");
}

#[test]
fn a_function_reading_a_lazy_name_rebound_by_the_import_reads_the_value() {
    let (output, _) = run(&[
        (
            "main.py",
            "\
__lazy_modules__ = ['heavy']
from heavy import VALUE


def show() -> None:
    print(VALUE)


show()
show()
",
        ),
        ("heavy.py", HEAVY),
    ]);
    assert_eq!(output, "heavy runs\n42\n42\n");
}
