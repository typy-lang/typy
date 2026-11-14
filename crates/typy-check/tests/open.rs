//! Tests of checking `open`, text files and `OSError` through the public API (LIB-04,
//! LIB-14, EXC-14, EXC-16, ADR-0019): what TyPy accepts, and the code and message of each refusal.

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
fn what_is_planned_for_open_is_e0201() {
    for (source, message) in [
        (
            "n = 0\nf = open(\"a\", \"rb\", n)\n",
            "a `buffering` that is not a literal decides the class of a binary file when the \
             program runs, which is planned for TyPy 1.0 (LIB-19)",
        ),
        (
            "f = open(\"a\", closefd=False)\n",
            "the `closefd` argument of `open` is planned for TyPy 0.4 (LIB-14)",
        ),
        (
            "f = open(\"a\", \"r\", -1, None, None, None, True, len)\n",
            "the `opener` argument of `open` is planned for TyPy 1.0 (LIB-18)",
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
fn open_takes_buffering_an_encoding_errors_and_newline() {
    let source = "\
n: str | None = None
size = 1
a = open(\"a\", \"w\", size, \"Latin-1\", \"xmlcharrefreplace\", \"\\r\\n\", True, None)
b = open(\"a\", encoding=\"ascii\", errors=\"surrogateescape\", newline=n)
c = open(\"a\", buffering=-1, encoding=None, errors=None, newline=None)
d = open(\"a\", encoding=\" UTF8 \", newline=\"\")
e = open(\"a\", \"r+\")
g = open(\"a\", \"+wt\")
m = \"w\"
h = open(\"a\", m, encoding=\"ascii\")
text: str = open(\"a\", m + \"t\").read()
";
    assert_eq!(diagnostics_at(PythonFloor::V3_12, source), []);
}

#[test]
fn a_binary_mode_gives_the_class_cpython_makes() {
    let source = "\
r = open(\"a\", \"rb\")
w = open(\"a\", \"ab\", -1, None, None, None)
p = open(\"a\", \"r+b\", 4096)
raw = open(\"a\", \"wb\", buffering=0)
data: bytes = r.read() + r.read(None) + r.readline(2) + p.read(1) + raw.read()
lines: list[bytes] = r.readlines()
for line in r:
    data = data + line
count: int = w.write(b\"x\") + p.write(data) + raw.write(b\"\") + p.seek(-1, 2) + w.tell()
w.writelines([b\"a\", b\"b\"])
with open(\"a\", \"rb\") as inside:
    data = inside.read()
";
    assert_eq!(diagnostics_at(PythonFloor::V3_12, source), []);
    for (source, code, message) in [
        (
            "f = open(\"a\", \"wb\")\nf.write(\"text\")\n",
            "E0404",
            "`text` of `write` must be a bytes, and this is a str",
        ),
        (
            "f = open(\"a\", \"rb\")\ns: str = f.read()\n",
            "E0402",
            "`s` was declared as str and is assigned a bytes",
        ),
        (
            "f = open(\"a\", \"rb\")\nf.peek()\n",
            "E0412",
            "a BufferedReader has no attribute `peek` in TyPy",
        ),
        (
            "import sys\nf = open(\"a\", \"wb\")\nprint(1, file=f)\n",
            "E0404",
            "`file` must be a TextIOWrapper, and this is a BufferedWriter",
        ),
    ] {
        assert_eq!(
            one_error(source),
            (code.to_owned(), message.to_owned()),
            "{source}"
        );
    }
}

#[test]
fn what_a_binary_mode_does_not_take_is_e0404() {
    for (source, message) in [
        (
            "f = open(\"a\", \"rb\", encoding=\"utf-8\")\n",
            "`open` raises `ValueError: binary mode doesn't take an encoding argument` for this \
             mode",
        ),
        (
            "f = open(\"a\", \"wb\", errors=\"strict\")\n",
            "`open` raises `ValueError: binary mode doesn't take an errors argument` for this mode",
        ),
        (
            "f = open(\"a\", \"ab\", newline=\"\")\n",
            "`open` raises `ValueError: binary mode doesn't take a newline argument` for this mode",
        ),
        (
            "f = open(\"a\", \"rb\", 1)\n",
            "CPython warns that `buffering=1` does not buffer lines in a binary mode, naming the \
             program's file and line; leave it out",
        ),
    ] {
        assert_eq!(
            one_error(source),
            ("E0404".to_owned(), message.to_owned()),
            "{source}"
        );
    }
}

#[test]
fn an_argument_open_cannot_take_is_e0404() {
    for (source, message) in [
        (
            "f = open(\"a\", newline=\"x\")\n",
            "`open` raises `ValueError: illegal newline value: x` for this newline",
        ),
        (
            "f = open(\"a\", encoding=\"utf-16\")\n",
            "TyPy provides the codecs `utf-8`, `ascii` and `latin-1`, and \"utf-16\" is none of \
             them",
        ),
        (
            "e = \"ascii\"\nf = open(\"a\", encoding=e)\n",
            "`encoding` of `open` is a string literal, so TyPy can check it when compiling",
        ),
        (
            "f = open(\"a\", errors=\"namereplace\")\n",
            "TyPy provides the error handlers `strict`, `ignore`, `replace`, `backslashreplace`, \
             `xmlcharrefreplace` and `surrogateescape`, and \"namereplace\" is none of them",
        ),
        (
            "f = open(\"a\", buffering=\"1\")\n",
            "`buffering` of `open` must be an int, and this is a str",
        ),
        (
            "f = open(\"a\", 1)\n",
            "the mode of `open` must be a str, and this is an int",
        ),
        (
            "f = open(\"a\", newline=1)\n",
            "`newline` of `open` must be a str | None, and this is an int",
        ),
    ] {
        assert_eq!(
            one_error(source),
            ("E0404".to_owned(), message.to_owned()),
            "{source}"
        );
    }
}

#[test]
fn a_text_file_gives_its_methods_and_attributes_with_their_types() {
    let source = "\
import sys

f = open(\"a\")
n: int = f.fileno() + f.tell() + f.seek(0) + f.seek(0, 2) + f.truncate() + f.truncate(None)
m: int = f.truncate(3)
b: bool = f.isatty() or f.readable() or f.writable() or f.seekable() or f.closed
s: str = f.name + f.mode + sys.stdin.name
";
    assert_eq!(diagnostics_at(PythonFloor::V3_12, source), []);
}

#[test]
fn a_wrong_use_of_a_text_file_name_is_reported() {
    for (source, code, message) in [
        (
            "f = open(\"a\")\nf.name()\n",
            "E0401",
            "`name` of a text file is a str and cannot be called",
        ),
        (
            "f = open(\"a\")\nf.closed()\n",
            "E0401",
            "`closed` of a text file is a bool and cannot be called",
        ),
        (
            "f = open(\"a\")\nprint(f.encoding)\n",
            "E0412",
            "a TextIOWrapper has no attribute `encoding` in TyPy",
        ),
        (
            "f = open(\"a\")\nf.detach()\n",
            "E0412",
            "a TextIOWrapper has no attribute `detach` in TyPy",
        ),
        (
            "f = open(\"a\")\nf.seek(\"0\")\n",
            "E0404",
            "`cookie` of `seek` must be an int, and this is a str",
        ),
        (
            "f = open(\"a\")\nf.truncate(1.5)\n",
            "E0404",
            "`pos` of `truncate` must be an int, and this is a float",
        ),
    ] {
        assert_eq!(
            one_error(source),
            (code.to_owned(), message.to_owned()),
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
fn an_os_error_of_two_to_five_arguments_of_known_types_is_accepted() {
    for source in [
        "raise OSError(2, \"No such file\")\n",
        "raise FileNotFoundError(2, \"No such file\", \"a.txt\", None, \"b.txt\")\n",
        "class Missing(OSError):\n    pass\n\n\nraise Missing(2, \"gone\", None, 3)\n",
        "error = OSError(2, \"x\")\nprint(error.errno, error.strerror, error.filename, error.filename2)\n",
    ] {
        assert_eq!(
            diagnostics(source),
            Vec::<(String, String)>::new(),
            "{source}"
        );
    }
}

#[test]
fn an_os_error_of_other_arguments_is_planned() {
    for source in [
        "raise OSError(2, 3)\n",
        "raise OSError(\"2\", \"x\")\n",
        "raise OSError(2, \"x\", 3)\n",
        "raise OSError(2, \"x\", None, \"w\")\n",
        "raise OSError(1, \"a\", \"b\", None, \"c\", 6)\n",
        "class Missing(OSError):\n    pass\n\n\nraise Missing(1, 2)\n",
        "class Missing(OSError):\n    def __init__(self, path: str) -> None:\n        super().__init__(2, path)\n\n\nraise Missing(\"a\")\n",
        "class Missing(OSError):\n    def __init__(self, code: int, path: str) -> None:\n        self.path = path\n\n\nraise Missing(2, \"a\")\n",
    ] {
        let (code, message) = one_error(source);
        assert_eq!(code, "E0201", "{source}");
        assert!(message.ends_with("(EXC-16)"), "{source}: {message}");
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
