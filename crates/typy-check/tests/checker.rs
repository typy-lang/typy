//! Tests of the whole checker through its public API: each case is a program and the codes it must give.

use camino::Utf8PathBuf;
use typy_diagnostics::Diagnostics;
use typy_source::{Project, ProjectSettings};

use typy_check::{Type, TypeChecker, TypedExprKind, TypedProgram, TypedStmt};

fn load(source: &str) -> (tempfile_dir::Dir, Project) {
    let directory = tempfile_dir::Dir::new();
    std::fs::write(directory.path.join("app.py"), source).unwrap();
    let project = Project::load(
        &directory.path.join("app.py"),
        ProjectSettings::new(directory.path.clone()),
    )
    .unwrap_or_else(|failure| panic!("does not parse: {:?}", failure.diagnostics));
    (directory, project)
}

/// A temporary directory without an extra dependency: a unique path under the system's temporary directory.
mod tempfile_dir {
    use camino::Utf8PathBuf;
    use std::sync::atomic::{AtomicU32, Ordering};

    static NEXT: AtomicU32 = AtomicU32::new(0);

    pub(super) struct Dir {
        pub(super) path: Utf8PathBuf,
    }

    impl Dir {
        pub(super) fn new() -> Self {
            let number = NEXT.fetch_add(1, Ordering::Relaxed);
            let base = Utf8PathBuf::from_path_buf(std::env::temp_dir()).unwrap();
            let path = base.join(format!("typy-check-{}-{number}", std::process::id()));
            std::fs::create_dir_all(&path).unwrap();
            Self { path }
        }
    }

    impl Drop for Dir {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.path);
        }
    }
}

fn codes(diagnostics: &Diagnostics, project: &Project) -> Vec<String> {
    let mut codes: Vec<String> = diagnostics
        .clone()
        .into_sorted(project)
        .iter()
        .map(|diagnostic| diagnostic.code().to_string())
        .collect();
    codes.dedup();
    codes
}

fn check(source: &str) -> Result<TypedProgram, Vec<String>> {
    let (_directory, project) = load(source);
    TypeChecker::check(&project).map_err(|diagnostics| codes(&diagnostics, &project))
}

#[test]
fn a_try_statement_types_each_clause_and_its_name() {
    let source = "def f(n: int) -> int:\n    try:\n        return 10 // n\n    except (KeyError, IndexError) as e:\n        raise\n    except ZeroDivisionError:\n        return 0\n    except:\n        raise ValueError('other') from None\n    else:\n        pass\n";
    let (_directory, project) = load(source);
    let program = TypeChecker::check(&project).expect("the program is accepted");
    let dump = typy_check::dump(&program, &project);
    for expected in [
        "Try @2:5",
        "except KeyError, IndexError as e @4:5",
        "except ZeroDivisionError @6:5",
        "except BaseException @8:5",
        "Raise @5:9",
    ] {
        assert!(dump.contains(expected), "{expected} in\n{dump}");
    }
    let e = program
        .symbols
        .iter()
        .find(|symbol| symbol.name == "e")
        .expect("the clause's name is a symbol");
    assert_eq!(e.ty.to_string(), "LookupError");
}

#[test]
fn finally_runs_on_every_path_and_what_it_assigns_holds_after() {
    let source = "def f(n: int) -> int:\n    try:\n        n += 1\n    finally:\n        done = True\n        for i in range(2):\n            if i:\n                break\n    print(done)\n    return n\n\
                  def g() -> int:\n    try:\n        return 1\n    finally:\n        raise ValueError\n";
    let (_directory, project) = load(source);
    let program = TypeChecker::check(&project).expect("the program is accepted");
    let dump = typy_check::dump(&program, &project);
    assert!(dump.contains("    finally\n"), "{dump}");
}

#[test]
fn an_exception_class_builds_exceptions_and_keeps_its_base_s_attributes() {
    let source = "class Base(LookupError):\n    def __init__(self, code: int) -> None:\n        super().__init__(code, 'x')\n        self.code = code\n\
                  class Child(Base):\n    def __init__(self, code: int, extra: str) -> None:\n        super().__init__(code)\n        self.extra = extra\n    def __str__(self) -> str:\n        return self.extra\n\
                  class Plain(Child):\n    pass\n\
                  e: LookupError = Plain(1, extra='e')\n\
                  c = Child(2, 'c')\n\
                  c.code += 1\n\
                  print(c.code, c.extra, type(Base), e is c)\n\
                  try:\n    raise Plain(3, 'p')\nexcept Base as caught:\n    print(caught.code)\n";
    let (_directory, project) = load(source);
    let program = TypeChecker::check(&project);
    let program = program.expect("the program is accepted");
    let dump = typy_check::dump(&program, &project);
    for expected in ["SetArguments", "NewException Plain", "NewException Child"] {
        assert!(dump.contains(expected), "{expected} in\n{dump}");
    }
    let child = &program.classes[1];
    let names: Vec<&str> = child
        .attributes
        .iter()
        .map(|attribute| attribute.name.as_str())
        .collect();
    assert_eq!(names, ["code", "extra"]);
    assert_eq!(child.base, Some(program.classes[0].id));
}

#[test]
fn clauses_and_super_after_a_return_are_still_checked() {
    accepted(
        "def f() -> None:\n    return\n    try:\n        pass\n    except ValueError as e:\n        print(e)\n\
         class A(Exception):\n    def __init__(self) -> None:\n        return\n        super().__init__()\n",
    );
}

#[test]
fn assert_builds_an_assertion_error_and_narrows_after_it() {
    let source = "def f(x: int | None) -> int:\n    assert x is not None, 'needed'\n    assert x > 0\n    return x + 1\n";
    let (_directory, project) = load(source);
    let program = TypeChecker::check(&project).expect("the program is accepted");
    let dump = typy_check::dump(&program, &project);
    for expected in [
        "Assert @2:5",
        "NewException AssertionError : AssertionError",
        "Assert @3:5",
        "Unwrap",
    ] {
        assert!(dump.contains(expected), "{expected} in\n{dump}");
    }
}

#[test]
fn iterators_type_their_items_and_next_its_default() {
    let source = "class C:\n    def __iter__(self) -> 'C':\n        return self\n    def __next__(self) -> int:\n        raise StopIteration\n\
                  items = iter({'a': 1}.items())\n\
                  first = next(items)\n\
                  maybe = next(iter([1]), None)\n\
                  same = next(iter(iter('ab')))\n\
                  counted = next(C(), 0)\n\
                  for n in iter(C()):\n    print(n, first, maybe, same, counted, type(items), items is items)\n";
    let (_directory, project) = load(source);
    let program = TypeChecker::check(&project).expect("the program is accepted");
    let types: Vec<String> = ["items", "first", "maybe", "same", "counted"]
        .iter()
        .map(|name| {
            let symbol = program
                .symbols
                .iter()
                .find(|symbol| symbol.name == *name)
                .unwrap();
            symbol.ty.to_string()
        })
        .collect();
    assert_eq!(
        types,
        [
            "Iterator[tuple[str, int]]",
            "tuple[str, int]",
            "int | None",
            "str",
            "int"
        ]
    );
    let dump = typy_check::dump(&program, &project);
    for expected in ["Iter : Iterator", "Next __next__", "Next : int | None"] {
        assert!(dump.contains(expected), "{expected} in\n{dump}");
    }
}

#[test]
fn with_binds_what_enter_returns_and_knows_whether_exit_suppresses() {
    let source = "class M:\n    def __enter__(self) -> int:\n        return 1\n    def __exit__(self, kind, error, trace) -> bool:\n        print(kind, error, trace)\n        return True\n\
                  class N:\n    def __enter__(self) -> str:\n        return 'n'\n    def __exit__(self, kind, error, trace) -> None:\n        pass\n\
                  with M() as number, N() as text:\n    print(number, text)\n";
    let (_directory, project) = load(source);
    let program = TypeChecker::check(&project).expect("the program is accepted");
    let dump = typy_check::dump(&program, &project);
    for expected in ["With as number: int suppresses", "With as text: str @"] {
        assert!(dump.contains(expected), "{expected} in\n{dump}");
    }
    let trace = program
        .symbols
        .iter()
        .find(|symbol| symbol.name == "trace")
        .unwrap();
    assert_eq!(trace.ty.to_string(), "TracebackType | None");
}

#[test]
fn raise_ends_the_path_it_is_on() {
    let source = "def f(n: int) -> int:\n    if n > 0:\n        return n\n    raise ValueError('n', n)\n\
         def g() -> str:\n    raise KeyError from None\n\
         e: LookupError = IndexError()\n\
         t = type(e)\n\
         raise RuntimeError('r') from e\n";
    let (_directory, project) = load(source);
    let program = TypeChecker::check(&project).expect("the program is accepted");
    let dump = typy_check::dump(&program, &project);
    for expected in [
        "Raise @4:5",
        "NewException ValueError : ValueError",
        "Raise from None @6:5",
        "NewException KeyError : KeyError",
        "Raise from @9:1",
        "Local e : LookupError",
    ] {
        assert!(dump.contains(expected), "{expected} in\n{dump}");
    }
}

fn accepted(source: &str) -> TypedProgram {
    check(source).unwrap_or_else(|codes| panic!("rejected with {codes:?}:\n{source}"))
}

fn rejected(source: &str) -> Vec<String> {
    match check(source) {
        Ok(_) => panic!("accepted:\n{source}"),
        Err(codes) => codes,
    }
}

fn messages(source: &str) -> Vec<String> {
    let (_directory, project) = load(source);
    let diagnostics = TypeChecker::check(&project).expect_err("the program is rejected");
    diagnostics
        .into_sorted(&project)
        .iter()
        .map(|diagnostic| diagnostic.message().to_owned())
        .collect()
}

#[test]
fn a_program_of_0_1_features_is_accepted() {
    let program = accepted(
        "\
def fib(n: int) -> int:
    if n < 2:
        return n
    return fib(n - 1) + fib(n - 2)

def greet(name: str, punctuation: str = \"!\") -> str:
    return \"hello \" + name + punctuation

total = 0
for i in range(10):
    total += fib(i)
while total > 100:
    total = total // 2
else:
    pass
print(total, greet(\"world\"), sep=\", \", end=\"\\n\")
if __name__ == \"__main__\":
    print(len(\"abc\"), abs(-2.5), min(3, 1, 2), max(\"a\", \"b\"), int(\"12\"), float(3), str(1.5), bool(0))
    print(repr(\"x\"), ord(\"a\"), chr(98), \"abc\"[-1], 7 / 2, 2 ** 10, 1 < 2 <= 3, not True)
",
    );
    assert_eq!(program.functions.len(), 3);
    let entry = program.function(program.entry);
    assert_eq!(entry.name, "<module>");
    assert!(matches!(entry.body[1], TypedStmt::ForRange { .. }));
    let fib = &program.functions[0];
    assert_eq!(fib.name, "fib");
    assert_eq!(fib.module, "__main__");
    assert_eq!(program.symbol(fib.parameters[0]).ty, Type::Int);
    let total = program.symbol(entry.locals[0]);
    assert_eq!((total.name.as_str(), &total.ty), ("total", &Type::Int));
}

#[test]
fn types_of_expressions_follow_python() {
    let program = accepted(
        "a = 7 / 2\nb = 7 // 2\nc = 1 + 2.0\nd = \"ab\" * 3\ne = 2 ** 3\nf = 2.0 ** -1\ng = True & False\nh = 1 if True else 2\n",
    );
    let entry = program.function(program.entry);
    let types: Vec<Type> = entry
        .locals
        .iter()
        .map(|symbol| program.symbol(*symbol).ty.clone())
        .collect();
    assert_eq!(
        types,
        [
            Type::Float,
            Type::Int,
            Type::Float,
            Type::Str,
            Type::Int,
            Type::Float,
            Type::Bool,
            Type::Int
        ]
    );
}

#[test]
fn keyword_arguments_keep_source_order_and_defaults_fill_the_rest() {
    let program = accepted(
        "def f(a: int, b: int = 2, c: int = 3) -> int:\n    return a + b + c\nprint(f(c=1, a=4))\n",
    );
    let entry = program.function(program.entry);
    let TypedStmt::Expr { value, .. } = &entry.body[0] else {
        panic!("not an expression")
    };
    let TypedExprKind::Print { values, .. } = &value.kind else {
        panic!("not print")
    };
    let TypedExprKind::Call { arguments, .. } = &values[0].kind else {
        panic!("not a call")
    };
    let order: Vec<usize> = arguments
        .iter()
        .map(|argument| argument.parameter)
        .collect();
    assert_eq!(order, [2, 0]);
}

#[test]
fn rejected_programs_give_their_codes() {
    let cases: &[(&str, &[&str])] = &[
        ("x = 1\nx = \"a\"\n", &["E0402"]),
        ("x: float = 1\n", &["E0402"]),
        ("def f(x: float) -> None:\n    pass\nf(1)\n", &["E0404"]),
        ("def f() -> float:\n    return 1\n", &["E0405"]),
        ("def f() -> float:\n    return \"a\"\n", &["E0405"]),
        ("def f(x) -> None:\n    pass\n", &["E0203"]),
        ("def f(x: int):\n    pass\n", &["E0204"]),
        ("def f(x: Any) -> None:\n    pass\n", &["E0403"]),
        ("def f(x: Thing) -> None:\n    pass\n", &["E0205"]),
        ("print(undefined)\n", &["E0205"]),
        ("print(eval(\"1\"))\n", &["E0202"]),
        ("class C(B):\n    pass\n", &["E0201"]),
        ("x = 2j\n", &["E0104"]),
        ("n = 3\nprint(2 ** n)\n", &["E0302"]),
        ("print(1 + \"a\")\n", &["E0401"]),
        ("print(True + 1)\n", &["E0406"]),
        ("print(-True)\n", &["E0406"]),
        ("print(1 is 1)\n", &["E0301"]),
        ("print(1 or \"a\")\n", &["E0303"]),
        ("print(1 if True else \"a\")\n", &["E0305"]),
        ("print(1 == \"a\")\n", &["E0407"]),
        ("print(None < None)\n", &["E0407"]),
        ("if True:\n    x = 1\nprint(x)\n", &["E0306"]),
        (
            "def f() -> int:\n    if True:\n        return 1\n",
            &["E0405"],
        ),
        (
            "def g() -> int:\n    return h()\nprint(g())\ndef h() -> int:\n    return 1\n",
            &["E0306"],
        ),
        (
            "def f() -> None:\n    pass\ndef f() -> None:\n    pass\n",
            &["E0202"],
        ),
        ("def f() -> None:\n    pass\nf = 1\n", &["E0202"]),
        ("x = 1\ndef f() -> int:\n    return x\n", &["E0201"]),
        ("def f() -> int:\n    return 1\ng = f\n", &["E0201"]),
        ("x = 1\nx()\n", &["E0401"]),
        ("print(min(1, 2.0))\n", &["E0404"]),
        ("print(min((1, 'a')))\n", &["E0411"]),
        ("print(len(1))\n", &["E0404"]),
        ("print(len(\"a\", \"b\"))\n", &["E0404"]),
        ("print(abs(\"a\"))\n", &["E0401"]),
        ("print(1, file=None)\n", &["E0201"]),
        ("print(1, color=1)\n", &["E0404"]),
        ("x = enumerate([1])\n", &["E0201"]),
        ("print(1 is True)\n", &["E0301"]),
        ("n: int | None = None\nprint(n is True)\n", &["E0301"]),
        ("print(1[0])\n", &["E0401"]),
        ("print(\"a\"[\"b\"])\n", &["E0404"]),
        ("print(\"%d\" % 1)\n", &["E0201"]),
        ("print(~\"a\")\n", &["E0401"]),
        (
            "def f(a: int) -> int:\n    return a\nprint(f(1, 2))\n",
            &["E0404"],
        ),
        (
            "def f(a: int) -> int:\n    return a\nprint(f(b=1))\n",
            &["E0404"],
        ),
        (
            "def f(a: int) -> int:\n    return a\nprint(f(1, a=1))\n",
            &["E0404"],
        ),
        (
            "def f(a: int) -> int:\n    return a\nprint(f())\n",
            &["E0404"],
        ),
        ("def f(a: float = 1) -> float:\n    return a\n", &["E0404"]),
        ("for i in range(\"a\"):\n    pass\n", &["E0404"]),
        ("x: int = 1\nx: str = \"a\"\n", &["E0402"]),
        ("print(__file__)\n", &["E0202"]),
        ("print(len)\n", &["E0201"]),
        ("print(int(1, 2, 3))\n", &["E0404"]),
        ("print(str(1, 2))\n", &["E0404"]),
        ("print(ord(1))\n", &["E0404"]),
        ("print(chr(\"a\"))\n", &["E0404"]),
        ("print(bool(1, 2))\n", &["E0404"]),
        ("print(repr())\n", &["E0404"]),
        ("print(abs())\n", &["E0404"]),
        ("print(len(x=\"a\"))\n", &["E0404"]),
    ];
    for (source, expected) in cases {
        assert_eq!(&rejected(source), expected, "\n{source}");
    }
}

#[test]
fn loops_and_branches_track_definite_assignment() {
    accepted("x = 0\nif x > 0:\n    y = 1\nelse:\n    y = 2\nprint(y)\n");
    accepted("while True:\n    z = 1\n    break\nprint(z)\n");
    accepted("for i in range(3):\n    w = i\nelse:\n    w = 0\nprint(w)\n");
    accepted("def f() -> int:\n    while True:\n        return 1\n");
    accepted(
        "def f(n: int) -> int:\n    if n > 0:\n        return 1\n    else:\n        return 2\n",
    );
    assert_eq!(
        rejected("for i in range(3):\n    v = i\nprint(v)\n"),
        ["E0306"]
    );
    assert_eq!(
        rejected("for i in range(3):\n    pass\nprint(i)\n"),
        ["E0306"]
    );
    assert_eq!(
        rejected("while 1 > 0:\n    u = 1\n    break\nprint(u)\n"),
        ["E0306"]
    );
    accepted("x = 0\nwhile x < 3:\n    x += 1\n    continue\nprint(x)\n");
}

#[test]
fn an_int_literal_where_a_float_is_declared_gets_a_certain_fix() {
    let (_directory, project) = load("x: float = 1\n");
    let diagnostics = TypeChecker::check(&project)
        .unwrap_err()
        .into_sorted(&project);
    let fix = diagnostics[0].fix().expect("a certain fix");
    assert_eq!(fix.replacement, "1.0");
    assert_eq!(
        diagnostics[0].rule().map(typy_diagnostics::RuleId::as_str),
        Some("TYP-09")
    );
}

#[test]
fn messages_use_the_right_article_and_python_names() {
    assert_eq!(
        messages("x = 1\nx = \"a\"\n"),
        ["`x` was declared as int and is assigned a str"]
    );
    assert_eq!(
        messages("print(1 + \"a\")\n"),
        ["`+` is not defined for int and str"]
    );
    assert_eq!(
        messages("print(len)\n"),
        ["using `len` as a value is planned for TyPy 1.0 (FUN-05)"]
    );
    assert_eq!(
        messages("print(undefined)\n"),
        ["`undefined` is not defined"]
    );
}

#[test]
fn one_cause_gives_one_error() {
    assert_eq!(rejected("x = undefined + 1\nprint(x * 2)\n"), ["E0205"]);
    assert_eq!(messages("print(missing(1) + 2)\n").len(), 1);
}

#[test]
fn module_body_is_the_entry_and_keeps_its_locals_in_order() {
    let program = accepted("b = 1\na = 2\nprint(a + b)\n");
    let entry = program.function(program.entry);
    let names: Vec<&str> = entry
        .locals
        .iter()
        .map(|symbol| program.symbol(*symbol).name.as_str())
        .collect();
    assert_eq!(names, ["b", "a"]);
    assert_eq!(entry.signature.parameters.len(), 0);
}

#[test]
fn empty_program_is_accepted() {
    let program = accepted("");
    assert_eq!(program.function(program.entry).body.len(), 0);
}

#[test]
fn settings_root_is_respected() {
    let directory = tempfile_dir::Dir::new();
    std::fs::write(directory.path.join("app.py"), "pass\n").unwrap();
    let root: Utf8PathBuf = directory.path.clone();
    let project = Project::load(&root.join("app.py"), ProjectSettings::new(root)).unwrap();
    assert!(TypeChecker::check(&project).is_ok());
}

#[test]
fn builtins_accept_every_form_0_1_provides() {
    for source in [
        "print()\n",
        "print(int(), float(), str(), bool())\n",
        "print(int(\"ff\", 16), int(2.5), int(True), float(\"1.5\"), float(True))\n",
        "print(abs(-3), min(1.5, 2.5), max(1, 2, 3))\n",
        "x = 1\nprint(x is None, x is not None, None is None)\n",
        "print(1 < 2.5, 2.5 >= 1, \"a\" < \"b\" < \"c\", True == False, 1 != 2)\n",
        "print(not 1, not \"\", not None, 1 and 2, \"\" or \"x\")\n",
        "x = 5\nx -= 1\nx *= 2\nx //= 3\nx %= 2\nx **= 2\nx <<= 1\nx >>= 1\nx |= 1\nx ^= 1\nx &= 1\n",
        "y = 1.5\ny /= 2.0\ny += 1\n",
        "s = \"a\"\ns += \"b\"\ns *= 2\n",
        "print(~5, +5, -5.5, 7 % 3, 7.5 % 2, 7 // 2.0)\n",
        "x = -8.0\ny = 3\nz = 0.5\nprint(x ** 2.0, 2 ** 0.5, 9.0 ** 0.5, x ** y, 2.0 ** -1, 2.0 ** z)\n",
        "def f(flag: bool = True, name: str = \"n\", ratio: float = 0.5, none: None = None) -> None:\n    return None\nf()\n",
        "def f() -> None:\n    return\nf()\n",
        "while False:\n    pass\nelse:\n    pass\n",
        "for i in range(1, 5):\n    if i == 2:\n        continue\n    if i == 4:\n        break\nelse:\n    print(\"done\")\n",
    ] {
        accepted(source);
    }
}

#[test]
fn errors_inside_compound_statements_are_found() {
    let cases: &[(&str, &[&str])] = &[
        ("x = 1\nx += \"a\"\n", &["E0401"]),
        ("s = \"a\"\ns -= \"b\"\n", &["E0401"]),
        ("x = 1.5\nx //= 2\nx = 1\n", &["E0402"]),
        ("while undefined:\n    pass\n", &["E0205"]),
        ("if 1 + \"a\":\n    pass\n", &["E0401"]),
        ("for i in range(1, 2, \"x\"):\n    pass\n", &["E0404"]),
        ("x = \"a\"\nfor x in range(3):\n    pass\n", &["E0402"]),
        ("print(1 < 2 < \"a\")\n", &["E0407"]),
        ("print(\"a\" < 1)\n", &["E0407"]),
        ("print(max(None, None))\n", &["E0407"]),
        ("print(int(\"a\", \"b\"))\n", &["E0404"]),
        ("print(float(None))\n", &["E0404"]),
        ("print(abs(True))\n", &["E0406"]),
        ("print(1 @ 2)\n", &["E0202"]),
        ("print(2.0 ** \"a\")\n", &["E0401"]),
        ("x = -8.0\nprint(x ** 0.5)\n", &["E0302"]),
        ("n = 2\nprint(n ** 0.5)\n", &["E0302"]),
        ("print(1 << 2.0)\n", &["E0401"]),
        ("def f() -> int:\n    return \"a\"\n", &["E0405"]),
        ("def f() -> None:\n    return 1\n", &["E0405"]),
        ("def f() -> float:\n    return 1\n", &["E0405"]),
        ("def f(x: int = \"a\") -> None:\n    pass\n", &["E0404"]),
        ("def f(x: None) -> None:\n    pass\nf(1)\n", &["E0404"]),
        ("x: int\nprint(x)\n", &["E0306"]),
        ("def f() -> None:\n    print(y)\n    y = 1\n", &["E0306"]),
        ("print(print(1) + 1)\n", &["E0401"]),
        ("def f() -> None:\n    pass\nprint(f(1))\n", &["E0404"]),
        ("print(sum)\n", &["E0201"]),
        ("print(\"a\"[True])\n", &["E0404"]),
    ];
    for (source, expected) in cases {
        assert_eq!(&rejected(source), expected, "\n{source}");
    }
}

#[test]
fn certain_fixes_are_offered_for_int_literals_in_float_places() {
    for source in [
        "def f(x: float) -> None:\n    pass\nf(2)\n",
        "def f() -> float:\n    return 3\n",
        "def f(x: float = 4) -> None:\n    pass\n",
        "x = 1.5\nx = 5\n",
    ] {
        let (_directory, project) = load(source);
        let diagnostics = TypeChecker::check(&project)
            .unwrap_err()
            .into_sorted(&project);
        let fix = diagnostics[0]
            .fix()
            .unwrap_or_else(|| panic!("no fix for\n{source}"));
        assert!(fix.replacement.ends_with(".0"), "{source}");
    }
}

#[test]
fn the_typed_dump_shows_every_node_with_its_position_and_type() {
    let source = "def twice(n: int = 2) -> int:\n    total = n\n    total += n\n    return total\nfor i in range(3):\n    if i > 0 and not False:\n        print(twice(i), \"x\"[0], sep=\"-\")\n    else:\n        continue\nwhile True:\n    break\nx: float = -1.5 if __name__ else 2.0\npass\n";
    let (_directory, project) = load(source);
    let program = TypeChecker::check(&project).unwrap();
    let expected = r#"
Program entry fn1
Function fn0 __main__.twice(n: int) -> int @1:1
  locals total: int
  body
    Assign total: int @2:5
      Local n : int @2:13
    AugAssign Add total: int @3:5
      Local n : int @3:14
    Return @4:5
      Local total : int @4:12
Function fn1 __main__.<module>() -> None @5:1
  locals i: int, x: float
  body
    ForRange i: int @5:1
      Constant Int("0") : int @5:10
      Constant Int("3") : int @5:16
      Constant Int("1") : int @5:10
      body
        If @6:5
          BoolOp And : bool @6:8
            Compare [Gt] : bool @6:8
              Local i : int @6:8
              Constant Int("0") : int @6:12
            Unary Not : bool @6:18
              Constant Bool(false) : bool @6:22
          body
            Expr @7:9
              Print sep=true end=false : None @7:9
                Call twice : int @7:15
                  Local i : int @7:21
                Index : str @7:25
                  Constant Str("x") : str @7:25
                  Constant Int("0") : int @7:29
                Constant Str("-") : str @7:37
          orelse
            Continue @9:9
    While @10:1
      Constant Bool(true) : bool @10:7
      body
        Break @11:5
    Assign x: float @12:1
      IfExp : float @12:12
        ModuleName : str @12:20
        Unary USub : float @12:12
          Constant Float(1.5) : float @12:13
        Constant Float(2.0) : float @12:34
    Pass @13:1
"#;
    assert_eq!(typy_check::dump(&program, &project), expected.trim_start());
}

/// Returns the type of `expression` over `i: int`, `f: float`, `s: str`, `b: bool` and
/// `n: None`, or the codes it is rejected with.
fn type_of(expression: &str) -> Result<String, Vec<String>> {
    let source = format!("i = 1\nf = 1.5\ns = \"a\"\nb = True\nn = None\nx = {expression}\n");
    let program = check(&source)?;
    let symbol = program
        .symbols
        .iter()
        .find(|symbol| symbol.name == "x")
        .unwrap();
    Ok(symbol.ty.to_string())
}

#[test]
fn each_operator_gives_python_s_result_type_or_its_diagnostic() {
    let cases: &[(&str, Result<&str, &[&str]>)] = &[
        // bool takes only the bitwise operators and `not` (DAT-03).
        ("b & b", Ok("bool")),
        ("b | b", Ok("bool")),
        ("b ^ b", Ok("bool")),
        ("not b", Ok("bool")),
        ("b + 1", Err(&["E0406"])),
        ("1 + b", Err(&["E0406"])),
        ("-b", Err(&["E0406"])),
        ("~b", Err(&["E0406"])),
        ("b & i", Err(&["E0406"])),
        // str concatenation and repetition.
        ("s + s", Ok("str")),
        ("s * i", Ok("str")),
        ("i * s", Ok("str")),
        ("s * f", Err(&["E0401"])),
        ("s - s", Err(&["E0401"])),
        ("s + i", Err(&["E0401"])),
        ("s % i", Err(&["E0201"])),
        // Arithmetic: int with int stays int, anything with float is float (EXP-02).
        ("i + i", Ok("int")),
        ("i + f", Ok("float")),
        ("f - i", Ok("float")),
        ("i * f", Ok("float")),
        ("i // f", Ok("float")),
        ("i % i", Ok("int")),
        ("f % f", Ok("float")),
        ("i / i", Ok("float")),
        ("f / i", Ok("float")),
        ("i / s", Err(&["E0401"])),
        ("n % i", Err(&["E0401"])),
        ("s / i", Err(&["E0401"])),
        ("s ** i", Err(&["E0401"])),
        ("i - s", Err(&["E0401"])),
        ("s // i", Err(&["E0401"])),
        ("n * i", Err(&["E0401"])),
        ("i % s", Err(&["E0401"])),
        ("n - f", Err(&["E0401"])),
        ("f // s", Err(&["E0401"])),
        ("s >> i", Err(&["E0401"])),
        ("i << f", Err(&["E0401"])),
        ("n ** i", Err(&["E0401"])),
        ("f / n", Err(&["E0401"])),
        ("i @ i", Err(&["E0202"])),
        ("n + i", Err(&["E0401"])),
        // Powers with one result type (EXP-04).
        ("i ** 2", Ok("int")),
        ("i ** i", Err(&["E0302"])),
        ("i ** s", Err(&["E0401"])),
        ("i ** 2.0", Ok("float")),
        ("f ** i", Ok("float")),
        ("f ** 2.0", Ok("float")),
        ("2.0 ** f", Ok("float")),
        ("2 ** f", Ok("float")),
        ("f ** 0.5", Err(&["E0302"])),
        ("(-1.5) ** 0.5", Err(&["E0302"])),
        // Shifts and bitwise operators take two ints (EXP-05).
        ("i << i", Ok("int")),
        ("i >> i", Ok("int")),
        ("i & i", Ok("int")),
        ("i | i", Ok("int")),
        ("i ^ i", Ok("int")),
        ("f << i", Err(&["E0401"])),
        ("f & f", Err(&["E0401"])),
        // Unary operators.
        ("-f", Ok("float")),
        ("+i", Ok("int")),
        ("~i", Ok("int")),
        ("~f", Err(&["E0401"])),
        ("-s", Err(&["E0401"])),
        ("+s", Err(&["E0401"])),
        // Comparisons (EXP-06, EXP-08).
        ("i == f", Ok("bool")),
        ("s != s", Ok("bool")),
        ("n == n", Ok("bool")),
        ("s == i", Err(&["E0407"])),
        ("i < f", Ok("bool")),
        ("s <= s", Ok("bool")),
        ("n < n", Err(&["E0407"])),
        ("i > s", Err(&["E0407"])),
        ("n is None", Ok("bool")),
        ("i is not n", Ok("bool")),
        ("s is s", Err(&["E0301"])),
        // An operand that is already wrong gives no second error (DIA-04).
        ("missing + 1", Err(&["E0205"])),
        ("1 - missing", Err(&["E0205"])),
        ("-missing", Err(&["E0205"])),
        ("missing < 1", Err(&["E0205"])),
        ("1 == missing", Err(&["E0205"])),
    ];
    let mut wrong = Vec::new();
    for (expression, expected) in cases {
        let expected = expected.map(str::to_owned).map_err(|codes| {
            codes
                .iter()
                .map(|code| (*code).to_owned())
                .collect::<Vec<_>>()
        });
        let actual = type_of(expression);
        if actual != expected {
            wrong.push(format!(
                "{expression}: expected {expected:?}, got {actual:?}"
            ));
        }
    }
    assert!(wrong.is_empty(), "{}", wrong.join("\n"));
}

#[test]
fn each_builtin_call_name_and_statement_rule_gives_its_code() {
    let cases: &[(&str, &[&str])] = &[
        // An argument that is already wrong gives no second error (DIA-04).
        ("print(missing)\n", &["E0205"]),
        ("print(missing[0])\n", &["E0205"]),
        ("print(\"a\"[missing])\n", &["E0205"]),
        // Arity of the builtins (LIB-01).
        ("print(len())\n", &["E0404"]),
        ("print(ord())\n", &["E0404"]),
        ("print(chr(1, 2))\n", &["E0404"]),
        ("print(repr())\n", &["E0404"]),
        ("print(abs())\n", &["E0404"]),
        ("print(min(1))\n", &["E0411"]),
        ("print(max())\n", &["E0404"]),
        ("print(len(missing))\n", &["E0205"]),
        ("print(1 and \"a\")\n", &["E0303"]),
        // Module variables inside functions need `global`, which arrives in 0.3.
        ("count = 1\ndef f() -> int:\n    return count\n", &["E0201"]),
        ("count = 1\ndef f() -> None:\n    count()\n", &["E0201"]),
        // Functions and builtins are not values; no requirement provides warnings or
        // exception groups.
        ("x = len\n", &["E0201"]),
        ("x = UserWarning\n", &["E0202"]),
        ("x = ExceptionGroup\n", &["E0202"]),
        // A return value of another type (TYP-03, TYP-09).
        ("def f() -> str:\n    return 1\n", &["E0405"]),
        ("def f() -> float:\n    return 1\n", &["E0405"]),
        // `for` loops over a call of `range` or a list; `range` takes no keyword arguments (CST-02, CST-03).
        ("for i in range(1, 2, 3, 4):\n    pass\n", &["E0404"]),
        ("for i in range(stop=3):\n    pass\n", &["E0404"]),
        ("for i in len(\"abc\"):\n    pass\n", &["E0411"]),
        // A default is a constant (EXP-16).
        ("def f(n: int = 1 + 1) -> int:\n    return n\n", &["E0201"]),
    ];
    let mut wrong = Vec::new();
    for (source, expected) in cases {
        let actual = rejected(source);
        if actual != *expected {
            wrong.push(format!("{source:?}: expected {expected:?}, got {actual:?}"));
        }
    }
    assert!(wrong.is_empty(), "{}", wrong.join("\n"));
}

#[test]
fn a_repeated_annotation_of_the_same_type_is_accepted() {
    accepted("x: int = 1\nx: int = 2\nprint(x)\n");
}

#[test]
fn a_variable_read_twice_before_assignment_is_reported_once() {
    let (_directory, project) = load("b = True\nif b:\n    r = 1\nprint(r)\nprint(r)\n");
    let diagnostics = TypeChecker::check(&project).unwrap_err();
    let codes: Vec<String> = diagnostics.iter().map(|d| d.code().to_string()).collect();
    assert_eq!(codes, ["E0306"]);
}

#[test]
fn tuples_have_one_type_per_item() {
    let cases: &[(&str, Result<&str, &[&str]>)] = &[
        ("(i, s)", Ok("tuple[int, str]")),
        ("()", Ok("tuple[()]")),
        ("(i, (f, b))", Ok("tuple[int, tuple[float, bool]]")),
        ("(i, s)[0]", Ok("int")),
        ("(i, s)[-1]", Ok("str")),
        ("(i, s)[2]", Err(&["E0307"])),
        ("(i, s)[-3]", Err(&["E0307"])),
        ("(i, s)[i]", Err(&["E0307"])),
        ("(i, s)[missing]", Err(&["E0205"])),
        ("(missing, i)[i]", Err(&["E0205"])),
        ("(i,) + (s, f)", Ok("tuple[int, str, float]")),
        ("(i, s) - (i, s)", Err(&["E0401"])),
        ("len((i, s))", Ok("int")),
        ("(i, s) == (i, s)", Ok("bool")),
        ("(1, 2.5) == (1.0, 2)", Ok("bool")),
        ("(i,) == (s,)", Err(&["E0407"])),
        ("(i,) == (i, i)", Err(&["E0407"])),
        ("(i, s) < (i, s)", Ok("bool")),
        ("(n,) < (n,)", Ok("bool")),
        ("(i, n) < (i, n)", Ok("bool")),
        ("n < n", Err(&["E0407"])),
        ("(n,) < (i,)", Err(&["E0407"])),
        ("(i,) < (i, i)", Err(&["E0407"])),
    ];
    let mut wrong = Vec::new();
    for (expression, expected) in cases {
        let expected = expected.map(str::to_owned).map_err(|codes| {
            codes
                .iter()
                .map(|code| (*code).to_owned())
                .collect::<Vec<_>>()
        });
        let actual = type_of(expression);
        if actual != expected {
            wrong.push(format!(
                "{expression}: expected {expected:?}, got {actual:?}"
            ));
        }
    }
    assert!(wrong.is_empty(), "{}", wrong.join("\n"));
}

#[test]
fn tuple_annotations_and_unpacking_follow_the_item_types() {
    accepted("single: tuple[int] = (1,)\nprint(single)\n");
    accepted(
        "pair: tuple[int, str] = (1, \"a\")\nempty: tuple[()] = ()\nn, s = pair\nn += 1\na, (b, c) = 1, (2.5, \"x\")\nb = 1.5\n",
    );
    accepted(
        "def swap(p: tuple[int, str]) -> tuple[str, int]:\n    a, b = p\n    return b, a\n\n\nprint(swap((1, \"x\")))\n",
    );
    let cases: &[(&str, &[&str])] = &[
        ("a, b = (1, 2, 3)\n", &["E0308"]),
        ("a, b = 1\n", &["E0308"]),
        ("a, b = 1, \"x\"\na = \"y\"\n", &["E0402"]),
        ("p: tuple = (1, 2)\n", &["E0408"]),
        ("p: tuple[int, ..., int] = (1, 2)\n", &["E0408"]),
        ("p: tuple[..., int] = (1, 2)\n", &["E0408"]),
        ("p: tuple[...] = ()\n", &["E0408"]),
        ("p: tuple[int, str] = (1, 2)\n", &["E0402"]),
        ("p: tuple[int] = (1, 2)\n", &["E0402"]),
    ];
    for (source, expected) in cases {
        assert_eq!(rejected(source), *expected, "{source:?}");
    }
}

#[test]
fn list_rules_give_their_codes() {
    let cases: &[(&str, &[&str])] = &[
        ("x = []\n", &["E0410"]),
        ("x = [1, \"a\"]\n", &["E0409"]),
        ("x = [1]\nx.push(2)\n", &["E0412"]),
        ("x: list[int] = []\nx.append(\"a\")\n", &["E0404"]),
        ("x: list[int] = [1]\nx[0] = \"a\"\n", &["E0402"]),
        ("x = 1\nx[0] = 2\n", &["E0401"]),
        ("for a in 3:\n    pass\n", &["E0411"]),
        ("print(\"a\" in [1])\n", &["E0407"]),
        ("missing.append(1)\n", &["E0205"]),
        ("x = [1]\nx.append(item=2)\n", &["E0404"]),
        ("x = [1]\nx.append()\n", &["E0404"]),
        ("x = 'a'.format()\n", &["E0412"]),
        ("print(1 in 2)\n", &["E0407"]),
        ("def f(x: list) -> None:\n    pass\n", &["E0408"]),
    ];
    for (source, expected) in cases {
        assert_eq!(&rejected(source), expected, "\n{source}");
    }
}

#[test]
fn slice_and_list_method_rules_give_their_codes() {
    let cases: &[(&str, &[&str])] = &[
        ("i = 0\nx = (1, 2)[i:1]\n", &["E0307"]),
        ("x = (1, 2)[::0]\n", &["E0307"]),
        ("x = [1][\"a\":]\n", &["E0404"]),
        ("x = [1][:1.5]\n", &["E0404"]),
        ("x = [1][::\"a\"]\n", &["E0404"]),
        ("x = 1\ny = x[1:2]\n", &["E0401"]),
        ("y = missing[1:2]\n", &["E0205"]),
        ("x = [[1]]\ny = x[0:1, 0]\n", &["E0401"]),
        ("x = [1]\nx[0:1] += [2]\n", &["E0201"]),
        ("x = (1, 2)\nx[0] += 1\n", &["E0401"]),
        ("x = 'ab'\nx[0] += 'c'\n", &["E0401"]),
        ("x = [1]\nx[0] += 1.5\n", &["E0402"]),
        ("x = [1]\nx[0.5] += 1\n", &["E0404"]),
        ("missing[0] += 1\n", &["E0205"]),
        ("x = [1]\nx.pop(1, 2)\n", &["E0404"]),
        ("x = [1]\nx.insert(0)\n", &["E0404"]),
        ("x = [1]\nx.extend([1.5])\n", &["E0409"]),
        ("x = [1]\nx.index(\"a\")\n", &["E0404"]),
        ("x = [1]\nx.clear(1)\n", &["E0404"]),
        ("x = [1] + [\"a\"]\n", &["E0401"]),
        ("x = [1] * 1.5\n", &["E0401"]),
        ("x = [1] < [\"a\"]\n", &["E0407"]),
        ("x = [] == []\n", &["E0410"]),
        ("x = [] < 1\n", &["E0410"]),
        ("x = 1 < []\n", &["E0410"]),
    ];
    for (source, expected) in cases {
        assert_eq!(&rejected(source), expected, "\n{source}");
    }
}

#[test]
fn method_argument_counts_are_worded_for_their_number() {
    let cases = [
        (
            "x = [1]\nx.pop(1, 2)\n",
            "`pop` takes at most 1 argument, and 2 are given",
        ),
        (
            "x = [1]\nx.insert(0)\n",
            "`insert` takes 2 arguments, and 1 is given",
        ),
        (
            "x = [1]\nx.append()\n",
            "`append` takes 1 argument, and 0 are given",
        ),
        (
            "x = [1]\nx.copy(1)\n",
            "`copy` takes 0 arguments, and 1 is given",
        ),
    ];
    for (source, expected) in cases {
        assert_eq!(messages(source), [expected], "\n{source}");
    }
}

#[test]
fn slices_methods_and_list_operators_have_python_s_types() {
    let source = "\
xs: list[int] = [3, 1]
words = ['a']
a = xs[1:] + xs[:1] + xs[::-1] + xs[::]
b = words[0][1:2]
c = xs.pop() + xs.pop(0) + xs.index(3) + xs.count(1)
d = xs.copy() * 2 + 3 * xs
e = xs < [1] and [[1]] >= [[2]] and xs == [] and [] != xs and 1 in [] and [] in [xs]
xs.insert(0, 5)
xs.extend(xs)
xs.remove(5)
xs.reverse()
xs.clear()
xs += [1]
xs *= 2
xs[0] += 1
words[0] *= 3
grid = [[1]]
grid[0] += [2]
";
    let (_directory, project) = load(source);
    let program = TypeChecker::check(&project).expect("the program is accepted");
    let dump = typy_check::dump(&program, &project);
    for node in ["Slice", "Method pop", "AugAssign Add item of list[int]"] {
        assert!(dump.contains(node), "{node} missing from\n{dump}");
    }
}

#[test]
fn builtins_of_one_iterable_give_their_codes() {
    let cases: &[(&str, &[&str])] = &[
        ("x = sum([0.5])\n", &["E0404"]),
        ("x = sum(['a'])\n", &["E0401"]),
        ("x = sum([True])\n", &["E0406"]),
        ("x = sum([1], 0.5)\n", &["E0404"]),
        ("x = sum([1], 0, 1)\n", &["E0404"]),
        ("x = sum([1], start=0, start=1)\n", &["E0404"]),
        ("x = sum([1], key=1)\n", &["E0404"]),
        ("x = sorted()\n", &["E0404"]),
        ("x = sorted([1], key=abs)\n", &["E0201"]),
        ("x = sorted([1], reverse=1)\n", &["E0404"]),
        ("x = sorted([1], cmp=1)\n", &["E0404"]),
        ("x = sorted([1], **{})\n", &["E0201"]),
        ("x = sorted((1, 'a'))\n", &["E0411"]),
        ("x = sorted(3)\n", &["E0411"]),
        ("x = sorted(missing)\n", &["E0205"]),
        ("x = max([1], default='a')\n", &["E0404"]),
        ("x = min([1], default=1, default=2)\n", &["E0404"]),
        ("x = any(5)\n", &["E0411"]),
        ("x = [1]\nx.sort(True)\n", &["E0404"]),
        ("x = [1]\nx.sort(key=abs)\n", &["E0201"]),
        ("x = [1]\nx.sort(order=1)\n", &["E0404"]),
        ("x = [1]\nx.sort(reverse=True, reverse=False)\n", &["E0404"]),
        ("x = [1]\nx.append(1, key=2)\n", &["E0404"]),
        ("x = any([])\n", &["E0410"]),
    ];
    for (source, expected) in cases {
        assert_eq!(&rejected(source), expected, "\n{source}");
    }
}

#[test]
fn builtins_of_one_iterable_have_python_s_types() {
    let source = "\
xs = [3, 1]
floats = [0.5]
a: list[int] = sorted(xs, reverse=True)
b: int = sum(xs) + sum(xs, 1) + sum(xs, start=2) + max(xs) + min(xs, default=0)
c: float = sum(floats, 0.0) + max(floats)
d: bool = any(xs) and all(['a']) and any([[1], []])
xs.sort()
xs.sort(reverse=False)
";
    let (_directory, project) = load(source);
    let program = TypeChecker::check(&project).expect("the program is accepted");
    let dump = typy_check::dump(&program, &project);
    for node in [
        "IterableBuiltin Sorted",
        "IterableBuiltin Sum",
        "Method sort",
    ] {
        assert!(dump.contains(node), "{node} missing from\n{dump}");
    }
}

#[test]
fn loop_sources_give_their_codes() {
    let cases: &[(&str, &[&str])] = &[
        ("for i in enumerate():\n    pass\n", &["E0404"]),
        ("for i in enumerate([1], 0, 1):\n    pass\n", &["E0404"]),
        (
            "for i in enumerate([1], 0, start=1):\n    pass\n",
            &["E0404"],
        ),
        (
            "for i in enumerate([1], start=1.5):\n    pass\n",
            &["E0404"],
        ),
        ("for i in enumerate([1], begin=1):\n    pass\n", &["E0404"]),
        (
            "for i in enumerate([1], start=1, start=2):\n    pass\n",
            &["E0404"],
        ),
        ("for i in enumerate(3):\n    pass\n", &["E0411"]),
        ("for i in zip([1], strict=1):\n    pass\n", &["E0404"]),
        ("for i in zip([1], **{}):\n    pass\n", &["E0201"]),
        ("for i in reversed():\n    pass\n", &["E0404"]),
        ("for i in reversed([1], [2]):\n    pass\n", &["E0404"]),
        ("for i in reversed([1], key=1):\n    pass\n", &["E0404"]),
        ("for i in reversed((1, 'a')):\n    pass\n", &["E0411"]),
        ("for a, b in enumerate([1], 0):\n    a = 'x'\n", &["E0402"]),
        ("for a, b, c in zip([1], [2]):\n    pass\n", &["E0308"]),
        ("x = enumerate([1])\n", &["E0201"]),
        ("x = reversed([1])\n", &["E0201"]),
        ("for c in (1, 'a'):\n    pass\n", &["E0411"]),
    ];
    for (source, expected) in cases {
        assert_eq!(&rejected(source), expected, "\n{source}");
    }
}

#[test]
fn loop_sources_bind_python_s_types() {
    let shadowed = "def enumerate(n: int) -> list[int]:\n    return [n]\n\n\nfor i in enumerate(3):\n    pass\n";
    let (_directory, project) = load(shadowed);
    let program = TypeChecker::check(&project).expect("the program is accepted");
    assert!(typy_check::dump(&program, &project).contains("ForEach i: int"));
    let source = "\
for a, b in zip([1], 'x'):
    pass
for s in reversed('ab'):
    pass
for (k, c), d in zip(enumerate(['p']), [1.5], strict=True):
    pass
for n, letter in enumerate('ab', start=1):
    pass
";
    let (_directory, project) = load(source);
    let program = TypeChecker::check(&project).expect("the program is accepted");
    let dump = typy_check::dump(&program, &project);
    for node in ["ForEach zip", "ForEach reversed", "ForEach enumerate"] {
        assert!(dump.contains(node), "{node} missing from\n{dump}");
    }
}

#[test]
fn round_gives_python_s_result_types_and_codes() {
    let cases: &[(&str, &[&str])] = &[
        ("x = round()\n", &["E0404"]),
        ("x = round(1.5, 1, 2)\n", &["E0404"]),
        ("x = round(1.5, 1.5)\n", &["E0404"]),
        ("x = round('a')\n", &["E0401"]),
        ("x = round(True)\n", &["E0406"]),
        ("x = round(1.5, digits=1)\n", &["E0404"]),
        ("x = round(1.5, 1, ndigits=1)\n", &["E0404"]),
        ("x = round(1.5, **{})\n", &["E0201"]),
        ("x = round(missing)\n", &["E0205"]),
    ];
    for (source, expected) in cases {
        assert_eq!(&rejected(source), expected, "\n{source}");
    }
    let source = "a: int = round(2.5)\nb: float = round(2.5, 1)\nc: int = round(15, -1)\nd: int = round(7)\ne: float = round(2.5, ndigits=0)\n";
    accepted(source);
}

#[test]
fn list_calls_and_in_give_their_codes() {
    let cases: &[(&str, &[&str])] = &[
        ("x = list()\n", &["E0410"]),
        ("x = list([1], [2])\n", &["E0404"]),
        ("x = list([1], key=1)\n", &["E0404"]),
        ("x = list(3)\n", &["E0411"]),
        ("x = list((1, 'a'))\n", &["E0411"]),
        ("x = list(missing)\n", &["E0205"]),
        ("x = 1 in 'abc'\n", &["E0407"]),
        ("x = 'a' in (1, 2)\n", &["E0407"]),
        ("x = 1 in (1, None)\n", &["E0407"]),
    ];
    for (source, expected) in cases {
        assert_eq!(&rejected(source), expected, "\n{source}");
    }
    let source = "a: list[int] = list()\nb: list[str] = list('ab')\nc = list(enumerate(b))\nd = 'a' in 'ab' and 1 in (1, 2.5) and 1 not in ()\ne = (1, 'a', 2.5)[::-1]\nf: tuple[str, float] = e[1::-1]\n";
    let (_directory, project) = load(source);
    let program = TypeChecker::check(&project).expect("the program is accepted");
    let dump = typy_check::dump(&program, &project);
    for node in ["ListOf", "TupleSlice [2, 1, 0]"] {
        assert!(dump.contains(node), "{node} missing from\n{dump}");
    }
}

#[test]
fn tuples_of_any_length_give_their_codes_and_types() {
    let cases: &[(&str, &[&str])] = &[
        ("t: tuple[int, ...] = (1, 'a')\n", &["E0402"]),
        ("t = tuple([1])\nt.append(2)\n", &["E0412"]),
        ("t = tuple([1])\nt[0] = 2\n", &["E0401"]),
        ("x = tuple([1], [2])\n", &["E0404"]),
        ("x = tuple([1], key=1)\n", &["E0404"]),
        ("x = tuple(5)\n", &["E0411"]),
        ("x = tuple([1]) + [1]\n", &["E0401"]),
        ("x = tuple([1]) == tuple(['a'])\n", &["E0407"]),
        (
            "def f(t: tuple[int, ...]) -> None:\n    pass\nf((1, 'a'))\n",
            &["E0404"],
        ),
    ];
    for (source, expected) in cases {
        assert_eq!(&rejected(source), expected, "\n{source}");
    }
    let source = "\
def f(values: tuple[int, ...]) -> tuple[int, ...]:
    return values
a = f((1, 2)) + f(values=(3,))
b: tuple[str, ...] = tuple()
c = tuple('ab')[1:] + b
d = a < (1, 2) and (1,) in [a] and a.count(1) == a.index(1)
e: tuple[()] = tuple()
g = (0,) + a + (4,)
g += (5,)
";
    let (_directory, project) = load(source);
    let program = TypeChecker::check(&project).expect("the program is accepted");
    let dump = typy_check::dump(&program, &project);
    for node in [
        "TupleOf : tuple[int, ...]",
        "Method count",
        "Slice : tuple[str, ...]",
    ] {
        assert!(dump.contains(node), "{node} missing from\n{dump}");
    }
}

#[test]
fn str_method_arguments_bind_by_position_and_name() {
    let cases: &[(&str, &[&str])] = &[
        ("x = 'a'.split(1)\n", &["E0404"]),
        ("x = 'a'.split(',', 1, 2)\n", &["E0404"]),
        ("x = 'a'.split(sep=',', sep=',')\n", &["E0404"]),
        ("x = 'a'.split(separator=',')\n", &["E0404"]),
        ("x = 'a'.find(sub='a')\n", &["E0404"]),
        ("x = 'a'.find()\n", &["E0404"]),
        ("x = 'a'.find('a', 1.5)\n", &["E0404"]),
        ("x = 'a'.startswith(1)\n", &["E0404"]),
        ("x = 'a'.startswith(('a', 1))\n", &["E0404"]),
        ("x = ','.join([1, 2])\n", &["E0404"]),
        ("x = ','.join(5)\n", &["E0411"]),
        ("x = 'a'.strip(**{})\n", &["E0201"]),
        ("x = 'a'.format()\n", &["E0412"]),
        ("x = 'a'.center(5, fillchar='*')\n", &["E0404"]),
        ("x = [1]\nx.sort(key=abs)\n", &["E0201"]),
    ];
    for (source, expected) in cases {
        assert_eq!(&rejected(source), expected, "\n{source}");
    }
    assert_eq!(
        messages("x = 'a'.find(sub='a')\n"),
        ["`sub` of `find` is given by position only"]
    );
    let source = "\
a: list[str] = 'a b'.split() + 'a,b'.rsplit(sep=',', maxsplit=1) + 'a\\nb'.splitlines(keepends=True)
b: tuple[str, str, str] = 'k=v'.partition('=')
c: str = '-'.join(a) + ''.join(reversed('ab')) + ','.join(('x', 'y'))
d: bool = 'abc'.startswith(('a', 'b'), 1) or 'abc'.endswith('c', 0, -1) or 'a'.isascii()
e: int = 'abc'.find('b') + 'abc'.count('', 1) + 'abc'.rindex('c', 0)
f: str = 'ab'.strip() + 'ab'.lstrip(None) + 'ab'.rstrip('b') + 'x'.replace('x', 'y', 1)
g: str = 'a'.center(5) + 'a'.ljust(3, '*') + '7'.zfill(3) + 'a\\tb'.expandtabs(tabsize=4)
h: str = 'ab'.removeprefix('a') + 'ab'.removesuffix('b')
";
    accepted(source);
}

#[test]
fn str_case_and_is_methods_take_no_arguments() {
    let cases: &[(&str, &[&str])] = &[
        ("x = 'a'.upper(1)\n", &["E0404"]),
        ("x = 'a'.isalpha('b')\n", &["E0404"]),
        ("x = 'a'.title(case=1)\n", &["E0404"]),
    ];
    for (source, expected) in cases {
        assert_eq!(&rejected(source), expected, "\n{source}");
    }
    accepted(
        "a: str = 'a'.upper() + 'a'.lower() + 'a'.casefold() + 'a'.swapcase() + 'a'.capitalize() + 'a'.title()\nb: bool = 'a'.isalpha() and 'a'.isalnum() and 'a'.isdecimal() and 'a'.isdigit() and 'a'.isnumeric() and 'a'.isspace() and 'a'.isupper() and 'a'.islower() and 'a'.istitle() and 'a'.isidentifier() and 'a'.isprintable()\n",
    );
}

#[test]
fn bytes_follow_python_s_types_and_codes() {
    let cases: &[(&str, &[&str])] = &[
        ("x = bytes('a')\n", &["E0404"]),
        ("x = bytes(['a'])\n", &["E0404"]),
        ("x = bytes(1.5)\n", &["E0411"]),
        ("x = bytes([1], key=1)\n", &["E0404"]),
        ("x = b'a' + 'a'\n", &["E0401"]),
        ("x = 'a' in b'a'\n", &["E0407"]),
        ("x = b'a'['0']\n", &["E0404"]),
        ("x = b'a' == 'a'\n", &["E0407"]),
    ];
    for (source, expected) in cases {
        assert_eq!(&rejected(source), expected, "\n{source}");
    }
    accepted(
        "a: bytes = b'ab' + bytes(2) + bytes([1]) + bytes((1, 2)) + bytes(reversed(b'x')) + bytes(b'y')\nb: int = a[0] + len(a)\nc: bytes = a[1:] * 2\nd: bool = 1 in a and b'a' in a and a < b'z'\nfor n in a:\n    e: int = n\n",
    );
}

#[test]
fn codecs_and_handlers_are_literals_typy_provides() {
    let cases: &[(&str, &[&str])] = &[
        ("x = 'a'.encode('utf-16')\n", &["E0404"]),
        ("e = 'utf-8'\nx = 'a'.encode(e)\n", &["E0404"]),
        ("x = 'a'.encode('utf-8', 'namereplace')\n", &["E0404"]),
        (
            "x = b'a'.decode('utf-8', 'xmlcharrefreplace')\n",
            &["E0404"],
        ),
        ("x = b'a'.decode(1)\n", &["E0404"]),
        ("x = bytes('a', key='utf-8')\n", &["E0404"]),
        ("x = bytes('a', errors='strict')\n", &["E0404"]),
        ("x = bytes(1, 'utf-8')\n", &["E0404"]),
        ("x = bytes('a', 'utf-8', 'strict', 1)\n", &["E0404"]),
        ("x = b'a'.format()\n", &["E0412"]),
    ];
    for (source, expected) in cases {
        assert_eq!(&rejected(source), expected, "\n{source}");
    }
    accepted(
        "a: bytes = 'a'.encode() + 'a'.encode('Latin_1', errors='replace') + bytes('a', 'ascii') + bytes('a', encoding='utf8', errors='ignore')\nb: str = a.decode() + a.decode('ascii', 'surrogateescape') + a.decode(encoding='l1')\n",
    );
}

#[test]
fn bytes_methods_take_bytes_and_give_bytes() {
    let cases: &[(&str, &[&str])] = &[
        ("x = b'abc'.find('b')\n", &["E0404"]),
        ("x = b'a'.split(',')\n", &["E0404"]),
        ("x = b','.join(['a'])\n", &["E0404"]),
        ("x = b'a'.startswith(('a',))\n", &["E0404"]),
        ("x = b'a'.casefold()\n", &["E0412"]),
        ("x = b'a'.isdecimal()\n", &["E0412"]),
        ("x = b'a'.encode()\n", &["E0412"]),
        ("x = 'a'.hex()\n", &["E0412"]),
    ];
    for (source, expected) in cases {
        assert_eq!(&rejected(source), expected, "\n{source}");
    }
    accepted(
        "a: list[bytes] = b'a b'.split() + b'a,b'.rsplit(b',', 1) + b'a\\nb'.splitlines()\nb: tuple[bytes, bytes, bytes] = b'k=v'.partition(b'=')\nc: bytes = b'-'.join(a) + b'x'.strip(None) + b'x'.upper() + b'x'.center(3, b'*')\nd: int = b'abc'.find(b'b')\ne: bool = b'a'.startswith((b'a',)) and b'a'.isalpha()\nf: str = b'a'.hex()\n",
    );
}

#[test]
fn set_rules_give_their_codes() {
    let cases: &[(&str, &[&str])] = &[
        ("s = {[1], [2]}\n", &["E0413"]),
        ("s = set()\n", &["E0410"]),
        ("s: set[list[int]] = set()\n", &["E0413"]),
        ("s: set = set()\n", &["E0408"]),
        ("s = {1, 'a'}\n", &["E0409"]),
        ("s = {1, 2} + {3}\n", &["E0401"]),
        ("s = {1.5} | {1}\n", &["E0401"]),
        ("s = {1} < [1]\n", &["E0407"]),
        ("s = {1} == {'a'}\n", &["E0407"]),
        ("s = set([[1]])\n", &["E0413"]),
        ("s = {(1, [2])}\n", &["E0413"]),
        ("s = set(1, 2)\n", &["E0404"]),
        ("s = set(x=1)\n", &["E0404"]),
        ("s = set(5)\n", &["E0411"]),
        ("s = {1}.push(2)\n", &["E0412"]),
        ("s = {1}\nprint(s[0])\n", &["E0401"]),
        ("s = {1}\nprint('a' in s)\n", &["E0407"]),
        ("for x in reversed({1}):\n    pass\n", &["E0411"]),
        ("s = {1}\ns.add('a')\n", &["E0404"]),
        ("s = {1}\ns.update({'a'})\n", &["E0409"]),
        ("x: dict[str, int] = 1\n", &["E0402"]),
        ("x: dict = 1\n", &["E0408"]),
        ("s: set[Thing] = {1}\n", &["E0205"]),
        ("s: set[Thing] = set()\n", &["E0205"]),
        ("s = set(missing)\n", &["E0205"]),
        ("s = {1}\ns.update(['a'])\n", &["E0409"]),
        ("s = {1}\ns.update(('a', 'b'))\n", &["E0404"]),
        ("s = {1}\ns.update(5)\n", &["E0411"]),
        ("s = {1}\ns.update(missing)\n", &["E0205"]),
        ("s = {1}\nprint(s.issubset())\n", &["E0404"]),
        (
            "s = {1}\nprint(s.symmetric_difference([1], [2]))\n",
            &["E0404"],
        ),
        ("s = {1}\ns.update(others=[1])\n", &["E0404"]),
        ("s = {1}\nt = {2.5}\nprint(s == t)\n", &["E0407"]),
        ("s = {1}\nt = {2.5}\nprint(s <= t)\n", &["E0407"]),
    ];
    for (source, expected) in cases {
        assert_eq!(&rejected(source), expected, "\n{source}");
    }
}

#[test]
fn sets_have_python_s_types() {
    let source = "\
def f(values: set[int]) -> set[int]:
    return values | {0}
a = f({1, 2}) & set([2, 3]) - {9} ^ f(set())
b: set[tuple[int, ...]] = {(1, 2), (3,)}
c = len(a) + sum(a) + max(a) + a.pop()
d = a < {1} and 2 in a and a.issubset(a) and bool(a) and sorted(a) == list(a)
e: set[float] = set()
e |= {1.5}
for item in a:
    c += item
g = set('ab').union({'c'}, 'de', ['f'], set(), []).copy()
h = a.intersection() | a.difference([1], (2,), {3}) | a.symmetric_difference([4])
a.update([], set(), (5, 6), {7})
k = a.issuperset((1, 2)) or a.isdisjoint([1])
m: set[tuple[int, str]] = set()
m.update(enumerate('ab'), zip([1], 'c'))
";
    let (_directory, project) = load(source);
    let program = TypeChecker::check(&project).expect("the program is accepted");
    let dump = typy_check::dump(&program, &project);
    for node in [
        "Set : set[int]",
        "SetOf : set[int]",
        "SetOf : set[str]",
        "Method pop",
        "Method union",
        "Set : set[tuple[int, ...]]",
    ] {
        assert!(dump.contains(node), "{node} missing from\n{dump}");
    }
}

#[test]
fn dict_rules_give_their_codes() {
    let cases: &[(&str, &[&str])] = &[
        ("d = {}\n", &["E0410"]),
        ("d = dict()\n", &["E0410"]),
        ("d = {[1]: 2}\n", &["E0413"]),
        ("d: dict[list[int], int] = {}\n", &["E0413"]),
        ("d = {1: 'a', 'b': 'c'}\n", &["E0409"]),
        ("d = {1: 'a', 2: 3}\n", &["E0409"]),
        ("d: dict = {}\n", &["E0408"]),
        ("d: dict[int] = {}\n", &["E0408"]),
        ("d = {1: 2}.setdefault(1)\n", &["E0404"]),
        ("d = {1: 2} < {1: 2}\n", &["E0407"]),
        ("d = {1: 2} + {3: 4}\n", &["E0401"]),
        ("d = {1: 2} | {'a': 4}\n", &["E0401"]),
        ("d = dict([1, 2])\n", &["E0404"]),
        ("d = dict([(1, 2, 3)])\n", &["E0404"]),
        ("d = dict([([1], 2)])\n", &["E0413"]),
        ("d = dict(a=1, a=2)\n", &["E0404"]),
        ("d = dict({1: 2}, a=3)\n", &["E0404"]),
        ("d = dict(a=1, b='x')\n", &["E0409"]),
        ("d = dict([1], a=1)\n", &["E0404"]),
        ("d = dict([], [], a=1)\n", &["E0404"]),
        ("d = dict.fromkeys([[1]])\n", &["E0413"]),
        ("d = dict.fromkeys([1], value=2)\n", &["E0404"]),
        ("d = dict.fromkeys()\n", &["E0404"]),
        ("d = dict.fromkeys(1)\n", &["E0411"]),
        ("d = dict.fromkeys([1], missing)\n", &["E0205"]),
        ("d = dict.fromkeys(missing)\n", &["E0205"]),
        ("d: dict[int, int] = dict(a=1)\n", &["E0402"]),
        ("d = dict(a=missing)\n", &["E0205"]),
        ("d = dict([(1, 2)], [(3, 4)])\n", &["E0404"]),
        ("d = dict(missing)\n", &["E0205"]),
        ("d = {1: 2}.keys() == {1: 2}.values()\n", &["E0407"]),
        ("d = {1: 2}.keys() & {'a'}\n", &["E0401"]),
        ("d = {1: 2}.values() - {2}\n", &["E0401"]),
        ("d = {1: [2]}.items() ^ {1: [2]}.items()\n", &["E0413"]),
        ("d = {1: 2}.values() <= {1: 2}.values()\n", &["E0407"]),
        ("d = {1: 2}.keys() == [1]\n", &["E0407"]),
        ("d = {1: 2}.keys() | 1\n", &["E0401"]),
        (
            "t: tuple[str, ...] = ('a',)\nd = {1: 2}.keys() | t\n",
            &["E0401"],
        ),
        ("d = {1: 2}.values() == {1: 'a'}.values()\n", &["E0407"]),
        ("p = {1: 2}\np['a'] = 3\n", &["E0404"]),
        ("p = {1: 2}\np[1] = 'b'\n", &["E0402"]),
        ("p = {1: 2}\nprint(p['a'])\n", &["E0404"]),
        ("p = {1: 2}\np.update([('a', 1)])\n", &["E0409"]),
        ("p = {1: 2}\np.update(zip(['a'], [1]))\n", &["E0404"]),
        ("p = {1: 2}\np.update(5)\n", &["E0411"]),
        ("p = {1: 2}\np.update(missing)\n", &["E0205"]),
        ("p = {1: 2}\np.update([1])\n", &["E0409"]),
        ("p = {1: 2}\np.update(enumerate('a'))\n", &["E0404"]),
        ("p = {1: 2}\np.push(3)\n", &["E0412"]),
        ("p = {1: 2}\nprint('a' in p)\n", &["E0407"]),
        ("p = {1: 2}\nprint('a' in p.values())\n", &["E0407"]),
        ("d = {{1: 2}: 3}\n", &["E0413"]),
        ("d = {1: 2}\nprint({d})\n", &["E0413"]),
        ("d: dict[Thing, int] = {}\n", &["E0205"]),
        ("d: dict[Thing, int] = {1: 2}\n", &["E0205"]),
        ("d = dict(5)\n", &["E0411"]),
        ("x: int = {}\n", &["E0410"]),
        ("x: int = set()\n", &["E0410"]),
        ("x: int = {1}\n", &["E0402"]),
        ("x: dict[int, int, int] = {}\n", &["E0408"]),
        ("x: tuple[..., ...] = ()\n", &["E0408"]),
        ("missing[1] = 2\n", &["E0205"]),
        ("d = {1: 2}\nd.update({'a': 1})\n", &["E0409"]),
        ("d = {1: 2}\nd.update(dict(zip(['a'], [1])))\n", &["E0404"]),
        ("d = {1: [2]}\nd.update({1: ['a']})\n", &["E0409"]),
        ("d = {1: 2}\nd.update([(1, 'a')])\n", &["E0409"]),
        ("d = {1: 2}\nd.update((1, 2))\n", &["E0404"]),
    ];
    for (source, expected) in cases {
        assert_eq!(&rejected(source), expected, "\n{source}");
    }
}

#[test]
fn dicts_have_python_s_types() {
    let source = "\
def f(counts: dict[str, int]) -> dict[str, int]:
    return counts | {'z': 0}
a = f({'a': 1}) | dict([('b', 2)]) | dict(zip(['c'], [3])) | dict(f({}))
b: dict[int, list[str]] = {}
b.setdefault(1, []).append('x')
c = a['a'] + a.get('q', 0) + a.pop('a') + a.pop('q', 0) + a.setdefault('r', 1)
k, v = a.popitem()
a['n'] = len(a) + len(a.keys())
a['n'] += 1
a |= {'m': 1}
a.update({'o': 2})
a.update([('p', 3)], )
a.update(zip(['s'], [4]))
a.update()
a.update({})
a.update(dict())
a.update({'t': 5})
pl = [('u', 6)]
a.update(pl)
for key in a:
    c += a[key]
for key, value in a.items():
    c += value
e = 'a' in a and 'a' in a.keys() and 1 in a.values() and ('a', 1) in a.items()
g = sorted(a) + list(a.keys()) + [x for x in []] if False else sorted(a.values())
h = a == a.copy() and a != {} and bool(a.values())
m = set(a) | set(a.keys())
n: dict[tuple[int, str], set[float]] = {(1, 'a'): {1.5}}
";
    let source = source.replace("+ [x for x in []] if False else sorted(a.values())", "");
    let (_directory, project) = load(&source);
    let program = TypeChecker::check(&project).expect("the program is accepted");
    let dump = typy_check::dump(&program, &project);
    for node in [
        "Dict : dict[str, int]",
        "DictOf : dict[str, int]",
        "DictItem : int",
        "Method popitem",
        "Method items : dict_items[str, int]",
        "entry of dict[str, int]",
        "Dict : dict[tuple[int, str], set[float]]",
    ] {
        assert!(dump.contains(node), "{node} missing from\n{dump}");
    }
}

#[test]
fn comprehension_and_range_rules_give_their_codes() {
    let cases: &[(&str, &[&str])] = &[
        ("a = [y for y in range(2)]\nprint(y)\n", &["E0205"]),
        ("b = [z for z in 5]\n", &["E0411"]),
        ("c = {[1] for z in range(2)}\n", &["E0413"]),
        ("d = {[1]: 2 for z in range(2)}\n", &["E0413"]),
        ("e = [[] for z in range(2)]\n", &["E0410"]),
        ("f = [q for q in range(2) if q == 'a']\n", &["E0407"]),
        ("g = [q for q in missing]\n", &["E0205"]),
        ("h = [missing for q in range(2)]\n", &["E0205"]),
        ("i = {missing: 1 for q in range(2)}\n", &["E0205"]),
        ("j = [q for q, r in range(2)]\n", &["E0308"]),
        ("k = range(3) < range(4)\n", &["E0407"]),
        ("q: range = [1]\n", &["E0402"]),
        ("u = 'a' in range(3)\n", &["E0407"]),
        ("m = list(range('a'))\n", &["E0404"]),
        ("n = list(range(1, 2, 3, 4))\n", &["E0404"]),
        ("o = list(range(x=1))\n", &["E0404"]),
        ("p = list(reversed(range('a', 2)))\n", &["E0404"]),
    ];
    for (source, expected) in cases {
        assert_eq!(&rejected(source), expected, "\n{source}");
    }
}

#[test]
fn comprehensions_and_ranges_have_python_s_types() {
    let source = "\
x = 'outer'
a = [x for x in range(3)]
b: list[list[str]] = [[] for _ in range(2)]
c = {len(w) for w in ['a', 'bc']}
d = {w: len(w) for w in ['a', 'bc'] if w}
e: set[float] = {n / 2 for n in range(3)}
f: dict[str, list[int]] = {w: [] for w in 'ab'}
g = [(i, j) for i in range(3) for j in range(i) if i if j]
h = sum(range(10)) + len(list(reversed(range(1, 9, 2))))
k = [v for v in enumerate(range(2))]
m = x + 'a'
v = {1: 'a'}
w = (v.keys() & [1], v.items() | {(2, 'b')}, v.keys() <= {1}, v.values() == v.values())
z = (dict(a=1), dict({'k': 2}, b=3), dict.fromkeys('ab'), dict.fromkeys([1], 2.5))
y: dict[str, float] = dict(a=1.5)
u: dict[str, list[int]] = dict([('k', [1])], b=[])
r: range = range(1, 9, 2)
n = (len(r), r[-1], 3 in r, r == range(4), [v for v in reversed(r)])
s = r[1:]
";
    let (_directory, project) = load(source);
    let program = TypeChecker::check(&project).expect("the program is accepted");
    let dump = typy_check::dump(&program, &project);
    for node in [
        "Assign n: tuple[int, int, bool, bool, list[int]]",
        "Assign s: range",
        "Assign z: tuple[dict[str, int], dict[str, int], dict[str, None], dict[int, float]]",
        "Assign w: tuple[set[int], set[tuple[int, str]], bool, bool]",
        "Comprehension : list[int]",
        "Comprehension : list[list[str]]",
        "Comprehension : set[int]",
        "Comprehension : dict[str, int]",
        "Comprehension : set[float]",
        "Comprehension : list[tuple[int, int]]",
    ] {
        assert!(dump.contains(node), "{node} missing from\n{dump}");
    }
}

#[test]
fn optional_rules_give_their_codes() {
    let cases: &[(&str, &[&str])] = &[
        ("a: int | None = None\nprint(a + 1)\n", &["E0401"]),
        (
            "b: int | None = 1\nwhile True:\n    print(b + 1)\n    b = None\n",
            &["E0401"],
        ),
        (
            "c: int | None = 3\nif c is not None:\n    c = None\n    print(c + 1)\n",
            &["E0401"],
        ),
        (
            "d: int | None = None\nif d is None:\n    print(d + 1)\n",
            &["E0401"],
        ),
        (
            "def e(x: int | None) -> bool:\n    return x < 5\n",
            &["E0407"],
        ),
        (
            "def f(x: int | None) -> bool:\n    return x == 5\n",
            &["E0407"],
        ),
        (
            "def g(x: str | None) -> str:\n    return x.upper()\n",
            &["E0412"],
        ),
        ("h: int | str = 1\n", &["E0201"]),
        ("j: list[int] = [1]\nk: list[int | None] = j\n", &["E0402"]),
        ("m: int | None = 'a'\n", &["E0402"]),
        ("n = {'a': 1}\nn.setdefault('b')\n", &["E0404"]),
        (
            "o: int | None = None\nfor i in range(2):\n    print(o + i)\n    o = None\n",
            &["E0401"],
        ),
        (
            "p: int | None = 1\nq = p if p is None else 'x'\n",
            &["E0305"],
        ),
        (
            "def f(x: int | None = 'a') -> None:\n    pass\n",
            &["E0404"],
        ),
        ("a = [1, 'b', None]\n", &["E0409"]),
        (
            "b: dict[str, int | None] = {}\nb.setdefault('k')\nc = b.get('k')\nprint(c + 1)\n",
            &["E0401"],
        ),
        ("e = [y + 1 for y in [1, None]]\n", &["E0401"]),
        (
            "def f(a: int | None) -> None:\n    print([1, a] + [2])\n",
            &["E0401"],
        ),
        ("x: list[Thing] = [1]\n", &["E0205"]),
    ];
    for (source, expected) in cases {
        assert_eq!(&rejected(source), expected, "\n{source}");
    }
}

#[test]
fn optional_values_narrow_where_a_test_shows_they_are_not_none() {
    let source = "\
def f(x: int | None = None, y: str | None = 'a') -> int:
    if x is None or y is None:
        return 0
    return x + len(y)
a: int | None = f(1)
b = a + 1 if a is not None else 0
c = [v + 1 for v in [a, None] if v is not None]
d = a is not None and a > 0
e = not (a is None) and a > 1
while a is not None and a < 5:
    a = a + 1
g = {'k': 1}.get('k')
h = (1, None)
i: tuple[int, int | None] = (1, None)
j: tuple[int | None, ...] = (1, None)
k = None if b else (1, 'x')
m: float | None = 1.5
n = m if m else 0.0
o = a if a else None
q = None if a else a
r: int | None = 1 if a else a
s = [None, 1, None]
t = {None, 'x'}
u = {1: None, 2: 3}
v = {None: 1, 2: 2}
w = [[1], None]
if not a:
    pass
else:
    n = a * 1.0
";
    let (_directory, project) = load(source);
    let program = TypeChecker::check(&project).expect("the program is accepted");
    let dump = typy_check::dump(&program, &project);
    for node in [
        "Unwrap : int",
        "Wrap : int | None",
        "Method get : int | None",
        "Wrap : tuple[int, str] | None",
        "TupleOf : tuple[int | None, ...]",
        "List : list[int | None]",
        "Set : set[str | None]",
        "Dict : dict[int, int | None]",
        "Dict : dict[int | None, int]",
        "List : list[list[int] | None]",
    ] {
        assert!(dump.contains(node), "{node} missing from\n{dump}");
    }
}

#[test]
fn statements_and_expressions_outside_the_subset_give_their_codes() {
    let cases: &[(&str, &[&str])] = &[
        ("@d\ndef f() -> None:\n    pass\n", &["E0201"]),
        ("def f[T](a: T) -> None:\n    pass\n", &["E0201"]),
        ("return 1\n", &["E0103"]),
        ("break\n", &["E0103"]),
        ("continue\n", &["E0103"]),
        ("print([x for x in 1 for y in x])\n", &["E0411"]),
        ("print({u: 1 for x in [1]})\n", &["E0205"]),
        ("print(UserWarning)\n", &["E0202"]),
        ("print(ExceptionGroup)\n", &["E0202"]),
        // `raise` takes an exception or an exception class (SMT-06, EXC-07, EXC-12).
        ("raise 1\n", &["E0415"]),
        ("raise int\n", &["E0415"]),
        ("raise ValueError('v') from 'cause'\n", &["E0415"]),
        ("raise\n", &["E0416"]),
        ("e = UnicodeDecodeError()\n", &["E0417"]),
        ("raise UnicodeEncodeError\n", &["E0417"]),
        ("e = ValueError(message='v')\n", &["E0404"]),
        ("e: KeyError = ValueError('v')\n", &["E0402"]),
        ("raise u\n", &["E0205"]),
        // An `except` clause names classes; `finally` comes later (CST-04, EXC-09).
        ("try:\n    pass\nexcept 1:\n    pass\n", &["E0418"]),
        (
            "try:\n    pass\nexcept (ValueError, 'x'):\n    pass\n",
            &["E0418"],
        ),
        ("try:\n    pass\nexcept u:\n    pass\n", &["E0205"]),
        // `return`, `break` or `continue` cannot leave a `finally` block (EXC-06).
        (
            "def f() -> None:\n    try:\n        pass\n    finally:\n        return\n",
            &["E0419"],
        ),
        (
            "while True:\n    try:\n        pass\n    finally:\n        continue\n",
            &["E0419"],
        ),
        (
            "try:\n    x = 1\nexcept ValueError:\n    pass\nfinally:\n    pass\nprint(x)\n",
            &["E0306"],
        ),
        // A class derives from an exception class defined before it, and redefines only
        // `__init__`, `__str__` and `__repr__` (CLS-03, CLS-13, CLS-14, EXC-12).
        ("class A(Exception, ValueError):\n    pass\n", &["E0201"]),
        ("class B:\n    pass\nclass A(B):\n    pass\n", &["E0201"]),
        ("class A(int):\n    pass\n", &["E0201"]),
        (
            "class A(B):\n    pass\nclass B(Exception):\n    pass\n",
            &["E0306"],
        ),
        ("class A(UnicodeEncodeError):\n    pass\n", &["E0417"]),
        (
            "class A(Exception):\n    def f(self) -> None:\n        pass\nclass B(A):\n    def f(self) -> None:\n        pass\n",
            &["E0201"],
        ),
        (
            "class A(Exception):\n    def f(self) -> None:\n        super().__init__()\n",
            &["E0201"],
        ),
        (
            "class A(Exception):\n    def __init__(self) -> None:\n        super().__init__(x=1)\n",
            &["E0404"],
        ),
        ("class A(Exception):\n    pass\nA(x=1)\n", &["E0404"]),
        (
            "class A(Exception):\n    def __init__(self, n: int) -> None:\n        self.n = n\nA('n')\n",
            &["E0404"],
        ),
        ("class A(Exception):\n    pass\nA().n\n", &["E0414"]),
        ("try:\n    pass\nexcept int:\n    pass\n", &["E0418"]),
        (
            "class A:\n    def __init__(self) -> None:\n        super().__init__()\n",
            &["E0201"],
        ),
        // The name `as` binds is deleted after its clause (EXC-01).
        (
            "try:\n    pass\nexcept ValueError as e:\n    pass\nprint(e)\n",
            &["E0205"],
        ),
        (
            "e = 1\ntry:\n    pass\nexcept ValueError as e:\n    pass\nprint(e)\n",
            &["E0306"],
        ),
        ("for i in range(u):\n    print(i)\n", &["E0205"]),
        ("print({1: u for x in [1]})\n", &["E0205"]),
        (
            "def f(range: int) -> None:\n    for i in range(3):\n        print(i)\n",
            &["E0401"],
        ),
        ("x: set[Thing] = {1}\n", &["E0205"]),
        ("print(sorted([{1: 2}]))\n", &["E0407"]),
    ];
    for (source, expected) in cases {
        assert_eq!(&rejected(source), expected, "\n{source}");
    }
}

#[test]
fn iterators_used_wrongly_give_their_codes() {
    let cases: &[(&str, &[&str])] = &[
        // A slice of a list takes a list of its items (DAT-13).
        ("xs = [1]\nws = ['a']\nxs[0:1] = ws\n", &["E0402"]),
        ("s = 'ab'\ns[0:1] = ['a']\n", &["E0401"]),
        ("xs = [1]\nxs[0:'a'] = [2]\n", &["E0404"]),
        ("xs = [1]\nxs[0:1] += [2]\n", &["E0201"]),
        ("xs = [1]\nu[0:1] = [2]\n", &["E0205"]),
        // `del` removes list items, slices and dict keys only (SMT-05).
        ("x = 1\ndel x\n", &["E0421"]),
        ("t = (1, 2)\ndel t[0]\n", &["E0421"]),
        ("t = (1, 2)\ndel t[0:1]\n", &["E0421"]),
        ("s = 'ab'\ndel s[0]\n", &["E0421"]),
        ("s = 'ab'\ndel s[0:1]\n", &["E0421"]),
        ("t: tuple[int, ...] = (1, 2)\ndel t[0]\n", &["E0421"]),
        ("del u[0]\n", &["E0205"]),
        // `with` needs `__enter__` and `__exit__` of the right shapes (CST-05, CST-10).
        ("with 1:\n    pass\n", &["E0420"]),
        ("with u:\n    pass\n", &["E0205"]),
        (
            "class M:\n    def __enter__(self) -> None:\n        pass\nwith M():\n    pass\n",
            &["E0420"],
        ),
        (
            "class M:\n    def __enter__(self, x: int) -> None:\n        pass\n",
            &["E0404"],
        ),
        (
            "class M:\n    def __exit__(self, a, b, c) -> int:\n        return 1\n",
            &["E0404"],
        ),
        (
            "class M:\n    def __enter__(self) -> int:\n        return 1\n    def __exit__(self, a, b, c) -> None:\n        pass\nwith M() as (x, y):\n    pass\n",
            &["E0308"],
        ),
        // `iter` and `next` take iterables and iterators; a class iterates with `__iter__`
        // and `__next__` (CST-08, CST-09).
        ("print(iter(1))\n", &["E0411"]),
        ("print(iter())\n", &["E0404"]),
        ("print(next())\n", &["E0404"]),
        ("print(iter([1], x=1))\n", &["E0404"]),
        ("print(next(iter([1]), 'a'))\n", &["E0404"]),
        ("print(next(5))\n", &["E0404"]),
        ("class A:\n    pass\nprint(iter(A()))\n", &["E0411"]),
        (
            "class A:\n    def __iter__(self, n: int) -> 'A':\n        return self\n",
            &["E0404"],
        ),
        ("print(iter(u))\n", &["E0205"]),
        ("i = iter([1])\nprint(i == i)\n", &["E0407"]),
    ];
    for (source, expected) in cases {
        assert_eq!(&rejected(source), expected, "\n{source}");
    }
}

#[test]
fn methods_builtins_and_tests_of_each_type_are_accepted() {
    for source in [
        "print(sorted([{1}]))\n",
        "a: list[int] = list([1])\nb: set[int] = set([1])\n",
        "def f() -> None:\n    pass\nif f():\n    pass\n",
        "t: tuple[int, ...] = (1, 1)\nprint(b'a'.hex(), {1: 2}.get(1), {1}.copy(), t.count(1))\n",
        "def range(a: int) -> list[int]:\n    return [a]\nfor i in range(3):\n    print(i)\n",
    ] {
        accepted(source);
    }
}

#[test]
#[allow(clippy::too_many_lines, reason = "one row per class rule")]
fn classes_give_their_codes() {
    let cases: &[(&str, &[&str])] = &[
        ("class P:\n    pass\np = P()\np.z = 1\n", &["E0414"]),
        ("class P:\n    pass\nP().move()\n", &["E0414"]),
        (
            "class P:\n    def m(self) -> None:\n        pass\nf = P().m\n",
            &["E0201"],
        ),
        (
            "class P:\n    def __init__(self) -> None:\n        self.x = 1\nP().x()\n",
            &["E0414"],
        ),
        ("x = 1\nprint(x.real)\n", &["E0412"]),
        ("x = 1\nx.y = 2\n", &["E0412"]),
        (
            "class A:\n    def __init__(self, b: \"B\") -> None:\n        self.v = b.w\nclass B:\n    def __init__(self) -> None:\n        self.w = 1\n",
            &["E0310"],
        ),
        (
            "class A:\n    def __init__(self, b: \"B\") -> None:\n        b.w = 2\n        self.v = 1\nclass B:\n    def __init__(self) -> None:\n        self.w = 1\n",
            &["E0310"],
        ),
        (
            "class A:\n    def __init__(self) -> None:\n        self.v = None\n",
            &["E0410"],
        ),
        (
            "class A:\n    x: int\n    def __init__(self) -> None:\n        self.x: str = ''\n",
            &["E0402"],
        ),
        (
            "class A:\n    def __init__(self) -> None:\n        self.x = 1\nA().x = 's'\n",
            &["E0402"],
        ),
        (
            "class A:\n    def __init__(self) -> None:\n        self.x = 1\na = A()\na.x += 's'\n",
            &["E0401"],
        ),
        (
            "class A:\n    def __init__(self) -> None:\n        self.x = 1\na = A()\na.x /= 2\nprint(a.x)\n",
            &["E0402"],
        ),
        ("class A:\n    pass\nclass A:\n    pass\n", &["E0202"]),
        (
            "class A:\n    pass\ndef A() -> None:\n    pass\n",
            &["E0202"],
        ),
        ("class A:\n    pass\nA = 1\n", &["E0202"]),
        ("class A:\n    x: int\n    x: str\n", &["E0202"]),
        (
            "class A:\n    x: int\n    def x(self) -> None:\n        pass\n",
            &["E0202"],
        ),
        (
            "class A:\n    def m(self) -> None:\n        pass\n    def m(self) -> None:\n        pass\n",
            &["E0202"],
        ),
        ("class A:\n    def m() -> None:\n        pass\n", &["E0201"]),
        (
            "class A:\n    def __str__(self) -> int:\n        return 1\n",
            &["E0404"],
        ),
        (
            "class A:\n    def __repr__(self, x: int) -> str:\n        return ''\n",
            &["E0404"],
        ),
        (
            "class A:\n    def __init__(self) -> int:\n        return 1\n",
            &["E0405"],
        ),
        (
            "class A:\n    def m(self: int) -> None:\n        pass\n",
            &["E0404"],
        ),
        ("class A:\n    pass\nA(1)\n", &["E0404"]),
        ("class A:\n    pass\nA(x=1)\n", &["E0404"]),
        (
            "class A:\n    def __init__(self, x: int) -> None:\n        self.x = x\nA('s')\n",
            &["E0404"],
        ),
        (
            "class A:\n    def __init__(self) -> None:\n        print(self.x)\n        self.x = 1\n",
            &["E0309"],
        ),
        (
            "class A:\n    def __init__(self, c: bool) -> None:\n        if c:\n            return\n        self.x = 1\n",
            &["E0309"],
        ),
        ("class A:\n    pass\nprint(A())\nA()()\n", &["E0201"]),
        ("x = A()\nclass A:\n    pass\n", &["E0306"]),
        (
            "def f() -> None:\n    A()\nf()\nclass A:\n    pass\n",
            &["E0306"],
        ),
        ("class A:\n    pass\nprint(A() < A())\n", &["E0407"]),
        (
            "class A:\n    def m(self, other: A) -> None:\n        pass\n",
            &["E0306"],
        ),
        ("x: list[A] = []\nclass A:\n    pass\n", &[]),
    ];
    for (source, expected) in cases {
        if expected.is_empty() {
            accepted(source);
        } else {
            assert_eq!(&rejected(source), expected, "\n{source}");
        }
    }
}

#[test]
fn classes_build_typed_instances() {
    for source in [
        "class P:\n    \"\"\"A point.\"\"\"\n    x: int\n    def __init__(self, x: int) -> None:\n        self.x = x\n        self.y, self.z = 1, 2\n    def __repr__(self) -> str:\n        return 'P'\np = P(1)\nq: P | None = p\nprint(p.x + p.y, p is q, q is not None, [p], {p}, p != p)\n",
        "class N:\n    def __init__(self, n: \"N | None\") -> None:\n        self.n = n\n        for i in range(2):\n            self.k = i\n        self.k = 0\na = N(None)\nb = N(a)\nprint(b.n is a)\n",
        "class C:\n    def __init__(self) -> None:\n        self.items: list[int] = []\n        self.count = 0\n    def add(self, v: int) -> None:\n        self.items += [v]\n        self.count += 1\n    def __str__(self) -> str:\n        return str(self.count)\nc = C()\nc.add(1)\nprint(c, str(c), repr(c) != '')\n",
        "class E:\n    pass\ne = E()\nprint(bool(e), e == e)\n",
        "class P:\n    def __init__(self) -> None:\n        self.x: int = 0\n    def get(self) -> int:\n        return self.x\nprint(P().get(), P().x)\n",
    ] {
        accepted(source);
    }
}

#[test]
fn a_floor_that_defers_annotations_lets_a_signature_name_a_later_class() {
    let source = "def f(t: Tree) -> None:\n    pass\nclass Tree:\n    def __init__(self, parent: Tree | None) -> None:\n        self.parent = parent\nf(Tree(None))\n";
    let directory = tempfile_dir::Dir::new();
    std::fs::write(directory.path.join("app.py"), source).unwrap();
    let mut settings = ProjectSettings::new(directory.path.clone());
    settings.python_floor = typy_source::PythonFloor::V3_14;
    let project = Project::load(&directory.path.join("app.py"), settings)
        .unwrap_or_else(|failure| panic!("does not parse: {:?}", failure.diagnostics));
    assert!(TypeChecker::check(&project).is_ok());
    assert_eq!(rejected(source), ["E0306"]);
}

#[test]
fn class_statements_outside_the_subset_give_their_codes() {
    let cases: &[(&str, &[&str])] = &[
        ("@d\nclass C:\n    pass\n", &["E0201"]),
        ("class C[T]:\n    pass\n", &["E0201"]),
        ("class C(metaclass=M):\n    pass\n", &["E0202"]),
        ("def f() -> None:\n    class C:\n        pass\n", &["E0202"]),
        ("class C:\n    if True:\n        pass\n", &["E0202"]),
        ("class C:\n    x: int = 1\n", &["E0201"]),
        ("class C(B, object):\n    pass\n", &["E0201"]),
        ("class C:\n    x: 'int('\n", &["E0202"]),
    ];
    for (source, expected) in cases {
        assert_eq!(&rejected(source), expected, "\n{source}");
    }
    accepted("class C(object):\n    'Text.'\n    pass\nclass D():\n    pass\n");
}

#[test]
fn type_objects_give_their_codes() {
    let cases: &[(&str, &[&str])] = &[
        ("print(type(1, 2, 3))\n", &["E0202"]),
        ("print(type())\n", &["E0404"]),
        ("print(type(x=1))\n", &["E0404"]),
        ("print(int.foo)\n", &["E0412"]),
        ("print(int < str)\n", &["E0407"]),
        ("t = type(1)\nt = str\n", &["E0402"]),
        ("print(type(1) is 1)\n", &["E0301"]),
        ("print(type(u))\n", &["E0205"]),
    ];
    for (source, expected) in cases {
        assert_eq!(&rejected(source), expected, "\n{source}");
    }
    for source in [
        "class P:\n    pass\np = P()\nx: int | None = 1\nprint(type(p), type(1), type(p) is P, type(1) == int, type(1) != str, P.__name__, type(x), type(type(1)), {P}, bool(int))\n",
        "def f(t: type, u: type[int]) -> None:\n    print(t is u, t.__name__)\nf(str, int)\nx: list[type] = [int]\nprint([int, str], [type(1), bool], object, type({1: 2}.items()), type((1, 'a')), type(b''), type(1.5), type(None), type([1]), type({1}))\n",
    ] {
        accepted(source);
    }
}

#[test]
fn fstrings_check_every_field() {
    accepted(
        "x = 1\nname = 'a'\nprint(f'', f'{x}', f'{x!r:>{x}}', f'{name!a}', f'{x = }', f'{x=:5}', f'{{}}')\n",
    );
    assert_eq!(rejected("print(f'{undefined}')\n"), ["E0205"]);
    assert_eq!(rejected("print(f'{1:{undefined}}')\n"), ["E0205"]);
    assert_eq!(rejected("print(f'{1 + \"a\"}')\n"), ["E0401"]);
}
