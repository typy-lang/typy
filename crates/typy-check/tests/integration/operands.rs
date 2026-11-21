//! Operators, membership tests, displays and spreads with every kind of operand.

use crate::common::{app, messages};

#[test]
fn operators_and_displays_of_every_operand_kind_are_accepted() {
    let accepted = [
        "class Acc:\n    def __init__(self) -> None:\n        self.total = 0\n\n    \
        def __iadd__(self, other: int) -> \"Acc\":\n        self.total += other\n        \
        return self\n\n\nclass Holder:\n    def __init__(self) -> None:\n        \
        self._acc = Acc()\n\n    @property\n    def acc(self) -> Acc:\n        \
        return self._acc\n\n    @acc.setter\n    def acc(self, value: Acc) -> None:\n        \
        self._acc = value\n\n\nh = Holder()\nh.acc += 2\nprint(h.acc.total)\n",
        "class Holder:\n    def __init__(self) -> None:\n        self._text = \"%d items\"\n\
        \n    @property\n    def text(self) -> str:\n        return self._text\n\n    \
        @text.setter\n    def text(self, value: str) -> None:\n        self._text = value\n\
        \n\nh = Holder()\nh.text %= 3\nprint(h.text)\n",
        "x: tuple[int, ...] = (*[1], 2)\nprint(x)\n",
        "d = {\"a\": 1}\nprint(d == dict(a=1))\n",
        "d = {\"a\": 1}\nd.update((k, 2) for k in \"bc\")\nprint(d)\n",
        "import ctypes\n\nIntArray = 3 * ctypes.c_int\nprint(len(IntArray()))\n",
        "class A:\n    pass\n\n\na, b, c = A(), A(), A()\nprint(a == b == c, a is b is c)\n",
        "print(NotImplemented is None)\n",
    ];
    for source in accepted {
        assert_eq!(app(source), Vec::<String>::new(), "{source}");
    }
}

#[test]
fn an_operand_an_operator_or_a_display_cannot_take_is_refused() {
    let refused = [
        (
            "class A:\n    pass\n\n\nprint(1 in A())\n",
            "E0407",
            "`in` looks for an int in an A, which cannot hold it",
        ),
        (
            "from collections.abc import Iterator\n\n\nclass Numbers:\n    \
            def __iter__(self) -> Iterator[int]:\n        return iter([1, 2])\n\n\n\
            print(\"a\" in Numbers())\n",
            "E0407",
            "`in` looks for a str in a Numbers, which cannot hold it",
        ),
        (
            "print([*\"x\".nope(), *[1]])\n",
            "E0412",
            "a str has no method `nope` in TyPy",
        ),
        (
            "print({**{\"a\": 1}, 2: 3})\n",
            "E0409",
            "the entries of a dict have one key type and one value type; these are int and \
            int, and the dict holds str and int",
        ),
        (
            "print({**{\"a\": 1}, **{2: 3}})\n",
            "E0409",
            "the entries of a dict have one key type and one value type; these are int and \
            int, and the dict holds str and int",
        ),
        (
            "x = \"a\".nope()\nx = {1}\n",
            "E0412",
            "a str has no method `nope` in TyPy",
        ),
        (
            "print({\"a\": 1}.keys() == {1})\n",
            "E0407",
            "dict_keys[str, int] and set[int] cannot be compared",
        ),
        (
            "fmt = \"%d\"\nprint(fmt % \"a\".nope())\n",
            "E0412",
            "a str has no method `nope` in TyPy",
        ),
        (
            "classes = (int, 3)\nprint(isinstance(1, classes))\n",
            "E0401",
            "isinstance() arg 2 must be a type, a tuple of types, or a union: CPython raises \
             `TypeError`",
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
