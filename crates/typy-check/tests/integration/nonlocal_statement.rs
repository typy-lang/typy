//! `nonlocal` in nested functions (SMT-09, ADR-0061).

use crate::common::{app, dump, messages};

#[test]
fn smt_09_a_nested_function_assigns_a_variable_of_a_function_around_it() {
    for source in [
        "def counter() -> int:\n    count = 0\n    def bump() -> None:\n        nonlocal count\n        \
         count += 1\n    bump()\n    return count\n",
        "def outer() -> int:\n    total = 1\n    def middle() -> None:\n        \
         def inner() -> None:\n            nonlocal total\n            total = total * 2\n        \
         inner()\n    middle()\n    return total\n",
        "def f() -> list[int]:\n    out: list[int] = []\n    for i in range(3):\n        \
         def bump() -> None:\n            nonlocal i\n            i = i * 10\n        bump()\n        \
         out.append(i)\n    return out\n",
        "def f() -> int:\n    x = 1\n    def g() -> None:\n        nonlocal x, x\n        \
         nonlocal x\n        x = 2\n    g()\n    return x\n",
        "def f() -> int:\n    error = 0\n    try:\n        return error\n    \
         except ValueError as error:\n        return 1\n",
        "from typing import Callable\n\ni = 0\n\n\ndef f() -> list[int]:\n    global i\n    \
         made: list[Callable[[], int]] = [lambda: i for i in range(3)]\n    \
         return [each() for each in made]\n",
        "def f(label: str) -> str:\n    def rename() -> None:\n        nonlocal label\n        \
         for label in [\"a\", \"b\"]:\n            pass\n    rename()\n    return label\n",
    ] {
        assert_eq!(app(source), Vec::<String>::new(), "{source}");
    }
}

#[test]
fn smt_09_the_typed_tree_reads_and_assigns_the_captured_cell() {
    let found = dump(
        "def counter() -> int:\n    count = 0\n    def bump() -> int:\n        nonlocal count\n        \
         count = count + 1\n        return count\n    return bump()\n",
    );
    for line in [
        "FunctionDef counter.<locals>.bump as bump",
        "CellObject count : int",
        "CellRead count : int",
    ] {
        assert!(found.contains(line), "{line} missing from\n{found}");
    }
}

#[test]
fn smt_09_nonlocal_where_typy_cannot_keep_cpythons_meaning_is_refused() {
    let cases = [
        (
            "def outer() -> None:\n    def inner() -> None:\n        nonlocal missing\n        \
             missing = 1\n",
            "E0103",
            "no binding for nonlocal 'missing' found",
        ),
        (
            "def f() -> None:\n    x = 1\n    def g() -> None:\n        nonlocal x\n        \
             x = \"text\"\n",
            "E0402",
            "`x` was declared as int and is assigned a str",
        ),
        (
            "def f() -> int:\n    def helper() -> int:\n        return 1\n    \
             def other() -> int:\n        return 2\n    def g() -> None:\n        \
             nonlocal helper\n        helper = other\n    g()\n    return helper()\n",
            "E0202",
            "`helper` is a function and is also assigned as a variable",
        ),
        (
            "def f() -> None:\n    x = ValueError(\"a\")\n    def g() -> None:\n        nonlocal x\n        \
             try:\n            pass\n        except ValueError as x:\n            pass\n",
            "E0202",
            "`x` is declared `nonlocal`, and `except ... as x` deletes it when the clause ends",
        ),
        (
            "def f() -> int:\n    x = 1\n    def g() -> int:\n        return x\n    try:\n        \
             pass\n    except ValueError as x:\n        pass\n    return g()\n",
            "E0202",
            "`x` is shared with a nested function, and `except ... as x` deletes it when the \
             clause ends",
        ),
        (
            "x: Exception = ValueError(\"a\")\n\n\ndef g() -> None:\n    global x\n    try:\n        \
             pass\n    except ValueError as x:\n        pass\n",
            "E0202",
            "`x` is declared `global`, and `except ... as x` deletes it when the clause ends",
        ),
        // A call may assign the variable, so no test narrows it.
        (
            "def f() -> int:\n    value: int | None = 1\n    def clear() -> None:\n        \
             nonlocal value\n        value = None\n    if value is not None:\n        clear()\n        \
             return value + 1\n    return 0\n",
            "E0401",
            "`+` is not defined for int | None and int",
        ),
        (
            "def f() -> int:\n    value: int | None = 1\n    def g() -> int:\n        \
             nonlocal value\n        value = 2\n        return value + 1\n    return g()\n",
            "E0401",
            "`+` is not defined for int | None and int",
        ),
    ];
    for (source, code, message) in cases {
        let found = messages(source);
        assert!(
            found
                .iter()
                .any(|(each, text, _)| each == code && text == message),
            "{source}: {found:?}"
        );
    }
}

#[test]
fn smt_09_an_except_clause_variable_a_function_inside_the_clause_reads_is_accepted() {
    let source = "def f() -> str:\n    try:\n        raise ValueError(\"v\")\n    \
                  except ValueError as error:\n        def show() -> str:\n            \
                  return str(error)\n        return show()\n    return \"\"\n";
    assert_eq!(app(source), Vec::<String>::new());
}
