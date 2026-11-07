//! Links real programs with the staged runtime archive and runs them (UC-02).

use std::process::Command;

use camino::Utf8PathBuf;
use typy_build::{ArtifactKind, BuildMode, Linker, RuntimeArchive};
use typy_check::TypeChecker;
use typy_codegen::{Target, object};
use typy_source::{Project, ProjectSettings};

/// The archive `make runtime` stages in Cargo's target directory; `make test` builds it first.
fn archive() -> RuntimeArchive {
    let target = option_env!("CARGO_TARGET_DIR").map_or_else(
        || Utf8PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../target"),
        Utf8PathBuf::from,
    );
    let path = target
        .join("lib/typy")
        .join(Target::host().triple.as_str())
        .join("libtypy_runtime.a");
    RuntimeArchive::open(&path)
        .unwrap_or_else(|diagnostics| panic!("{diagnostics:?}; run `make runtime`"))
}

/// Builds `source` in `mode`, runs it, and returns its exit status, standard output and error.
fn run(source: &str, mode: BuildMode) -> (i32, String, String) {
    let directory = tempfile::tempdir().unwrap();
    let root = Utf8PathBuf::from_path_buf(directory.path().to_owned()).unwrap();
    std::fs::write(root.join("app.py"), source).unwrap();
    let project = Project::load(&root.join("app.py"), ProjectSettings::new(root.clone()))
        .expect("the program parses");
    let program = TypeChecker::check(&project).expect("the program checks");
    let module = typy_lower::lower(&program).expect("the program lowers");
    let object = object(&module, Target::host(), mode.optimization(), &project).unwrap();
    let linker = Linker::new(Target::host().triple, archive());
    let artifact = linker
        .link(&object, &root.join("app"))
        .unwrap_or_else(|diagnostics| panic!("{diagnostics:?}"));
    assert_eq!(artifact.kind, ArtifactKind::Executable);
    let mode = std::os::unix::fs::PermissionsExt::mode(
        &std::fs::metadata(&artifact.path).unwrap().permissions(),
    );
    assert_eq!(mode & 0o555, 0o555, "{mode:o}");
    let output = Command::new(&artifact.path).output().unwrap();
    (
        output.status.code().unwrap_or(-1),
        String::from_utf8(output.stdout).unwrap(),
        String::from_utf8(output.stderr).unwrap(),
    )
}

#[test]
fn print_of_an_addition_runs_in_every_mode() {
    for mode in [
        BuildMode::Debug,
        BuildMode::Release,
        BuildMode::ReleaseSmall,
    ] {
        assert_eq!(
            run("print(1 + 2)\n", mode),
            (0, "3\n".to_owned(), String::new()),
            "{mode:?}"
        );
    }
}

/// Programs and what CPython 3.15 printed for them: standard output, exit status, and the last
/// line of standard error (STD-02, STD-05).
const PROGRAMS: &[(&str, &str, i32, &str)] = &[
    (
        "def fib(n: int) -> int:\n    if n < 2:\n        return n\n    return fib(n - 1) + fib(n - 2)\nprint(fib(20))\n",
        "6765\n",
        0,
        "",
    ),
    (
        "def greet(name: str, punctuation: str = \"!\") -> str:\n    return \"hello \" + name + punctuation\nprint(greet(\"world\"), greet(\"you\", \"?\"))\n",
        "hello world! hello you?\n",
        0,
        "",
    ),
    (
        "total = 0\nfor i in range(10):\n    total += i\nwhile total > 100:\n    total = total // 2\nelse:\n    pass\nprint(total, sep=\", \", end=\"\\n\")\n",
        "45\n",
        0,
        "",
    ),
    (
        "print(len(\"abc\"), abs(-2.5), abs(-3), min(3, 1, 2), max(\"a\", \"b\"), int(\"12\"), float(3), str(1.5), bool(0))\n",
        "3 2.5 3 1 b 12 3.0 1.5 False\n",
        0,
        "",
    ),
    (
        "print(repr(\"x\"), ord(\"a\"), chr(98), \"abc\"[-1], 7 / 2, 2 ** 10, 1 < 2 <= 3, not True, 1.5 != 2.5)\n",
        "'x' 97 b c 3.5 1024 True False True\n",
        0,
        "",
    ),
    (
        "print(1 < 2.5, 2.5 >= 1, \"a\" < \"b\" < \"c\", True == False, 1 != 2, not 1, not \"\", 1 and 2, \"\" or \"x\")\n",
        "True True True False True False True 2 x\n",
        0,
        "",
    ),
    (
        "y = 1.5\ny /= 2.0\ny += 1.0\ns = \"a\"\ns += \"\\u00e9\\u03b1\\U0001f600\"\ns *= 2\nprint(y, s, -y, 123456789012345678901234567890)\n",
        "1.75 a\u{e9}\u{3b1}\u{1f600}a\u{e9}\u{3b1}\u{1f600} -1.75 123456789012345678901234567890\n",
        0,
        "",
    ),
    (
        "n = 3\nfor i in range(10, 0, -n):\n    if i == 4:\n        break\n    print(i)\nelse:\n    print(\"done\")\n",
        "10\n7\n",
        0,
        "",
    ),
    (
        "x = 2 ** 100\nprint(x, x * x - 1, -x // 3, x % 7, x >> 90, 1 << 70, ~x, x & 0xff, 10 ** 30 / 7)\n",
        "1267650600228229401496703205376 1606938044258990275541962092341162602522202993782792835301375 -422550200076076467165567735126 2 1024 1180591620717411303424 -1267650600228229401496703205377 0 1.4285714285714285e+29\n",
        0,
        "",
    ),
    (
        "print(0.1 + 0.2, 1e16, 1e-5, 2.5 // 1, -7.5 % 2, 10 / 4, float(\"  1_000.5 \"), int(-2.9), 3 == 3.0)\n",
        "0.30000000000000004 1e+16 1e-05 2.0 0.5 2.5 1000.5 -2 True\n",
        0,
        "",
    ),
    (
        "print(int(\"-0x_ff\", 0), int(\"z\", 36), str(-12), repr(\"it's\"), chr(0x1F600), ord(\"\\u00e9\"), \"ab\" * 3, \"x\" < \"xy\")\n",
        "-255 35 -12 \"it's\" \u{1f600} 233 ababab True\n",
        0,
        "",
    ),
    (
        "print(1 // 0)\n",
        "",
        1,
        "ZeroDivisionError: division by zero",
    ),
    (
        "def f(n: int) -> int:\n    return 10 // n\ndef g(n: int) -> int:\n    return f(n) + 1\nprint(g(5))\nprint(g(0))\n",
        "3\n",
        1,
        "ZeroDivisionError: division by zero",
    ),
    (
        "print(\"abc\"[5])\n",
        "",
        1,
        "IndexError: string index out of range",
    ),
    (
        "print(int(\"12x\"))\n",
        "",
        1,
        "ValueError: invalid literal for int() with base 10: '12x'",
    ),
    (
        "def down(n: int) -> int:\n    return down(n + 1)\nprint(down(0))\n",
        "",
        1,
        "RecursionError: maximum recursion depth exceeded",
    ),
    (
        "print(\"\")\nprint()\nprint(None)\nprint(True, False, sep=\"-\", end=\"!\\n\")\n",
        "\n\nNone\nTrue-False!\n",
        0,
        "",
    ),
];

#[test]
fn programs_print_what_cpython_prints() {
    for (source, stdout, status, last_error_line) in PROGRAMS {
        let (code, out, error) = run(source, BuildMode::Debug);
        assert_eq!((code, out.as_str()), (*status, *stdout), "{source}");
        assert_eq!(
            error.lines().last().unwrap_or(""),
            *last_error_line,
            "{source}"
        );
    }
}

#[test]
fn an_uncaught_exception_prints_a_traceback_with_every_frame() {
    let source = "def f(n: int) -> int:\n    return 10 // n\ndef g(n: int) -> int:\n    return f(n) + 1\nprint(g(0))\n";
    let (code, out, error) = run(source, BuildMode::Release);
    assert_eq!((code, out.as_str()), (1, ""));
    assert_eq!(
        error,
        "Traceback (most recent call last):\n  File \"app.py\", line 5, in <module>\n  File \"app.py\", line 4, in g\n  File \"app.py\", line 2, in f\nZeroDivisionError: division by zero\n"
    );
}
