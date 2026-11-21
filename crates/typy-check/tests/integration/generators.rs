//! Generator functions (SMT-11, FUN-07, ADR-0063).

use crate::common::{app, dump, messages};

#[test]
fn fun_07_a_generator_function_takes_each_annotation_python_s_typing_gives() {
    for source in [
        "from collections.abc import Iterator\n\n\ndef count(n: int) -> Iterator[int]:\n    \
         i = 0\n    while i < n:\n        yield i\n        i += 1\n",
        "from typing import Iterable\n\n\ndef words() -> Iterable[str]:\n    yield \"a\"\n",
        "from collections.abc import Generator\n\n\ndef pairs() -> Generator[tuple[int, str], \
         None, None]:\n    yield (1, \"a\")\n    return\n",
        "from collections.abc import Iterator\n\n\ndef nothing() -> Iterator[None]:\n    yield\n",
        "from collections.abc import Iterator\n\n\ndef f() -> Iterator[int]:\n    \
         yield 1\n\n\nprint(list(f()), sum(f()), next(f()), [x for x in f()])\n",
        "from collections.abc import Generator, Iterator\n\n\ndef g() -> Generator[int, None, \
         None]:\n    yield 1\n\n\nit: Iterator[int] = g()\nfor x in g():\n    print(x)\n",
    ] {
        assert_eq!(app(source), Vec::<String>::new(), "{source}");
    }
}

#[test]
fn smt_11_the_typed_tree_yields_each_value() {
    let found = dump(
        "from collections.abc import Iterator\n\n\ndef count(n: int) -> Iterator[int]:\n    \
         yield n\n",
    );
    for line in ["Yield", "Local n : int"] {
        assert!(found.contains(line), "{line} missing from\n{found}");
    }
}

#[test]
fn fun_07_a_generator_of_another_annotation_or_value_is_refused() {
    let cases = [
        (
            "def f() -> int:\n    yield 1\n",
            "E0405",
            "`f` has a `yield`, so its call makes a generator, and it is annotated as \
             returning an int",
        ),
        (
            "from collections.abc import Iterator\n\n\ndef f() -> Iterator[int]:\n    \
             yield \"a\"\n",
            "E0405",
            "this yields a str, but the generator yields int",
        ),
        (
            "from collections.abc import Generator\n\n\ndef f() -> Generator[int]:\n    \
             yield 1\n",
            "E0408",
            "a `Generator` annotation names what it yields, what it is sent and what it \
             returns, such as `Generator[int, None, None]`",
        ),
        (
            "from collections.abc import Generator\n\nx: Generator | None = None\n",
            "E0408",
            "a `Generator` annotation names what it yields, what it is sent and what it \
             returns, such as `Generator[int, None, None]`",
        ),
        (
            "from collections.abc import Iterator\n\n\ndef f() -> Iterator[int]:\n    \
             yield 1\n\n\nprint(len(f()))\n",
            "E0404",
            "the argument of `len` must be a str, and this is an Iterator[int]",
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
fn smt_11_an_assigned_yield_receives_what_send_gives_or_none() {
    for source in [
        "from collections.abc import Generator\n\n\ndef total() -> Generator[int, int, None]:\n    \
         count = 0\n    while True:\n        received = yield count\n        if received is not \
         None:\n            count += received\n",
        "from collections.abc import Generator\n\n\ndef echo() -> Generator[str, str | None, \
         None]:\n    reply: str | None = yield \"ready\"\n    while reply is not None:\n        \
         reply = yield reply\n",
        "from collections.abc import Iterator\n\n\ndef ticks() -> Iterator[None]:\n    \
         nothing = yield\n    print(nothing)\n",
        "from collections.abc import Generator\n\n\ndef pairs() -> Generator[int, tuple[int, \
         str], None]:\n    got = yield 0\n    if got is not None:\n        number, name = got\n        \
         print(number, name)\n",
    ] {
        assert_eq!(app(source), Vec::<String>::new(), "{source}");
    }
    let found = dump(
        "from collections.abc import Generator\n\n\ndef total() -> Generator[int, int, None]:\n    \
         received = yield 0\n    print(received)\n",
    );
    for line in [
        "Yield",
        "Assign received: int | None",
        "Received : int | None",
    ] {
        assert!(found.contains(line), "{line} missing from\n{found}");
    }
}

#[test]
fn fun_07_send_throw_and_close_take_python_s_arguments() {
    let source = "from collections.abc import Generator\n\n\ndef g() -> Generator[int, str, \
                  None]:\n    yield 1\n\n\nit = g()\nfirst: int = next(it)\nsecond: int = \
                  it.send(\"a\")\nthird: int = it.throw(ValueError)\nfourth: int = \
                  it.throw(KeyError(\"k\"))\nit.close()\n";
    assert_eq!(app(source), Vec::<String>::new());
    let found = dump(source);
    for line in [
        "Method send : int",
        "Method throw : int",
        "Method close : None",
    ] {
        assert!(found.contains(line), "{line} missing from\n{found}");
    }
}

#[test]
fn fun_07_a_generator_s_methods_and_what_its_yield_receives_are_checked() {
    let generator = "from collections.abc import Generator, Iterator\n\n\ndef g() -> \
                     Generator[int, int, None]:\n    yield 1\n\n\ndef i() -> Iterator[int]:\n    \
                     yield 1\n\n\n";
    let cases = [
        (
            "i().close()\n",
            "E0412",
            "an Iterator[int] has no method `close` in TyPy",
            "FUN-07",
        ),
        (
            "g().send(None)\n",
            "E0404",
            "`value` of `send` must be an int, and this is None",
            "TYP-01",
        ),
        (
            "g().send(\"a\")\n",
            "E0404",
            "`value` of `send` must be an int, and this is a str",
            "TYP-01",
        ),
        (
            "g().throw(5)\n",
            "E0415",
            "`throw` takes an exception or an exception class, and this is an int",
            "SMT-06",
        ),
        (
            "def h() -> Generator[int, int, None]:\n    x: int = yield 1\n    print(x)\n",
            "E0402",
            "`x` was declared as int and is assigned an int | None",
            "TYP-03",
        ),
        (
            "def h() -> Iterator[int]:\n    yield from 5\n",
            "E0411",
            "an int cannot be iterated over",
            "CST-03",
        ),
        (
            "def h() -> Iterator[int]:\n    yield from [\"a\"]\n",
            "E0405",
            "this yields a str, but the generator yields int",
            "FUN-07",
        ),
        (
            "def h() -> Generator[int, str, None]:\n    yield from g()\n",
            "E0405",
            "the generator is sent a str, and the one it delegates to receives an int",
            "FUN-07",
        ),
        (
            "def t() -> Generator[int, tuple[int, int] | None, None]:\n    yield 1\n\n\n\
             def h() -> Generator[int, tuple[int, int], None]:\n    yield from t()\n",
            "E0405",
            "the generator is sent a tuple[int, int], and the one it delegates to receives a \
             tuple[int, int] | None",
            "FUN-07",
        ),
        (
            "def h() -> Generator[int, None, str]:\n    yield 1\n    return 5\n",
            "E0405",
            "this returns an int, but the generator returns str",
            "FUN-07",
        ),
        (
            "def h() -> Generator[int, None, str]:\n    yield 1\n",
            "E0405",
            "`h` can reach its end without returning a str",
            "TYP-01",
        ),
        (
            "def h() -> Iterator[int]:\n    yield 1\n    return 5\n",
            "E0405",
            "this returns an int, but the generator returns None",
            "FUN-07",
        ),
    ];
    for (code_line, code, message, rule) in cases {
        let source = format!("{generator}{code_line}");
        let found = messages(&source);
        assert!(
            found
                .iter()
                .any(|(each, text, of)| each == code && text == message && of == rule),
            "{source}: {found:?}"
        );
    }
}

#[test]
fn smt_11_yield_from_delegates_and_gives_what_the_generator_returns() {
    for source in [
        "from collections.abc import Generator, Iterator\n\n\ndef inner() -> Generator[int, \
         None, str]:\n    yield 1\n    return \"done\"\n\n\ndef outer() -> Generator[int | \
         None, None, int]:\n    result: str = yield from inner()\n    yield from [2, 3]\n    \
         yield from range(4)\n    yield from \"ab\".encode()\n    return len(result)\n",
        "from collections.abc import Iterator\n\n\ndef letters() -> Iterator[str]:\n    \
         yield from \"abc\"\n    nothing = yield from {\"k\": 1}\n    print(nothing)\n",
        "from collections.abc import Generator\n\n\ndef inner() -> Generator[int, int | None, \
         None]:\n    yield 1\n\n\ndef outer() -> Generator[int, int, None]:\n    yield from \
         inner()\n",
        // `next()` resumes the outer generator with `None`, which an `int` one receives as it
        // receives `None` from its own `next()`.
        "from collections.abc import Generator\n\n\ndef inner() -> Generator[int, int, \
         None]:\n    received = yield 1\n    print(received)\n\n\ndef outer() -> \
         Generator[int, int | None, None]:\n    yield from inner()\n",
    ] {
        assert_eq!(app(source), Vec::<String>::new(), "{source}");
    }
    let found = dump(
        "from collections.abc import Generator\n\n\ndef inner() -> Generator[int, None, str]:\n    \
         yield 1\n    return \"done\"\n\n\ndef outer() -> Generator[int, None, None]:\n    \
         result = yield from inner()\n    print(result)\n",
    );
    for line in [
        "YieldFrom",
        "DelegatedItem : int",
        "Assign result: str",
        "DelegatedResult : str",
        "Return",
    ] {
        assert!(found.contains(line), "{line} missing from\n{found}");
    }
    // A `yield from` statement receives nothing back.
    let found = dump(
        "from collections.abc import Generator\n\n\ndef inner() -> Generator[int, None, None]:\n    \
         yield 1\n\n\ndef outer() -> Generator[int, None, None]:\n    yield from inner()\n",
    );
    assert!(found.contains("DelegatedItem : int"), "{found}");
    assert!(!found.contains("DelegatedResult"), "{found}");
}

#[test]
fn fun_07_yield_from_with_an_unknown_send_type_gives_one_error() {
    for (outer, inner) in [("Missing", "int"), ("int", "Missing")] {
        let source = format!(
            "from collections.abc import Generator\n\n\ndef inner() -> Generator[int, {inner}, \
             None]:\n    received = yield 1\n    print(received)\n\n\n\
             def outer() -> Generator[int, {outer}, None]:\n    yield from inner()\n"
        );
        assert_eq!(app(&source), ["E0205"], "{source}");
    }
}

#[test]
fn exp_15_a_generator_expression_is_a_nested_generator_of_its_first_iterator() {
    for source in [
        "print(sum(value * value for value in range(3)))\n",
        "words = [\"a\"]\nprint(\",\".join(word.upper() for word in words))\n",
        "from collections.abc import Iterator\n\nvalues: Iterator[int | None] = (value for value in \
         [1])\nprint(list(values))\n",
        "def scaled(factor: int) -> list[int]:\n    return list(value * factor for value in \
         range(3) if value)\n",
        "from collections.abc import Generator\n\nmade: Generator[int | None, None, None] = \
         (value for value in range(2))\nprint(list(made))\n",
    ] {
        assert_eq!(app(source), Vec::<String>::new(), "{source}");
    }
    let found = dump("doubled = (value * 2 for value in [1, 2])\n");
    for line in [
        "CallValue : Generator[int, None, None]",
        "Iter : Iterator[int]",
        "<genexpr>",
    ] {
        assert!(found.contains(line), "{line} missing from\n{found}");
    }
}

#[test]
fn exp_15_a_generator_expression_refuses_what_it_cannot_compile() {
    let cases = [
        (
            "values = (v for v in 5)\n",
            "E0411",
            "an int cannot be iterated over",
        ),
        (
            "def f() -> None:\n    total = 0\n    values = (total := total + v for v in \
             range(3))\n    print(list(values))\n",
            "E0201",
            "an assignment expression inside a generator expression is planned for TyPy 1.0 \
             (EXP-18)",
        ),
        (
            "def f() -> None:\n    values = ((yield v) for v in range(3))\n    print(values)\n",
            "E0103",
            "'yield' inside generator expression",
        ),
        (
            "items: list[int] = (v for v in range(3))\n",
            "E0402",
            "`items` was declared as list[int] and is assigned a Generator[int, None, None]",
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
