//! End-to-end tests of C libraries built with `typy build --lib`: the library is built as a
//! developer builds it, then linked into a C program, or loaded by CPython through `ctypes`,
//! and run (CLI-09, FFI-05, EXC-05, ADR-0044).

use std::process::{Command, Output};

use camino::{Utf8Path, Utf8PathBuf};

/// A library of every C type an export takes and returns, and of every way a call ends.
const TOOLS: &str = "\
__all__ = ['add', 'scale', 'positive', 'describe', 'fail', 'leave', 'depth']

calls = 0


def add(a: int, b: int) -> int:
    global calls
    calls += 1
    return a + b


def scale(value: float, factor: float) -> float:
    return value * factor


def positive(number: int) -> bool:
    return number > 0


def describe(number: int) -> None:
    print('number', number, 'calls', calls, 'module', __name__)


def fail(code: int) -> int:
    raise ValueError('refused ' + str(code))


def leave(status: int) -> None:
    raise SystemExit(status)


def depth(levels: int) -> int:
    if levels == 0:
        return 0
    return depth(levels - 1) + 1


print('tools loaded as', __name__)
if __name__ == '__main__':
    print('never printed')
";

/// A C program that calls each export, and one that ends the way its first argument names.
const HOST: &str = r#"
#include <stdio.h>
#include <string.h>
#include "tools.h"

static void say(const char *text) { printf("%s\n", text); fflush(stdout); }

int main(int argc, char **argv) {
    say("host starts");
    printf("add %lld %lld\n", (long long)add(40, 2), (long long)add(INT64_MAX - 1, 1));
    printf("scale %g positive %d %d\n", scale(1.5, 4.0), positive(3), positive(-3));
    fflush(stdout);
    describe(7);
    printf("depth %lld\n", (long long)depth(5000));
    fflush(stdout);
    if (argc > 1 && strcmp(argv[1], "fail") == 0) fail(5);
    if (argc > 1 && strcmp(argv[1], "overflow") == 0) add(INT64_MAX, 1);
    if (argc > 1 && strcmp(argv[1], "leave") == 0) leave(3);
    say("host ends");
    return 0;
}
"#;

/// The lines a full run of `HOST` writes.
const HOST_OUTPUT: &str = "\
tools loaded as tools
host starts
add 42 9223372036854775807
scale 6 positive 1 0
number 7 calls 2 module tools
depth 5000
";

struct Project {
    _directory: tempfile::TempDir,
    root: Utf8PathBuf,
}

impl Project {
    fn new(files: &[(&str, &str)]) -> Self {
        let directory = tempfile::tempdir().unwrap();
        let root = Utf8PathBuf::from_path_buf(directory.path().to_owned()).unwrap();
        for (name, text) in files {
            std::fs::write(root.join(name), text).unwrap();
        }
        Self {
            _directory: directory,
            root,
        }
    }

    fn typy(&self, arguments: &[&str]) -> Output {
        Command::new(env!("CARGO_BIN_EXE_typy"))
            .args(arguments)
            .current_dir(&self.root)
            .env("TYPY_HOME", self.root.join("home"))
            .output()
            .unwrap()
    }

    fn build(&self, arguments: &[&str]) {
        let built = self.typy(arguments);
        assert!(built.status.success(), "{}", text(&built.stderr));
    }

    /// Runs a program of the C toolchain, which must succeed.
    fn tool(&self, program: &str, arguments: &[&str]) {
        let run = Command::new(program)
            .args(arguments)
            .current_dir(&self.root)
            .output()
            .unwrap();
        assert!(run.status.success(), "{program}: {}", text(&run.stderr));
    }

    fn run(&self, program: &str, arguments: &[&str]) -> Output {
        Command::new(self.root.join(program))
            .args(arguments)
            .current_dir(&self.root)
            .env("LD_LIBRARY_PATH", &self.root)
            .output()
            .unwrap()
    }
}

fn text(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).into_owned()
}

#[test]
fn cli_09_a_static_library_runs_in_a_c_program() {
    let project = Project::new(&[("tools.py", TOOLS), ("host.c", HOST)]);
    project.build(&["build", "--lib", "static", "tools.py"]);
    let header = std::fs::read_to_string(project.root.join("tools.h")).unwrap();
    for declaration in [
        "int64_t add(int64_t a, int64_t b);",
        "double scale(double value, double factor);",
        "bool positive(int64_t number);",
        "void describe(int64_t number);",
    ] {
        assert!(header.contains(declaration), "{header}");
    }
    project.tool("cc", &["-o", "host", "host.c", "-L.", "-ltools", "-lm"]);
    let run = project.run("host", &[]);
    assert_eq!(run.status.code(), Some(0), "{}", text(&run.stderr));
    assert_eq!(text(&run.stdout), format!("{HOST_OUTPUT}host ends\n"));
}

#[test]
fn cli_09_a_static_library_exports_only_its_functions() {
    let project = Project::new(&[("tools.py", TOOLS)]);
    project.build(&[
        "build",
        "--lib",
        "static",
        "tools.py",
        "-o",
        "out.a",
        "--release",
    ]);
    let listed = Command::new("nm")
        .args(["--defined-only", "--extern-only", "out.a"])
        .current_dir(&project.root)
        .output()
        .unwrap();
    let mut names: Vec<String> = text(&listed.stdout)
        .lines()
        .filter_map(|line| line.split_whitespace().nth(2).map(str::to_owned))
        .collect();
    names.sort();
    assert_eq!(
        names,
        [
            "add", "depth", "describe", "fail", "leave", "positive", "scale"
        ]
    );
    assert!(project.root.join("tools.h").exists());
}

#[test]
fn exc_05_an_exception_that_leaves_an_export_ends_the_process() {
    let project = Project::new(&[("tools.py", TOOLS), ("host.c", HOST)]);
    project.build(&["build", "--lib", "static", "tools.py"]);
    project.tool("cc", &["-o", "host", "host.c", "-L.", "-ltools", "-lm"]);
    let failed = project.run("host", &["fail"]);
    assert_eq!(failed.status.code(), None, "the process aborts");
    assert_eq!(text(&failed.stdout), HOST_OUTPUT);
    assert_eq!(
        text(&failed.stderr),
        "Traceback (most recent call last):\n  File \"tools.py\", line 25, in fail\nValueError: \
         refused 5\n"
    );
    let overflow = project.run("host", &["overflow"]);
    assert_eq!(overflow.status.code(), None);
    assert_eq!(
        text(&overflow.stderr),
        "OverflowError: Python int too large to convert to C int64_t\n"
    );
    let left = project.run("host", &["leave"]);
    assert_eq!(left.status.code(), Some(3));
    assert_eq!(text(&left.stdout), HOST_OUTPUT);
    assert_eq!(text(&left.stderr), "");
}

#[test]
fn cli_09_cpython_calls_a_shared_library_through_ctypes() {
    let script = "\
import ctypes
import sys

library = ctypes.CDLL(sys.argv[1])
library.add.argtypes = [ctypes.c_int64, ctypes.c_int64]
library.add.restype = ctypes.c_int64
library.positive.argtypes = [ctypes.c_int64]
library.positive.restype = ctypes.c_bool
library.scale.argtypes = [ctypes.c_double, ctypes.c_double]
library.scale.restype = ctypes.c_double
print('python', library.add(2, 3), library.positive(-1), library.scale(2.0, 0.25))
library.describe(9)
print('python ends')
";
    let project = Project::new(&[("tools.py", TOOLS), ("use_tools.py", script)]);
    project.build(&["build", "--lib", "shared", "tools.py"]);
    let exported = Command::new("nm")
        .args(["-D", "--defined-only", "libtools.so"])
        .current_dir(&project.root)
        .output()
        .unwrap();
    let names: Vec<String> = text(&exported.stdout)
        .lines()
        .filter_map(|line| line.split_whitespace().nth(2).map(str::to_owned))
        .collect();
    assert_eq!(names.len(), 7, "{names:?}");
    let run = Command::new("python3")
        .args(["-I", "-u", "use_tools.py", "./libtools.so"])
        .current_dir(&project.root)
        .output()
        .unwrap();
    assert_eq!(run.status.code(), Some(0), "{}", text(&run.stderr));
    assert_eq!(
        text(&run.stdout),
        "tools loaded as tools\npython 5 False 0.5\nnumber 9 calls 1 module tools\npython ends\n"
    );
}

#[test]
fn cli_09_two_libraries_link_into_one_program_and_serve_another_thread() {
    let other = "__all__ = ['twice']\n\n\ndef twice(value: int) -> int:\n    return value * 2\n";
    let host = r#"
#include <pthread.h>
#include <stdio.h>
#include "tools.h"
#include "other.h"

static void *work(void *argument) {
    (void)argument;
    printf("thread depth %lld\n", (long long)depth(20000));
    fflush(stdout);
    return NULL;
}

int main(void) {
    pthread_t thread;
    pthread_create(&thread, NULL, work, NULL);
    pthread_join(thread, NULL);
    printf("twice %lld add %lld\n", (long long)twice(21), (long long)add(1, 1));
    return 0;
}
"#;
    let project = Project::new(&[("tools.py", TOOLS), ("other.py", other), ("host.c", host)]);
    project.build(&["build", "--lib", "static", "tools.py"]);
    project.build(&["build", "--lib", "static", "other.py"]);
    project.tool(
        "cc",
        &[
            "-o",
            "host",
            "host.c",
            "-L.",
            "-ltools",
            "-lother",
            "-lm",
            "-lpthread",
        ],
    );
    let run = project.run("host", &[]);
    assert_eq!(run.status.code(), Some(0), "{}", text(&run.stderr));
    assert_eq!(
        text(&run.stdout),
        "tools loaded as tools\nthread depth 20000\ntwice 42 add 2\n"
    );
}

#[test]
fn ffi_05_a_name_a_library_cannot_export_is_e0429_in_check_and_build() {
    let source = "def label(n: int) -> str:\n    return str(n)\n\n\n__all__ = ['label']\n";
    let project = Project::new(&[("tools.py", source)]);
    for command in [
        &["check", "--lib", "shared", "tools.py"][..],
        &["build", "--lib", "static", "tools.py"],
    ] {
        let output = project.typy(command);
        assert_eq!(output.status.code(), Some(1), "{command:?}");
        let error = text(&output.stderr);
        assert!(
            error.starts_with("error[E0429]: the result of `label` is `str`"),
            "{error}"
        );
        assert!(error.contains("tools.py:1:1"), "{error}");
    }
    assert!(!project.root.join("libtools.a").exists());
    let plain = project.typy(&["check", "tools.py"]);
    assert_eq!(plain.status.code(), Some(0), "{}", text(&plain.stderr));
}

#[test]
fn cli_09_a_library_file_must_be_named_as_a_module() {
    let project = Project::new(&[("my-tools.py", TOOLS)]);
    let output = project.typy(&["build", "--lib", "static", "my-tools.py"]);
    let error = text(&output.stderr);
    assert_eq!(output.status.code(), Some(1), "{error}");
    assert!(
        error.starts_with("error[E0429]: a library's module is named after its file"),
        "{error}"
    );
    let wrong = project.typy(&["build", "--lib", "dynamic", "my-tools.py"]);
    assert_eq!(wrong.status.code(), Some(2));
}

#[test]
fn ffi_05_the_refused_names_cover_every_c_function_the_runtime_calls() {
    let compiler = Utf8Path::new(env!("CARGO_BIN_EXE_typy"));
    let archive = compiler
        .parent()
        .and_then(Utf8Path::parent)
        .unwrap()
        .join("lib/typy/x86_64-unknown-linux-gnu/libtypy_runtime.a");
    let listed = Command::new("nm")
        .args(["--undefined-only", archive.as_str()])
        .output()
        .unwrap();
    assert!(listed.status.success(), "{}", text(&listed.stderr));
    let defined = Command::new("nm")
        .args(["--defined-only", archive.as_str()])
        .output()
        .unwrap();
    let defined: Vec<String> = text(&defined.stdout)
        .lines()
        .filter_map(|line| line.split_whitespace().nth(2).map(str::to_owned))
        .collect();
    let missing: Vec<String> = text(&listed.stdout)
        .lines()
        .filter_map(|line| line.split_whitespace().last().map(str::to_owned))
        .filter(|name| !name.ends_with(':') && !name.starts_with('_'))
        .filter(|name| !name.starts_with("typy_rt_heap_") && !defined.contains(name))
        .filter(|name| !typy_check::RUNTIME_C_FUNCTIONS.contains(&name.as_str()))
        .collect();
    assert_eq!(missing, Vec::<String>::new());
}
