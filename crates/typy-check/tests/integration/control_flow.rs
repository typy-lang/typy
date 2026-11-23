//! Narrowing, patterns, comprehension targets, `nonlocal` variables and imports of the module
//! itself.

use crate::common::{app, messages};

#[test]
fn narrowing_patterns_and_imports_are_accepted() {
    let accepted = [
        "from collections.abc import Iterable\n\n\ndef f(x: Iterable[int]) -> int:\n    \
        if isinstance(x, list):\n        return 1\n    return 0\n\n\nprint(f([1]))\n",
        "class Base:\n    pass\n\n\nclass Box[T](Base):\n    \
        def __init__(self, v: T) -> None:\n        self.v = v\n\n\n\
        def f(x: Base | str) -> int:\n    if isinstance(x, Box):\n        return 1\n    \
        return 0\n\n\nprint(f(\"a\"))\n",
        "from collections.abc import Generator, Iterator\n\n\ndef f(name: str) -> None:\n    \
        with open(name) as handle:\n        \
        print(isinstance(handle, Generator), isinstance(handle, Iterator))\n\n\n\
        f(\"/etc/hostname\")\n",
        "class A:\n    __match_args__ = [\"x\"]\n\n    def __init__(self) -> None:\n        \
        self.x = 1\n\n\nmatch A():\n    case A():\n        print(\"a\")\n",
        "def f(t: tuple[int, str]) -> None:\n    match t:\n        case (a, *rest, b):\n            \
        print(a, rest, b)\n\n\nf((1, \"x\"))\n",
        "def f(flag: bool) -> int:\n    if flag:\n        raise ValueError(\"no\")\n        \
        n = 1\n    return 0\n\n\nprint(f(False))\n",
        "def outer() -> None:\n    def mid() -> int:\n        x = 1\n\n        \
        def inner() -> None:\n            nonlocal x\n            x = 2\n\n        inner()\n        \
        return x\n\n    print(mid())\n\n\nouter()\n",
        "from collections.abc import Iterable, Iterator\n\n\nclass Odd:\n    \
        def __iter__(self) -> Iterator[int]:\n        return iter([1])\n\n    \
        def __next__(self) -> int:\n        return 1\n\n\n\
        def total(xs: Iterable[int]) -> int:\n    return sum(xs)\n\n\nprint(total(Odd()))\n",
        "type Num = int\nfrom app import Num\n\nprint(Num)\n",
    ];
    for source in accepted {
        assert_eq!(app(source), Vec::<String>::new(), "{source}");
    }
}

#[test]
fn a_pattern_a_target_or_an_import_cpython_refuses_is_refused() {
    let refused = [
        (
            "match 5:\n    case int(a, b):\n        print(a, b)\n",
            "E0401",
            "int() accepts 1 positional sub-pattern (2 given): CPython raises `TypeError`",
        ),
        (
            "from typing import Final\n\nN: Final = 0\nv: int | str = \"a\"\nmatch v:\n    \
            case str() as N:\n        print(N)\n    case _:\n        pass\n",
            "E0414",
            "`N` is declared `Final`, so it is not assigned again",
        ),
        (
            "from typing import Final\n\nREST: Final = 0\nv: dict[str, int] = {\"a\": 1}\n\
            match v:\n    case {\"a\": 1, **REST}:\n        print(REST)\n    case _:\n        \
            pass\n",
            "E0414",
            "`REST` is declared `Final`, so it is not assigned again",
        ),
        (
            "from typing import Final\n\nN: Final = 0\nif (N := 5):\n    print(N)\n",
            "E0414",
            "`N` is declared `Final`, so it is not assigned again",
        ),
        (
            "t = (1, 2)\nprint([1 for t[0] in [3]])\n",
            "E0401",
            "an item of a tuple[int, int] cannot be assigned",
        ),
        (
            "print([i for i in range(\"a\".nope())])\n",
            "E0412",
            "a str has no method `nope` in TyPy",
        ),
        (
            "import math\n\nmath.pi += 1.0\n",
            "E0401",
            "an augmented assignment cannot assign a float read this way",
        ),
        (
            "import app\n\napp.x = 1\n",
            "E0425",
            "module `app` has no name `x`",
        ),
        (
            "from collections.abc import Awaitable\n",
            "E0425",
            "TyPy's `collections.abc` gives `Iterator`, `Iterable`, `Generator`, `Callable`, \
            `Sequence`, `MutableSequence`, `Mapping`, `MutableMapping`, `Set`, `MutableSet`, \
            `Collection`, `Container`, `Reversible`, `KeysView`, `ValuesView` and `ItemsView`, \
            not `Awaitable`",
        ),
        (
            "from typing import Awaitable\n",
            "E0425",
            "TyPy's `typing` gives `Iterator`, `Iterable`, `Generator`, `Callable`, \
            `Optional`, `Union`, `List`, `Dict`, `Set`, `Tuple`, `Type`, `Final`, `ClassVar`, \
            `Protocol`, `NamedTuple`, `Literal`, `overload`, `Self`, `TypeGuard`, `TypeIs`, `Sequence`, `MutableSequence`, `Mapping`, \
            `MutableMapping`, `AbstractSet`, `MutableSet`, `Collection`, `Container`, \
            `Reversible`, `KeysView`, `ValuesView`, `ItemsView` and `cast`, not `Awaitable`",
        ),
        (
            "from dataclasses import asdict\n",
            "E0425",
            "TyPy's `dataclasses` gives `dataclass` and `field`, not `asdict`",
        ),
    ];
    for (source, code, message) in refused {
        let found = messages(source);
        assert!(
            found
                .iter()
                .any(|(each, text, _)| each == code && text == message),
            "{source}\n{found:?}"
        );
    }
}
