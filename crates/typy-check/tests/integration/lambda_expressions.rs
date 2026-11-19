//! `lambda` expressions, which take their types from the `Callable` expected (EXP-14).

use crate::common::{app, dump, messages};

#[test]
fn exp_14_a_lambda_is_accepted_where_a_callable_is_expected() {
    for source in [
        "from typing import Callable\n\nf: Callable[[int], int] = lambda x: x + 1\n",
        "from typing import Callable\n\n\ndef apply(f: Callable[[str], int]) -> int:\n    \
         return f(\"ab\")\n\n\nprint(apply(lambda text: len(text)))\n",
        "from typing import Callable\n\n\ndef make(n: int) -> Callable[[], int]:\n    \
         return lambda: n\n",
        "words = [\"b\", \"a\"]\nwords.sort(key=lambda word: word)\nprint(sorted(words, \
         key=lambda word: -len(word)), min(words, key=lambda word: word))\n",
        "answer = lambda: 42\nprint(answer())\n",
        "from typing import Callable\n\n\ndef pick(flag: bool) -> Callable[[int], int]:\n    \
         return (lambda x: x) if flag else (lambda x: -x)\n",
        "from typing import Callable\n\nfs: list[Callable[[int], int]] = [lambda x: x]\n",
        "from typing import Callable\n\nf: Callable[[int], int] | None = lambda x: x\n",
        "def f() -> list[int]:\n    made = [lambda: i for i in range(3)]\n    \
         return [each() for each in made]\n",
    ] {
        assert_eq!(app(source), Vec::<String>::new(), "{source}");
    }
}

#[test]
fn exp_14_the_typed_tree_makes_a_function_object_with_its_captures() {
    let found = dump(
        "from typing import Callable\n\n\ndef make(n: int) -> Callable[[int], int]:\n    \
         return lambda x: x + n\n",
    );
    for line in [
        "Lambda make.<locals>.<lambda> : Callable[[int], int]",
        "Local n : int",
    ] {
        assert!(found.contains(line), "{line} missing from\n{found}");
    }
}

#[test]
fn exp_14_a_lambda_without_the_types_it_needs_is_refused() {
    let cases = [
        (
            "f = lambda x: x\n",
            "E0203",
            "`lambda` parameter `x` has no type: a `lambda` takes the types of its parameters \
             from the `Callable` its context expects",
        ),
        (
            "from typing import Callable\n\nf: Callable[[int], int] = lambda x, y: x\n",
            "E0203",
            "`lambda` parameter `y` has no type: the `Callable` expected here passes 1 argument",
        ),
        (
            "from typing import Callable\n\nf: Callable[[], int] = lambda x: 1\n",
            "E0203",
            "`lambda` parameter `x` has no type: the `Callable` expected here passes 0 arguments",
        ),
        (
            "from typing import Callable\n\nf: Callable[[int, int], int] = lambda x: x\n",
            "E0402",
            "`f` was declared as Callable[[int, int], int] and is assigned a Callable[[int], int]",
        ),
        (
            "from typing import Callable\n\nf: Callable[[int], str] = lambda x: x + 1\n",
            "E0405",
            "this returns an int, but the function returns str",
        ),
        (
            "from typing import Callable\n\nf: Callable[[int], int] = lambda *, x: x\n",
            "E0202",
            "the `lambda` takes `x` by keyword only, and a `Callable` passes every argument by \
             position",
        ),
        (
            "from typing import Callable\n\n\ndef g[T](value: T) -> Callable[[], T]:\n    \
             return lambda: value\n",
            "E0201",
            "a `lambda` inside a generic function is planned for TyPy 0.10 (FUN-10)",
        ),
        (
            "def f() -> int:\n    g = lambda: x\n    x = 1\n    return g()\n",
            "E0306",
            "this `lambda` reads `x`, which has no type yet where it stands",
        ),
        (
            "words = [\"a\"]\nprint(sorted(words, key=lambda word: {word: 1}))\n",
            "E0407",
            "dict[str, int] and dict[str, int] cannot be compared",
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
