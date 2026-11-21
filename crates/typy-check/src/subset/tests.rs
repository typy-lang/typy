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
            "@d\nclass C:\n    pass\n",
            "class decorators other than `@dataclass` are planned for TyPy 1.0 (CLS-38)",
        ),
        (
            "@decorated\nclass C[T]:\n    pass\n",
            "class decorators other than `@dataclass` are planned for TyPy 1.0 (CLS-38)",
        ),
        (
            "x = [i async for i in y]\n",
            "`async` and `await` are planned for a release after TyPy 1.0",
        ),
        (
            "def f[*Ts](x: int) -> int:\n    return x\n",
            "`*Ts` and `**P` type parameters are planned for TyPy 0.13 (GEN-06)",
        ),
        (
            "class C:\n    @d\n    def m(self) -> None:\n        pass\n",
            "decorators of methods are planned for TyPy 0.14 (FUN-15)",
        ),
        (
            "class C:\n    @staticmethod\n    @d\n    def m() -> None:\n        pass\n",
            "decorators of methods are planned for TyPy 0.14 (FUN-15)",
        ),
        (
            "f = lambda *a: 0\n",
            "`*args` and `**kwargs` in a `lambda` are planned for TyPy 0.11 (FUN-19)",
        ),
        (
            "def f(a: int = g()) -> None:\n    pass\n",
            "default values other than constants are planned for TyPy 1.0 (EXP-20)",
        ),
        (
            "def f() -> None:\n    print((yield 1))\n",
            "`yield` and `yield from` inside a larger expression are planned for TyPy 1.0 \
             (FUN-12)",
        ),
        (
            "def f() -> None:\n    g = lambda: (yield 1)\n",
            "generator `lambda` expressions are planned for TyPy 1.0 (FUN-12)",
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
            "generic annotations other than `list`, `dict`, `set`, `tuple`, `type`, `Iterator`, `Iterable` and their `typing` names, such as `frozenset[int]`, cannot be compiled by TyPy with CPython's meaning",
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
fn nonlocal_where_cpython_rejects_it_is_a_syntax_error() {
    let cases = [
        (
            "nonlocal x\n",
            Some("nonlocal declaration not allowed at module level"),
        ),
        (
            "def f() -> None:\n    nonlocal x\n",
            Some("no binding for nonlocal 'x' found"),
        ),
        (
            "class C:\n    def m(self) -> None:\n        nonlocal x\n",
            Some("no binding for nonlocal 'x' found"),
        ),
        (
            "def f(x: int) -> None:\n    def g(x: int) -> None:\n        nonlocal x\n",
            Some("name 'x' is parameter and nonlocal"),
        ),
        (
            "def f() -> None:\n    x = 1\n    def g() -> None:\n        print(x)\n        \
             nonlocal x\n",
            Some("name 'x' is used prior to nonlocal declaration"),
        ),
        (
            "def f() -> None:\n    x = 1\n    def g() -> None:\n        x = 2\n        \
             nonlocal x\n",
            Some("name 'x' is assigned to before nonlocal declaration"),
        ),
        (
            "def f() -> None:\n    x = 1\n    def g() -> None:\n        x: int = 2\n        \
             nonlocal x\n",
            Some("annotated name 'x' can't be nonlocal"),
        ),
        (
            "def f() -> None:\n    x = 1\n    def g() -> None:\n        nonlocal x\n        \
             x: int = 2\n",
            Some("annotated name 'x' can't be nonlocal"),
        ),
        (
            "def f() -> None:\n    x = 1\n    def g() -> None:\n        global x\n        \
             nonlocal x\n",
            Some("name 'x' is nonlocal and global"),
        ),
        (
            "def f() -> None:\n    x = 1\n    def g() -> None:\n        nonlocal x\n        \
             global x\n",
            Some("name 'x' is nonlocal and global"),
        ),
        (
            "def f() -> None:\n    x = 1\n    def g() -> None:\n        nonlocal x\n        \
             x = 2\n",
            None,
        ),
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

#[test]
fn yield_where_cpython_rejects_it_is_a_syntax_error() {
    let cases = [
        ("yield 1\n", Some("'yield' outside function")),
        ("print((yield 1))\n", Some("'yield' outside function")),
        (
            "def f() -> list[int]:\n    return [(yield x) for x in []]\n",
            Some("'yield' inside list comprehension"),
        ),
        (
            "def f() -> set[int]:\n    return {(yield x) for x in []}\n",
            Some("'yield' inside set comprehension"),
        ),
        (
            "def f() -> dict[int, int]:\n    return {x: (yield x) for x in []}\n",
            Some("'yield' inside dict comprehension"),
        ),
        ("def f() -> None:\n    yield 1\n    yield\n", None),
        (
            "def f() -> None:\n    x = yield 1\n    x: int = yield 1\n",
            None,
        ),
        (
            "def f() -> None:\n    yield from []\n    x = yield from []\n",
            None,
        ),
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

#[test]
fn assigning_debug_or_binding_a_name_twice_in_a_pattern_is_a_syntax_error() {
    let cases = [
        ("print((__debug__ := 1))\n", "cannot assign to __debug__"),
        (
            "match 1:\n    case __debug__:\n        pass\n",
            "cannot assign to __debug__",
        ),
        (
            "match [1, 2]:\n    case [x, x]:\n        pass\n",
            "multiple assignments to name 'x' in pattern",
        ),
        // The names an alternative binds count once, against the rest of the pattern.
        (
            "match [1, [2]]:\n    case [x, ([x] | [x])]:\n        pass\n",
            "multiple assignments to name 'x' in pattern",
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
fn an_assertion_of_a_tuple_with_items_is_w0301() {
    assert_eq!(
        diagnostics("x = 1\nassert (x, 'message')\nassert ()\nassert x, 'message'\n"),
        [(
            "W0301".to_owned(),
            "this assertion is always true, since a tuple with items is true".to_owned()
        )]
    );
}

#[test]
fn a_generic_class_is_named_with_its_type_arguments_in_annotations_and_calls() {
    let source =
        "class Box[T]:\n    pass\nx: Box[int] = Box[int]()\ny: Box[tuple[int, str]] = Box()\n";
    assert_eq!(diagnostics(source), Vec::new());
    // The call is the checker's to refuse, as a call of any other value is.
    let other = diagnostics("x: frozenset[int] = 1\nprint(frozenset[int]())\n");
    assert_eq!(other.len(), 1, "{other:?}");
}

#[test]
fn a_keyword_repeated_in_a_call_is_a_syntax_error() {
    for (source, expected) in [
        (
            "print(1, sep=\"\", sep=\"\")\n",
            "keyword argument repeated: sep",
        ),
        (
            "import math\nprint(math.prod([1], start=1, start=2))\n",
            "keyword argument repeated: start",
        ),
        (
            "def f(a: int) -> None:\n    pass\nf(a=1, a=2)\n",
            "keyword argument repeated: a",
        ),
    ] {
        let found = diagnostics(source);
        assert_eq!(
            found,
            [("E0103".to_owned(), expected.to_owned())],
            "{source:?}"
        );
    }
}
