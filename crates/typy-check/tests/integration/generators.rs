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
            "def h() -> Generator[int, int, None]:\n    x = yield from g()\n",
            "E0201",
            "`yield from` is planned for TyPy 0.9 (SMT-11)",
            "SMT-11",
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
