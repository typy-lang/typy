//! End-to-end tests of C libraries built with `typy build --lib`: the library is built as a
//! developer builds it, then linked into a C program, or loaded by CPython through `ctypes`,
//! and run, with the names and tools of the host's system (CLI-09, CLI-13, CLI-16, FFI-05,
//! FFI-08, EXC-05, ADR-0044, ADR-0045, ADR-0046, ADR-0056).

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

/// The variable the dynamic loader searches for a library a program names without a path.
const LIBRARY_PATH: &str = if cfg!(target_os = "macos") {
    "DYLD_LIBRARY_PATH"
} else {
    "LD_LIBRARY_PATH"
};

/// Returns the name a program loads the shared library `name` of release `major` by:
/// `libNAME.so.X` on Linux and `libNAME.X.dylib` on macOS, or without a release `libNAME.so`
/// and `libNAME.dylib`.
fn loaded_name(name: &str, major: Option<&str>) -> String {
    match (cfg!(target_os = "macos"), major) {
        (true, Some(major)) => format!("lib{name}.{major}.dylib"),
        (true, None) => format!("lib{name}.dylib"),
        (false, Some(major)) => format!("lib{name}.so.{major}"),
        (false, None) => format!("lib{name}.so"),
    }
}

/// Returns a symbol by the name C gives it, without the underscore Mach-O adds.
fn c_name(symbol: &str) -> &str {
    if cfg!(target_os = "macos") {
        symbol.strip_prefix('_').unwrap_or(symbol)
    } else {
        symbol
    }
}

/// Returns the `llvm-nm` of the Rust toolchain, which reads both object formats and the LLVM
/// bitcode Rust's own objects carry, as the system's `nm` on macOS does not.
fn llvm_nm() -> Utf8PathBuf {
    let rustc = |argument: &str| {
        let output = Command::new("rustc").arg(argument).output().unwrap();
        text(&output.stdout)
    };
    let host = rustc("-vV")
        .lines()
        .find_map(|line| line.strip_prefix("host: ").map(str::to_owned))
        .unwrap();
    Utf8PathBuf::from(rustc("--print=sysroot").trim())
        .join("lib/rustlib")
        .join(host)
        .join("bin/llvm-nm")
}

/// Returns the C name of each symbol `llvm-nm` lists in `file` with `arguments`.
fn symbols(directory: &Utf8Path, arguments: &[&str], file: &str) -> Vec<String> {
    let listed = Command::new(llvm_nm())
        .args(arguments)
        .arg(file)
        .current_dir(directory)
        .output()
        .unwrap();
    assert!(listed.status.success(), "{}", text(&listed.stderr));
    text(&listed.stdout)
        .lines()
        .filter_map(|line| line.split_whitespace().nth(2))
        .map(|name| c_name(name).to_owned())
        .collect()
}

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
            .env(LIBRARY_PATH, &self.root)
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
    let mut names = symbols(&project.root, &["--defined-only", "--extern-only"], "out.a");
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
    let library = loaded_name("tools", None);
    // A Linux library exports through its dynamic symbols; a macOS one keeps the rest local.
    let listing: &[&str] = if cfg!(target_os = "macos") {
        &["--defined-only", "--extern-only"]
    } else {
        &["-D", "--defined-only"]
    };
    let names = symbols(&project.root, listing, &library);
    assert_eq!(names.len(), 7, "{names:?}");
    let run = Command::new("python3")
        .args(["-I", "-u", "use_tools.py", &format!("./{library}")])
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
    /* As large a stack as a main thread's, so the depth fits on every system: macOS gives a
       new thread 512 KiB, Linux the limit of the main thread's. */
    pthread_attr_t attributes;
    pthread_attr_init(&attributes);
    pthread_attr_setstacksize(&attributes, 8 * 1024 * 1024);
    pthread_t thread;
    pthread_create(&thread, &attributes, work, NULL);
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
        .join("lib/typy")
        .join(typy_codegen::Target::host().triple.as_str())
        .join("libtypy_runtime.a");
    let listed = Command::new(llvm_nm())
        .args(["--undefined-only", archive.as_str()])
        .output()
        .unwrap();
    assert!(listed.status.success(), "{}", text(&listed.stderr));
    let defined = symbols(Utf8Path::new("."), &["--defined-only"], archive.as_str());
    let missing: Vec<String> = text(&listed.stdout)
        .lines()
        .filter_map(|line| line.split_whitespace().last())
        .map(|name| c_name(name).to_owned())
        .filter(|name| !name.ends_with(':') && !name.starts_with('_'))
        .filter(|name| !name.starts_with("typy_rt_heap_") && !defined.contains(name))
        .filter(|name| !typy_check::RUNTIME_C_FUNCTIONS.contains(&name.as_str()))
        .collect();
    assert_eq!(missing, Vec::<String>::new());
}

/// Returns what the symbolic link `name` names, or `None` when it is not a link.
fn link_target(project: &Project, name: &str) -> Option<String> {
    std::fs::read_link(project.root.join(name))
        .ok()
        .map(|target| target.to_string_lossy().into_owned())
}

/// Returns the soname the shared library `name` records.
#[cfg(target_os = "linux")]
fn soname(project: &Project, name: &str) -> String {
    let dynamic = Command::new("readelf")
        .args(["-d", name])
        .current_dir(&project.root)
        .output()
        .unwrap();
    text(&dynamic.stdout)
        .lines()
        .find(|line| line.contains("(SONAME)"))
        .and_then(|line| line.split('[').nth(1))
        .map(|rest| rest.trim_end_matches(']').to_owned())
        .unwrap_or_default()
}

#[test]
#[cfg(target_os = "linux")]
fn cli_13_a_versioned_shared_library_is_named_as_linux_names_libraries() {
    let versioned = format!("{TOOLS}__version__ = '5.0rc1'\n");
    let project = Project::new(&[("tools.py", &versioned), ("host.c", HOST)]);
    project.build(&["build", "--lib", "shared", "tools.py"]);
    assert!(project.root.join("libtools.so.5.0").is_file());
    assert_eq!(link_target(&project, "libtools.so.5.0"), None);
    assert_eq!(
        link_target(&project, "libtools.so.5").as_deref(),
        Some("libtools.so.5.0")
    );
    assert_eq!(
        link_target(&project, "libtools.so").as_deref(),
        Some("libtools.so.5.0")
    );
    assert_eq!(soname(&project, "libtools.so.5.0"), "libtools.so.5");
    project.tool("cc", &["-o", "host", "host.c", "-L.", "-ltools", "-lm"]);
    let run = project.run("host", &[]);
    assert_eq!(run.status.code(), Some(0), "{}", text(&run.stderr));
    assert_eq!(text(&run.stdout), format!("{HOST_OUTPUT}host ends\n"));
    // A new release moves the links, and the program finds it through its soname.
    std::fs::write(
        project.root.join("tools.py"),
        format!("{TOOLS}__version__ = '5.1'\n"),
    )
    .unwrap();
    project.build(&["build", "--lib", "shared", "tools.py"]);
    assert_eq!(
        link_target(&project, "libtools.so.5").as_deref(),
        Some("libtools.so.5.1")
    );
    assert_eq!(
        link_target(&project, "libtools.so").as_deref(),
        Some("libtools.so.5.1")
    );
    assert_eq!(project.run("host", &[]).status.code(), Some(0));
}

#[test]
#[cfg(target_os = "linux")]
fn cli_13_a_versioned_static_library_has_the_link_a_linker_finds() {
    let project = Project::new(&[("tools.py", TOOLS), ("host.c", HOST)]);
    std::fs::write(project.root.join("typy.toml"), "version = \"2.4\"\n").unwrap();
    project.build(&["build", "--lib", "static", "tools.py"]);
    assert!(project.root.join("libtools.2.4.a").is_file());
    assert_eq!(
        link_target(&project, "libtools.a").as_deref(),
        Some("libtools.2.4.a")
    );
    project.tool("cc", &["-o", "host", "host.c", "-L.", "-ltools", "-lm"]);
    assert_eq!(project.run("host", &[]).status.code(), Some(0));
}

#[test]
#[cfg(target_os = "linux")]
fn cli_13_an_output_path_names_the_library_alone() {
    let versioned = format!("{TOOLS}__version__ = '1.2'\n");
    let project = Project::new(&[("tools.py", &versioned)]);
    project.build(&["build", "--lib", "shared", "tools.py", "-o", "custom.so"]);
    assert!(project.root.join("custom.so").is_file());
    assert_eq!(soname(&project, "custom.so"), "custom.so");
    assert!(!project.root.join("libtools.so").exists());
    assert!(!project.root.join("libtools.so.1").exists());
}

/// Returns what `otool` prints for `arguments`, which must succeed.
#[cfg(target_os = "macos")]
fn otool(project: &Project, arguments: &[&str]) -> String {
    let run = Command::new("otool")
        .args(arguments)
        .current_dir(&project.root)
        .output()
        .unwrap();
    assert!(run.status.success(), "{}", text(&run.stderr));
    text(&run.stdout)
}

/// Returns the install name the shared library `name` records.
#[cfg(target_os = "macos")]
fn install_name(project: &Project, name: &str) -> String {
    otool(project, &["-D", name])
        .lines()
        .nth(1)
        .unwrap_or_default()
        .to_owned()
}

#[test]
#[cfg(target_os = "macos")]
fn cli_16_a_versioned_shared_library_is_named_as_macos_names_libraries() {
    let versioned = format!("{TOOLS}__version__ = '5.0rc1'\n");
    let project = Project::new(&[("tools.py", &versioned), ("host.c", HOST)]);
    project.build(&["build", "--lib", "shared", "tools.py"]);
    assert!(project.root.join("libtools.5.dylib").is_file());
    assert_eq!(link_target(&project, "libtools.5.dylib"), None);
    assert_eq!(
        link_target(&project, "libtools.dylib").as_deref(),
        Some("libtools.5.dylib")
    );
    assert_eq!(
        install_name(&project, "libtools.5.dylib"),
        "libtools.5.dylib"
    );
    project.tool("cc", &["-o", "host", "host.c", "-L.", "-ltools"]);
    let loads = otool(&project, &["-L", "host"]);
    assert!(
        loads.contains("libtools.5.dylib (compatibility version 5.0.0, current version 5.0.0)"),
        "{loads}"
    );
    let run = project.run("host", &[]);
    assert_eq!(run.status.code(), Some(0), "{}", text(&run.stderr));
    assert_eq!(text(&run.stdout), format!("{HOST_OUTPUT}host ends\n"));
    // A new release replaces the library of its major version, which the program still loads.
    std::fs::write(
        project.root.join("tools.py"),
        format!("{TOOLS}__version__ = '5.1'\n"),
    )
    .unwrap();
    project.build(&["build", "--lib", "shared", "tools.py"]);
    let versions = otool(&project, &["-L", "libtools.5.dylib"]);
    assert!(
        versions.contains("(compatibility version 5.0.0, current version 5.1.0)"),
        "{versions}"
    );
    assert_eq!(project.run("host", &[]).status.code(), Some(0));
}

#[test]
#[cfg(target_os = "macos")]
fn cli_16_a_versioned_static_library_for_macos_is_libname_a() {
    let project = Project::new(&[("tools.py", TOOLS), ("host.c", HOST)]);
    std::fs::write(project.root.join("typy.toml"), "version = \"2.4\"\n").unwrap();
    project.build(&["build", "--lib", "static", "tools.py"]);
    assert!(project.root.join("libtools.a").is_file());
    assert_eq!(link_target(&project, "libtools.a"), None);
    assert!(!project.root.join("libtools.2.4.a").exists());
    project.tool("cc", &["-o", "host", "host.c", "-L.", "-ltools"]);
    assert_eq!(project.run("host", &[]).status.code(), Some(0));
}

#[test]
#[cfg(target_os = "macos")]
fn cli_16_an_output_path_is_the_install_name_of_the_library_alone() {
    let versioned = format!("{TOOLS}__version__ = '1.2'\n");
    let project = Project::new(&[("tools.py", &versioned)]);
    project.build(&["build", "--lib", "shared", "tools.py", "-o", "custom.dylib"]);
    assert_eq!(install_name(&project, "custom.dylib"), "custom.dylib");
    assert!(!project.root.join("libtools.dylib").exists());
    assert!(!project.root.join("libtools.1.dylib").exists());
}

#[test]
fn cli_13_two_versions_that_differ_stop_the_build() {
    let versioned = format!("{TOOLS}__version__ = '1.2'\n");
    let project = Project::new(&[("tools.py", &versioned)]);
    std::fs::write(project.root.join("typy.toml"), "version = \"1.3\"\n").unwrap();
    let output = project.typy(&["build", "--lib", "static", "tools.py"]);
    assert_eq!(output.status.code(), Some(1));
    let error = text(&output.stderr);
    assert!(
        error.starts_with("error[E0429]: `__version__` is \"1.2\" and `version`"),
        "{error}"
    );
    assert!(!project.root.join("libtools.1.2.a").exists());
}

#[test]
fn ffi_08_cpython_calls_a_library_through_the_binding_the_build_writes() {
    let script = "\
import sys

sys.path.insert(0, '.')
import tools_binding as tools

print('python', tools.add(2, 3), tools.positive(-1), tools.scale(2.0, 0.25))
tools.describe(9)
print(tools.__all__)
";
    let versioned = format!("{TOOLS}__version__ = '3.2'\n");
    let project = Project::new(&[("tools.py", &versioned), ("use_tools.py", script)]);
    project.build(&["build", "--lib", "shared", "tools.py"]);
    let binding = std::fs::read_to_string(project.root.join("tools_binding.py")).unwrap();
    assert!(
        binding.contains(&format!(
            "_library = ctypes.CDLL(\"{}\")",
            loaded_name("tools", Some("3"))
        )),
        "{binding}"
    );
    assert!(binding.contains("def add(a: int, b: int) -> int:\n    return _library.add(a, b)\n"));
    let stub = std::fs::read_to_string(project.root.join("tools_binding.pyi")).unwrap();
    assert!(
        stub.contains("def describe(number: int) -> None: ..."),
        "{stub}"
    );
    let run = Command::new("python3")
        .args(["-I", "-u", "use_tools.py"])
        .current_dir(&project.root)
        .env(LIBRARY_PATH, &project.root)
        .output()
        .unwrap();
    assert_eq!(run.status.code(), Some(0), "{}", text(&run.stderr));
    assert_eq!(
        text(&run.stdout),
        "tools loaded as tools\npython 5 False 0.5\nnumber 9 calls 1 module tools\n\
         ['add', 'scale', 'positive', 'describe', 'fail', 'leave', 'depth']\n"
    );
}

#[test]
fn ffi_08_the_binding_and_its_stub_are_python_every_oracle_compiles() {
    let project = Project::new(&[("tools.py", TOOLS)]);
    project.build(&["build", "--lib", "static", "tools.py"]);
    let binding = std::fs::read_to_string(project.root.join("tools_binding.py")).unwrap();
    assert!(
        binding.contains(&format!("ctypes.CDLL(\"{}\")", loaded_name("tools", None))),
        "{binding}"
    );
    for python in ["python3.12", "python3.13", "python3.14", "python3.15"] {
        for file in ["tools_binding.py", "tools_binding.pyi"] {
            let compiled = Command::new(python)
                .args([
                    "-I",
                    "-c",
                    "import ast, sys; ast.parse(open(sys.argv[1]).read())",
                    file,
                ])
                .current_dir(&project.root)
                .output()
                .unwrap();
            assert!(
                compiled.status.success(),
                "{python} {file}: {}",
                text(&compiled.stderr)
            );
        }
    }
}

#[test]
fn ffi_04_a_program_calls_a_library_through_its_binding_linked_as_typy_toml_says() {
    let program = "\
import tools_binding as tools

print('program', tools.add(40, 2), tools.scale(1.5, 2.0), tools.positive(-4))
";
    let library = format!("{TOOLS}__version__ = '2.0'\n");
    let project = Project::new(&[("app.py", program)]);
    std::fs::create_dir(project.root.join("lib")).unwrap();
    std::fs::write(project.root.join("lib/tools.py"), library).unwrap();
    project.build(&["build", "--lib", "shared", "lib/tools.py"]);
    project.build(&["build", "--lib", "static", "lib/tools.py"]);
    let expected = "tools loaded as tools\nprogram 42 3.0 False\n";
    let oracle = Command::new("python3")
        .args(["-u", "app.py"])
        .current_dir(&project.root)
        .env("PYTHONPATH", "lib")
        .env(LIBRARY_PATH, project.root.join("lib"))
        .output()
        .unwrap();
    assert_eq!(text(&oracle.stdout), expected, "{}", text(&oracle.stderr));
    let settings = "module-paths = [\"lib\"]\nlibrary-paths = [\"lib\"]\n";
    std::fs::write(project.root.join("typy.toml"), settings).unwrap();
    project.build(&["build", "app.py", "-o", "dynamic"]);
    let dynamic = Command::new(project.root.join("dynamic"))
        .env(LIBRARY_PATH, project.root.join("lib"))
        .output()
        .unwrap();
    assert_eq!(text(&dynamic.stdout), expected, "{}", text(&dynamic.stderr));
    std::fs::write(
        project.root.join("typy.toml"),
        format!(
            "{settings}\n[libraries.\"{}\"]\nlink = \"static\"\n",
            loaded_name("tools", Some("2"))
        ),
    )
    .unwrap();
    project.build(&["build", "app.py", "-o", "static"]);
    // The libraries the executable loads: `readelf -d` on Linux, `otool -L` on macOS.
    let (lister, listing) = if cfg!(target_os = "macos") {
        ("otool", "-L")
    } else {
        ("readelf", "-d")
    };
    let linked = Command::new(lister)
        .args([listing, "static"])
        .current_dir(&project.root)
        .output()
        .unwrap();
    assert!(
        !text(&linked.stdout).contains("libtools"),
        "{}",
        text(&linked.stdout)
    );
    let run = Command::new(project.root.join("static")).output().unwrap();
    assert_eq!(text(&run.stdout), expected, "{}", text(&run.stderr));
}

#[test]
fn ffi_04_a_library_the_linker_cannot_find_fails_the_build() {
    let program = "\
import ctypes

missing = ctypes.CDLL(\"libtypy_missing.so.1\")
missing.nothing.argtypes = []
missing.nothing.restype = None
missing.nothing()
";
    let project = Project::new(&[("app.py", program)]);
    let output = project.typy(&["build", "app.py"]);
    assert_eq!(output.status.code(), Some(1));
    let error = text(&output.stderr);
    assert!(error.starts_with("error[E0702]"), "{error}");
    assert!(error.contains("libtypy_missing.so.1"), "{error}");
}
