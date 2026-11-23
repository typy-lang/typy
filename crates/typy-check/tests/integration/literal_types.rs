//! `typing.Literal` types: a constant passes for its own literal type, a value of another
//! passes only once a test narrows it, and `==`, `in` and `match` narrow a variable to the
//! constants it is compared with (TYP-28, ADR-0110).

use crate::common::{app, messages};

#[test]
fn typ_28_each_literal_kind_takes_its_constants() {
    let accepted = [
        "from typing import Literal\n\n\ndef f(x: Literal['r', 'w']) -> str:\n    return x\n\n\n\
         print(f('r'))\n",
        "from typing import Literal\n\n\ndef f(x: Literal[-1, 0, 1]) -> int:\n    return x\n\n\n\
         print(f(-1) + f(1))\n",
        "from typing import Literal\n\n\ndef f(x: Literal[True]) -> bool:\n    return x\n\n\n\
         print(f(True))\n",
        "from typing import Literal\n\n\ndef f(x: Literal[b'a']) -> bytes:\n    return x\n\n\n\
         print(f(b'a'))\n",
        "from typing import Literal\n\n\ndef f(x: Literal['a', None] = None) -> str:\n    \
         return '-' if x is None else x\n\n\nprint(f(), f('a'))\n",
        "import typing\n\n\ndef f(x: typing.Literal[typing.Literal['a'], 'b']) -> str:\n    \
         return x\n\n\nprint(f('b'))\n",
        // A variable, an attribute and a result declared of literal types pass for them.
        "from typing import Literal\n\ntype Mode = Literal['r', 'w']\n\n\nclass File:\n    \
         def __init__(self, mode: Mode) -> None:\n        self.mode: Mode = mode\n\n    \
         def current(self) -> Mode:\n        return self.mode\n\n\n\
         def use(mode: Literal['r', 'w', 'a']) -> str:\n    return mode\n\n\n\
         handle = File('r')\nchosen: Mode = handle.current()\n\
         print(use(chosen), use(handle.mode), use(handle.current()))\n",
        // Collections of literal types take displays of their constants.
        "from typing import Literal\n\nmodes: list[Literal['r', 'w']] = ['r', 'w']\n\
         table: dict[Literal['a'], int] = {'a': 1}\nprint(modes, table)\n",
    ];
    for source in accepted {
        assert_eq!(app(source), Vec::<String>::new(), "{source}");
    }
}

#[test]
fn typ_28_a_value_that_is_not_one_of_the_constants_is_e0431() {
    let cases = [
        (
            "from typing import Literal\n\n\ndef f(x: Literal['r', 'w']) -> None:\n    \
             print(x)\n\n\nf('x')\n",
            "this is `'x'`, and Literal['r', 'w'] is wanted here",
        ),
        (
            "from typing import Literal\n\n\ndef f(x: Literal['r']) -> None:\n    print(x)\n\n\n\
             def g(text: str) -> None:\n    f(text)\n",
            "this is a str, and Literal['r'] is wanted here",
        ),
        (
            "from typing import Literal\n\n\ndef f() -> Literal[1, 2]:\n    return 3\n",
            "this is `3`, and Literal[1, 2] is wanted here",
        ),
        (
            "from typing import Literal\n\nflag: Literal[True] = False\n",
            "this is `False`, and Literal[True] is wanted here",
        ),
        (
            "from typing import Literal\n\nmode: Literal['r', 'w'] = 'r'\nmode = 'a'\n",
            "this is `'a'`, and Literal['r', 'w'] is wanted here",
        ),
        (
            "from typing import Literal\n\nmodes: list[Literal['r']] = ['r', 'w']\n",
            "this is `'w'`, and Literal['r'] is wanted here",
        ),
        (
            "from typing import Literal\n\n\ndef f(x: Literal['r', 'w'] = 'a') -> None:\n    \
             print(x)\n",
            "the default value is `'a'`, and the parameter is Literal['r', 'w']",
        ),
        // A variable `isinstance` narrowed is of the base type, not a literal one.
        (
            "from typing import Literal\n\n\ndef f(x: Literal['r']) -> None:\n    print(x)\n\n\n\
             def g(value: str | int) -> None:\n    if isinstance(value, str):\n        f(value)\n",
            "this is a str, and Literal['r'] is wanted here",
        ),
        // A module variable a function assigns is not narrowed, since a call may change it.
        (
            "from typing import Literal\n\nmode = 'r'\n\n\ndef f(x: Literal['r']) -> None:\n    \
             print(x)\n\n\ndef reset() -> None:\n    global mode\n    mode = 'w'\n\n\n\
             def g() -> None:\n    if mode == 'r':\n        f(mode)\n",
            "this is a str, and Literal['r'] is wanted here",
        ),
    ];
    for (source, message) in cases {
        let found = messages(source);
        assert!(
            found
                .iter()
                .any(|(code, text, rule)| code == "E0431" && text == message && rule == "TYP-28"),
            "{source}\n{found:?}"
        );
    }
}

#[test]
fn typ_28_a_literal_annotation_takes_only_constants() {
    let cases = [
        "from typing import Literal\n\nx: Literal[1.5] = 1.5\n",
        "from typing import Literal\n\ny = 'a'\nx: Literal[y] = 'a'\n",
        "from typing import Literal\n\nx: Literal = 'a'\n",
        "from typing import Literal\n\nx: Literal[+1] = 1\n",
        "from typing import Literal\n\nx: Literal[list[int]] = []\n",
    ];
    for source in cases {
        let found = messages(source);
        assert!(
            found
                .iter()
                .any(|(code, _, rule)| code == "E0408" && rule == "TYP-28"),
            "{source}\n{found:?}"
        );
    }
}

#[test]
fn typ_28_a_comparison_with_constants_narrows_the_variable() {
    let accepted = [
        // `==` and `!=` on a `str`.
        "from typing import Literal\n\n\ndef f(x: Literal['r']) -> str:\n    return x\n\n\n\
         def g(text: str) -> str:\n    if text == 'r':\n        return f(text)\n    \
         if 'r' != text:\n        return '-'\n    return f(text)\n\n\nprint(g('r'))\n",
        // `in` with a tuple, a list and a set of constants.
        "from typing import Literal\n\n\ndef f(x: Literal['r', 'w']) -> str:\n    return x\n\n\n\
         def g(text: str) -> str:\n    if text in ('r', 'w'):\n        return f(text)\n    \
         if text in ['r']:\n        return f(text)\n    if text in {'w'}:\n        \
         return f(text)\n    return '-'\n\n\nprint(g('w'))\n",
        // A variable of literal types narrows to the others when the test fails.
        "from typing import Literal\n\n\ndef f(x: Literal['w']) -> str:\n    return x\n\n\n\
         def g(mode: Literal['r', 'w']) -> str:\n    if mode == 'r':\n        return 'read'\n    \
         return f(mode)\n\n\nprint(g('w'))\n",
        "from typing import Literal\n\n\ndef f(x: Literal['w']) -> str:\n    return x\n\n\n\
         def g(mode: Literal['r', 'w']) -> str:\n    if mode not in ('r',):\n        \
         return f(mode)\n    return 'read'\n\n\nprint(g('w'))\n",
        // An optional is not `None` when it equals a constant.
        "from typing import Literal\n\n\ndef f(x: Literal['r']) -> str:\n    return x\n\n\n\
         def g(mode: Literal['r', 'w'] | None) -> str:\n    if mode == 'r':\n        \
         return f(mode)\n    return '-'\n\n\nprint(g(None))\n",
        // A test against no constants narrows nothing.
        "def g(text: str) -> str:\n    if text in ():\n        return text\n    return '-'\n\n\n\
         print(g('r'))\n",
        // `True` equals `1`.
        "from typing import Literal\n\n\ndef f(x: Literal[True]) -> bool:\n    return x\n\n\n\
         def g(flag: Literal[True, False]) -> bool:\n    if flag == 1:\n        return f(flag)\n    \
         return False\n\n\nprint(g(True))\n",
    ];
    for source in accepted {
        assert_eq!(app(source), Vec::<String>::new(), "{source}");
    }
    let found = messages(
        "from typing import Literal\n\n\ndef f(x: Literal['r']) -> str:\n    return x\n\n\n\
         def g(text: str) -> str:\n    if text == 'r':\n        return '-'\n    return f(text)\n",
    );
    assert_eq!(found.len(), 1, "{found:?}");
    assert_eq!(found[0].0, "E0431", "{found:?}");
}

#[test]
fn typ_28_a_match_of_literal_types_narrows_and_covers_them() {
    let source = "from typing import Literal\n\n\ndef f(x: Literal['w']) -> str:\n    return x\n\n\n\
                  def name(mode: Literal['r', 'w', 'a']) -> str:\n    match mode:\n        \
                  case 'r' | 'a':\n            return 'other'\n        case _:\n            \
                  return f(mode)\n\n\ndef count(mode: Literal[1, -1]) -> int:\n    match mode:\n        \
                  case 1:\n            return 1\n        case -1:\n            return 2\n\n\n\
                  print(name('w'), count(-1))\n";
    assert_eq!(app(source), Vec::<String>::new());
    let flags = "from typing import Literal\n\n\ndef name(flag: Literal[True, False]) -> str:\n    \
                 match flag:\n        case True:\n            return 'on'\n        \
                 case False:\n            return 'off'\n\n\nprint(name(True))\n";
    assert_eq!(app(flags), Vec::<String>::new());
    let found = messages(
        "from typing import Literal\n\n\ndef count(mode: Literal['r', 'w']) -> int:\n    \
         match mode:\n        case 'r':\n            return 1\n",
    );
    let codes: Vec<&str> = found.iter().map(|(code, ..)| code.as_str()).collect();
    assert_eq!(codes, ["W0401", "E0405"], "{found:?}");
    assert!(found[0].1.contains("Literal['w']"), "{found:?}");
}
