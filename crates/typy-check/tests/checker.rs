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
    assert_eq!((total.name.as_str(), total.ty), ("total", Type::Int));
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
        .map(|symbol| program.symbol(*symbol).ty)
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
        ("def f(x) -> None:\n    pass\n", &["E0203"]),
        ("def f(x: int):\n    pass\n", &["E0204"]),
        ("def f(x: Any) -> None:\n    pass\n", &["E0403"]),
        ("def f(x: list) -> None:\n    pass\n", &["E0201"]),
        ("def f(x: Thing) -> None:\n    pass\n", &["E0205"]),
        ("print(undefined)\n", &["E0205"]),
        ("print(eval(\"1\"))\n", &["E0202"]),
        ("x = [1]\n", &["E0201"]),
        ("class C:\n    pass\n", &["E0201"]),
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
        ("print(min(\"ab\"))\n", &["E0201"]),
        ("print(len(1))\n", &["E0404"]),
        ("print(len(\"a\", \"b\"))\n", &["E0404"]),
        ("print(abs(\"a\"))\n", &["E0401"]),
        ("print(1, file=None)\n", &["E0201"]),
        ("print(1, color=1)\n", &["E0404"]),
        ("x = range(3)\n", &["E0201"]),
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
        ("print(list)\n", &["E0201"]),
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
        messages("print(list)\n"),
        ["`list` is planned for TyPy 0.2 (LIB-02)"]
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
