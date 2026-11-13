//! Tests of checking `open`, text files and `OSError` through the public API (LIB-04,
//! LIB-14, EXC-14, ADR-0019): what TyPy accepts, and the code and message of each refusal.

use typy_check::TypeChecker;
use typy_source::{Project, ProjectSettings, PythonFloor};

/// Returns each diagnostic of `source`, checked at `floor`, as its code and message.
fn diagnostics_at(floor: PythonFloor, source: &str) -> Vec<(String, String)> {
    use std::sync::atomic::{AtomicU32, Ordering};
    static NEXT: AtomicU32 = AtomicU32::new(0);
    let base = camino::Utf8PathBuf::from_path_buf(std::env::temp_dir()).unwrap();
    let directory = base.join(format!(
        "typy-open-{}-{}",
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

fn one_error(source: &str) -> (String, String) {
    let found = diagnostics(source);
    assert_eq!(found.len(), 1, "{source}: {found:?}");
    found[0].clone()
}

#[test]
fn text_modes_encodings_and_file_methods_are_accepted() {
    for source in [
        "f = open(\"a.txt\")\nprint(f.read(), f.read(2), f.readline(), f.readline(3))\n",
        "f = open(\"a.txt\", \"w\", encoding=\"utf-8\")\nf.write(\"x\")\nf.flush()\nf.close()\n",
        "f = open(file=\"a.txt\", mode=\"at\", encoding=None)\nf.writelines([\"a\", \"b\"])\n",
        "f = open(\"a.txt\", \"x\", -1, \"utf8\")\n",
        "f = open(\"a.txt\", \"r\", -1, None, \"strict\", None, True, None)\n",
        "lines: list[str] = open(\"a.txt\").readlines()\n",
        "with open(\"a.txt\") as f:\n    for line in f:\n        print(line)\n",
        "print([line for line in open(\"a.txt\")], next(open(\"a.txt\")))\n",
        "print(sorted(open(\"a.txt\")), list(iter(open(\"a.txt\"))))\n",
        "f = open(\"a.txt\", \"w\")\nprint(\"x\", file=f)\nf.writelines(open(\"b.txt\"))\n",
        "try:\n    open(\"a\")\nexcept (FileNotFoundError, PermissionError, IOError, EnvironmentError):\n    pass\n",
        "raise OSError(\"one message\")\n",
        "class Missing(FileNotFoundError):\n    pass\n\n\nraise Missing(\"gone\")\n",
    ] {
        assert_eq!(diagnostics(source), [], "{source}");
    }
}

#[test]
fn modes_open_refuses_are_e0404_with_cpythons_message() {
    for (mode, message) in [
        (
            "rw",
            "must have exactly one of create/read/write/append mode",
        ),
        ("z", "invalid mode: 'z'"),
        ("bt", "can't have text and binary mode at once"),
        (
            "t",
            "Must have exactly one of create/read/write/append mode and at most one plus",
        ),
    ] {
        let source = format!("f = open(\"a.txt\", \"{mode}\")\n");
        assert_eq!(
            one_error(&source),
            (
                "E0404".to_owned(),
                format!("`open` raises `ValueError: {message}` for this mode")
            ),
            "{mode}"
        );
    }
}

#[test]
fn what_0_4_brings_to_open_is_e0201() {
    for (source, message) in [
        (
            "f = open(\"a\", \"rb\")\n",
            "a binary mode of `open` is planned for TyPy 0.4 (LIB-14)",
        ),
        (
            "f = open(\"a\", \"r+\")\n",
            "a mode with `+` of `open` is planned for TyPy 0.4 (LIB-14)",
        ),
        (
            "m = \"r\"\nf = open(\"a\", m)\n",
            "a mode of `open` that is not a string literal is planned for TyPy 0.4 (LIB-14)",
        ),
        (
            "f = open(\"a\", buffering=1)\n",
            "the `buffering` argument of `open` is planned for TyPy 0.4 (LIB-14)",
        ),
        (
            "f = open(\"a\", \"r\", -1, None, \"ignore\")\n",
            "the `errors` argument of `open` is planned for TyPy 0.4 (LIB-14)",
        ),
        (
            "f = open(\"a\", newline=\"\")\n",
            "the `newline` argument of `open` is planned for TyPy 0.4 (LIB-14)",
        ),
        (
            "f = open(\"a\", encoding=\"latin-1\")\n",
            "TyPy 0.3 opens files in UTF-8 only, written as `\"utf-8\"` or `None`; other \
             encodings are planned for TyPy 0.4 (LIB-14)",
        ),
        (
            "f = open(\"a\")\nf.seek(0)\n",
            "`seek` of a text file is planned for TyPy 0.4 (LIB-14)",
        ),
        (
            "f = open(\"a\")\nprint(f.name)\n",
            "`name` of a text file is planned for TyPy 0.4 (LIB-14)",
        ),
        (
            "import sys\nprint(sys.stdout.closed)\n",
            "`closed` of a text file is planned for TyPy 0.4 (LIB-14)",
        ),
    ] {
        assert_eq!(
            one_error(source),
            ("E0201".to_owned(), message.to_owned()),
            "{source}"
        );
    }
}

#[test]
fn calls_open_cannot_take_are_e0404() {
    for (source, message) in [
        ("f = open()\n", "`open` needs the file to open"),
        (
            "f = open(\"a\", colour=1)\n",
            "`open` has no parameter named `colour`",
        ),
        (
            "f = open(\"a\", \"r\", mode=\"w\")\n",
            "`open` is given `mode` twice",
        ),
        (
            "f = open(\"a\", \"r\", 1, 2, 3, 4, 5, 6, 7)\n",
            "`open` takes at most 8 arguments, and 9 are given",
        ),
        (
            "f = open(3)\n",
            "the file of `open` must be a str, and this is an int",
        ),
        (
            "f = open(\"a\")\nf.writelines([1])\n",
            "`writelines` writes items of type str, and these items are int",
        ),
    ] {
        assert_eq!(
            one_error(source),
            ("E0404".to_owned(), message.to_owned()),
            "{source}"
        );
    }
    assert_eq!(
        one_error("f = open(\"a\")\nf.flip()\n"),
        (
            "E0412".to_owned(),
            "a TextIOWrapper has no attribute `flip` in TyPy".to_owned()
        )
    );
    assert_eq!(one_error("d = {\"a\": 1}\nf = open(**d)\n").0, "E0201");
}

#[test]
fn an_os_error_of_two_or_more_arguments_is_planned() {
    let message = "an `OSError` of two or more arguments, which CPython reads as `errno`, \
                   `strerror` and `filename`, is planned for TyPy 0.4 (EXC-14)";
    for source in [
        "raise OSError(2, \"No such file\")\n",
        "raise FileNotFoundError(2, \"No such file\", \"a.txt\")\n",
        "class Missing(OSError):\n    pass\n\n\nraise Missing(1, 2)\n",
        "class Missing(OSError):\n    def __init__(self, path: str) -> None:\n        super().__init__(2, path)\n\n\nraise Missing(\"a\")\n",
    ] {
        assert_eq!(
            one_error(source),
            ("E0201".to_owned(), message.to_owned()),
            "{source}"
        );
    }
}

#[test]
fn the_typed_dump_shows_open_and_a_with_of_a_file() {
    let base = camino::Utf8PathBuf::from_path_buf(std::env::temp_dir()).unwrap();
    let directory = base.join(format!("typy-open-dump-{}", std::process::id()));
    std::fs::create_dir_all(&directory).unwrap();
    std::fs::write(
        directory.join("app.py"),
        "with open(\"a.txt\", \"wt\") as f:\n    f.write(\"x\")\n",
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
        "With as f",
        "Open \"wt\" Write : TextIOWrapper",
        "Constant Str(\"a.txt\")",
    ] {
        assert!(dump.contains(expected), "{expected} in\n{dump}");
    }
}

#[test]
fn a_file_has_no_order_to_reverse() {
    for source in [
        "for line in reversed(open(\"a\")):\n    print(line)\n",
        "for item in reversed(iter([1])):\n    print(item)\n",
    ] {
        let (code, message) = one_error(source);
        assert_eq!(code, "E0411", "{source}");
        assert!(
            message.starts_with("`reversed` needs a sequence"),
            "{message}"
        );
    }
}

#[test]
fn the_lines_of_a_file_are_str_in_every_loop() {
    let source = concat!(
        "for number, line in enumerate(open(\"a\"), 1):\n",
        "    print(number + 1, line.rstrip())\n",
        "for first, second in zip(open(\"a\"), open(\"b\")):\n",
        "    print(first.strip() + second)\n",
        "print({line: len(line) for line in open(\"a\")})\n",
    );
    assert_eq!(diagnostics(source), []);
}

#[test]
fn an_error_inside_the_arguments_of_open_is_the_only_one_reported() {
    for source in ["f = open(missing)\n", "f = open(\"a\", missing)\n"] {
        assert_eq!(one_error(source).0, "E0205", "{source}");
    }
}
