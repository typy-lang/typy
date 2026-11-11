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
class C(object):
    \"\"\"A class.\"\"\"
    x: int
    pass
    def m(self, y: 'C') -> None:
        self.x = y.x
";
    assert_eq!(diagnostics(source), []);
}

#[test]
#[allow(clippy::too_many_lines, reason = "one row per planned feature")]
fn later_features_name_their_release_and_rule() {
    let cases = [
        (
            "class C(module.B):\n    pass\n",
            "base classes other than `object` are planned for TyPy 0.4 (CLS-04)",
        ),
        (
            "@d\nclass C:\n    pass\n",
            "class decorators are planned for TyPy 0.4 (CLS-05)",
        ),
        (
            "class C[T]:\n    pass\n",
            "type parameters are planned for TyPy 0.4 (GEN-01)",
        ),
        (
            "class C:\n    x = 1\n",
            "class variables are planned for TyPy 0.4 (CLS-11)",
        ),
        (
            "class C:\n    x: int = 1\n",
            "class variables are planned for TyPy 0.4 (CLS-11)",
        ),
        (
            "x = {**y}\n",
            "unpacking with `**` are planned for TyPy 1.0 (EXP-17)",
        ),
        (
            "x = [i async for i in y]\n",
            "`async` and `await` are planned for a release after TyPy 1.0",
        ),
        (
            "x = (i for i in range(3))\n",
            "generator expressions are planned for TyPy 1.0 (EXP-15)",
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
            "def f(x: int | str) -> None:\n    pass\n",
            "union annotations such as `int | str` are planned for TyPy 0.4 (TYP-14)",
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
fn lazy_imports_at_module_level_pass_with_floor_3_15() {
    let source =
        "lazy import json\nif True:\n    lazy from os import path\nwith x:\n    lazy import a\n";
    assert_eq!(diagnostics_with_floor(source, PythonFloor::V3_15), []);
}

#[test]
fn lazy_imports_cpython_refuses_give_its_messages() {
    let cases = [
        (
            "def f() -> None:\n    lazy import json\n",
            "lazy import not allowed inside functions",
        ),
        (
            "try:\n    lazy import json\nexcept ImportError:\n    pass\n",
            "lazy import not allowed inside try/except blocks",
        ),
        (
            "lazy from os import *\n",
            "lazy from ... import * is not allowed",
        ),
        (
            "lazy from __future__ import annotations\n",
            "lazy from __future__ import is not allowed",
        ),
    ];
    for (source, message) in cases {
        let found = diagnostics_with_floor(source, PythonFloor::V3_15);
        assert_eq!(
            found,
            [("E0103".to_owned(), message.to_owned())],
            "{source}"
        );
    }
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
            "def f(x: frozenset[int]) -> None:\n    pass\n",
            "generic annotations other than `list`, `dict`, `set`, `tuple`, `type`, `Iterator` and `Iterable`, such as `frozenset[int]`, cannot be compiled by TyPy with CPython's meaning",
        ),
        (
            "x = ...\n",
            "`...` as a value cannot be compiled by TyPy with CPython's meaning",
        ),
        (
            "def f(x: 'int(') -> None:\n    pass\n",
            "annotations written as strings cannot be compiled by TyPy with CPython's meaning",
        ),
        (
            "def f() -> None:\n    class C:\n        pass\n",
            "classes defined inside functions, classes or blocks cannot be compiled by TyPy with CPython's meaning",
        ),
        (
            "class C(metaclass=M):\n    pass\n",
            "class keywords such as `metaclass` cannot be compiled by TyPy with CPython's meaning",
        ),
        (
            "class C:\n    if x:\n        pass\n",
            "statements other than attribute annotations and methods in a class body cannot be compiled by TyPy with CPython's meaning",
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

#[test]
fn global_after_a_use_an_assignment_or_an_annotation_is_a_syntax_error() {
    let cases = [
        (
            "def f() -> None:\n    print(x)\n    global x\n",
            Some("name 'x' is used prior to global declaration"),
        ),
        (
            "def f() -> None:\n    x = 1\n    global x\n",
            Some("name 'x' is assigned to before global declaration"),
        ),
        (
            "def f(x: int) -> None:\n    global x\n",
            Some("name 'x' is parameter and global"),
        ),
        (
            "def f() -> None:\n    x: int = 1\n    global x\n",
            Some("annotated name 'x' can't be global"),
        ),
        (
            "def f() -> None:\n    global x\n    x: int = 1\n",
            Some("annotated name 'x' can't be global"),
        ),
        ("def f() -> None:\n    global x\n    x = 1\n", None),
        ("global x\nx = 1\n", None),
    ];
    for (source, expected) in cases {
        let found: Vec<(String, String)> = diagnostics(source)
            .into_iter()
            .filter(|(code, _)| code == "E0103")
            .collect();
        match expected {
            Some(message) => assert!(
                found.iter().any(|(_, found)| found == message),
                "{source:?} gave {found:?}"
            ),
            None => assert!(found.is_empty(), "{source:?} gave {found:?}"),
        }
    }
}
