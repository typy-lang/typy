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
