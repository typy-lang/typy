//! Tests of the 0.1 feature table.

use super::check;
use typy_diagnostics::FileId;
use typy_source::{PythonFloor, SourceFile, parse};

fn diagnostics(source: &str) -> Vec<(String, String)> {
    diagnostics_with_floor(source, PythonFloor::V3_12)
}

fn diagnostics_with_floor(source: &str, floor: PythonFloor) -> Vec<(String, String)> {
    let file = SourceFile::from_text(FileId(0), "app.py".into(), source);
    let tree = parse(&file, floor).expect("the source parses");
    let mut found: Vec<(String, String)> = Vec::new();
    for diagnostic in check(&tree).into_sorted(&Lookup(&file)) {
        found.push((
            diagnostic.code().to_string(),
            diagnostic.message().to_owned(),
        ));
    }
    found
}

struct Lookup<'file>(&'file SourceFile);

impl typy_diagnostics::SourceLookup for Lookup<'_> {
    fn path(&self, _file: FileId) -> &str {
        self.0.path().as_str()
    }
    fn position(
        &self,
        _file: FileId,
        offset: typy_diagnostics::TextOffset,
    ) -> typy_diagnostics::Position {
        self.0.position(offset)
    }
    fn line_text(&self, _file: FileId, line: u32) -> &str {
        self.0.lines().line_text(self.0.text(), line)
    }
}

#[test]
fn features_of_0_1_give_no_diagnostic() {
    let source = "\
def f(a: int, b: int = 1) -> int:
    x = a
    x += b
    y: int = 2
    for i in range(1, 10, 2):
        if i > 5:
            break
        continue
    while x > 0:
        x -= 1
    return x
print(f(1), not True, -1, 1 if True else 2, 'abc'[0], 1 < 2)
";
    assert_eq!(diagnostics(source), []);
}

#[test]
#[allow(clippy::too_many_lines, reason = "one row per planned feature")]
fn later_features_name_their_release_and_rule() {
    let cases = [
        (
            "class C:\n    pass\n",
            "classes are planned for TyPy 0.2 (CLS-01)",
        ),
        ("x = [1]\n", "lists are planned for TyPy 0.2 (DAT-07)"),
        ("x = (1, 2)\n", "tuples are planned for TyPy 0.2 (DAT-08)"),
        ("x = {1: 2}\n", "dicts are planned for TyPy 0.2 (DAT-09)"),
        ("x = {1}\n", "sets are planned for TyPy 0.2 (DAT-10)"),
        (
            "x = [i for i in range(3)]\n",
            "comprehensions are planned for TyPy 0.2 (EXP-12)",
        ),
        (
            "x = f'{1}'\n",
            "f-strings are planned for TyPy 0.2 (LEX-07)",
        ),
        ("x = b'a'\n", "bytes are planned for TyPy 0.2 (DAT-06)"),
        (
            "x = 'a'.upper()\n",
            "attributes and methods are planned for TyPy 0.2 (CLS-01)",
        ),
        (
            "x = 'abc'[0:1]\n",
            "slices are planned for TyPy 0.2 (DAT-12)",
        ),
        (
            "print(1 in 2)\n",
            "`in` and `not in` are planned for TyPy 0.2 (EXP-07)",
        ),
        (
            "for c in 'abc':\n    pass\n",
            "`for` loops over values other than `range` are planned for TyPy 0.2 (CST-03)",
        ),
        (
            "a, b = 1, 2\n",
            "unpacking assignments are planned for TyPy 0.2 (SMT-02)",
        ),
        ("import os\n", "imports are planned for TyPy 0.3 (SMT-07)"),
        (
            "try:\n    pass\nexcept E:\n    pass\n",
            "`try` statements are planned for TyPy 0.3 (CST-04)",
        ),
        (
            "raise E\n",
            "`raise` statements are planned for TyPy 0.3 (SMT-06)",
        ),
        (
            "with f:\n    pass\n",
            "`with` statements are planned for TyPy 0.3 (CST-05)",
        ),
        (
            "assert True\n",
            "`assert` statements are planned for TyPy 0.3 (SMT-04)",
        ),
        (
            "del x\n",
            "`del` statements are planned for TyPy 0.3 (SMT-05)",
        ),
        (
            "global x\n",
            "`global` statements are planned for TyPy 0.3 (SMT-08)",
        ),
        (
            "type X = int\n",
            "`type` statements are planned for TyPy 0.4 (SMT-10)",
        ),
        (
            "def f[T](x: T) -> T:\n    return x\n",
            "type parameters are planned for TyPy 0.4 (GEN-01)",
        ),
        (
            "match x:\n    case _:\n        pass\n",
            "`match` statements are planned for TyPy 0.5 (CST-06)",
        ),
        (
            "print(y := 1)\n",
            "assignment expressions are planned for TyPy 0.5 (EXP-13)",
        ),
        (
            "f = lambda: 1\n",
            "`lambda` expressions are planned for TyPy 1.0 (EXP-14)",
        ),
        (
            "x = (i for i in range(3))\n",
            "generator expressions are planned for TyPy 1.0 (EXP-15)",
        ),
        (
            "def f() -> None:\n    def g() -> None:\n        pass\n",
            "nested functions are planned for TyPy 1.0 (FUN-04)",
        ),
        (
            "@d\ndef f() -> None:\n    pass\n",
            "decorators are planned for TyPy 1.0 (FUN-06)",
        ),
        (
            "def f(*a: int) -> None:\n    pass\n",
            "`*args` and `**kwargs` parameters are planned for TyPy 1.0 (EXP-17)",
        ),
        (
            "def f(a: int, /) -> None:\n    pass\n",
            "positional-only and keyword-only parameters are planned for TyPy 1.0 (FUN-03)",
        ),
        (
            "def f(a: int = g()) -> None:\n    pass\n",
            "default values other than constants are planned for TyPy 1.0 (EXP-16)",
        ),
        (
            "print(*x)\n",
            "unpacking with `*` are planned for TyPy 1.0 (EXP-17)",
        ),
        (
            "print(**x)\n",
            "unpacking with `**` are planned for TyPy 1.0 (EXP-17)",
        ),
        (
            "def f() -> None:\n    yield 1\n",
            "generators are planned for TyPy 1.0 (SMT-11)",
        ),
        (
            "def f() -> None:\n    nonlocal x\n",
            "`nonlocal` statements are planned for TyPy 1.0 (SMT-09)",
        ),
        (
            "def f(x: list[int]) -> None:\n    pass\n",
            "generic annotations such as `list[int]` are planned for TyPy 0.2 (TYP-06)",
        ),
        (
            "def f(x: int | None) -> None:\n    pass\n",
            "union annotations such as `int | None` are planned for TyPy 0.2 (TYP-06)",
        ),
        (
            "x.y = 1\n",
            "attributes and methods are planned for TyPy 0.2 (CLS-01)",
        ),
        (
            "x[0] = 1\n",
            "assignments to items are planned for TyPy 0.2 (DAT-07)",
        ),
        (
            "f()()\n",
            "calls of values that are not names are planned for TyPy 1.0 (FUN-05)",
        ),
        (
            "async def f() -> None:\n    pass\n",
            "`async` and `await` are planned for a release after TyPy 1.0",
        ),
        (
            "try:\n    pass\nexcept* E:\n    pass\n",
            "`except*` clauses are planned for a release after TyPy 1.0",
        ),
    ];
    for (source, expected) in cases {
        let found = diagnostics(source);
        assert!(
            found.iter().any(|(_, message)| message == expected),
            "{source:?} gave {found:?}"
        );
    }
}

#[test]
fn lazy_imports_name_their_release_with_floor_3_15() {
    let found = diagnostics_with_floor(
        "lazy import json\nlazy from os import path\n",
        PythonFloor::V3_15,
    );
    let messages: Vec<&str> = found.iter().map(|(_, message)| message.as_str()).collect();
    assert_eq!(
        messages,
        ["lazy imports are planned for TyPy 0.3 (MOD-06)"; 2]
    );
}

#[test]
fn excluded_features_say_they_cannot_be_compiled() {
    let cases = [
        (
            "if True:\n    def f() -> None:\n        pass\n",
            "functions defined inside `if`, `for` or `while` blocks cannot be compiled by TyPy with CPython's meaning",
        ),
        (
            "x = a @ b\n",
            "the `@` operator cannot be compiled by TyPy with CPython's meaning",
        ),
        (
            "x = ...\n",
            "`...` as a value cannot be compiled by TyPy with CPython's meaning",
        ),
        (
            "def f(x: 'int') -> None:\n    pass\n",
            "annotations written as strings cannot be compiled by TyPy with CPython's meaning",
        ),
    ];
    for (source, expected) in cases {
        let found = diagnostics(source);
        assert!(
            found
                .iter()
                .any(|(code, message)| code == "E0202" && message == expected),
            "{source:?} gave {found:?}"
        );
    }
}

#[test]
fn imaginary_literals_give_e0104() {
    assert_eq!(diagnostics("x = 3j\n")[0].0, "E0104");
}

#[test]
fn return_break_and_continue_out_of_place_are_syntax_errors() {
    let cases = [
        ("return\n", "`return` outside a function"),
        ("break\n", "`break` outside a loop"),
        ("continue\n", "`continue` outside a loop"),
        (
            "for i in range(3):\n    def f() -> None:\n        break\n",
            "`break` outside a loop",
        ),
    ];
    for (source, expected) in cases {
        let found = diagnostics(source);
        assert!(
            found
                .iter()
                .any(|(code, message)| code == "E0103" && message == expected),
            "{source:?} gave {found:?}"
        );
    }
}
