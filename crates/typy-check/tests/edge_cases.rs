//! Tests of programs at the edges of what the checker reads: rarely written forms that it
//! accepts, and the codes it gives what it refuses.

mod common;

use common::app;

#[test]
fn rarely_written_forms_are_accepted() {
    let cases = [
        "xs = [1, 2, 3]\ndel (xs[0], xs[1])\nprint(xs)\n",
        "print(b'ab' * 2, 2 * b'ab')\n",
        "d = {1: 2}\nprint(d.keys() == d.keys(), d.items() == {(1, 2)})\n",
        "for i in range(1, 10, 0):\n    pass\n",
        "def g(x: int) -> int:\n    while True:\n        break\n    return x\n",
        "print({1: 2}.keys() | {3}, {1: 2}.items() == {1: 2}.items())\n",
        "import math\nprint(math.pi, math.tau, math.inf)\n",
        "d = [0]\nprint([0 for d[0] in range(3)])\n",
        "xs = [1, 2, 3]\ndel [xs[0], xs[1]]\nprint(xs)\n",
        "it = iter([1, None])\nprint(next(it, None))\n",
        "x = list((1, 2))\ny = set([1])\nz = dict(a=1)\nprint(x, y, z)\n",
        "print(bytes([1, 2]))\n",
        "print(dict(zip([1], [2])))\n",
        "x = None if len('a') > 0 else None\nprint(x)\n",
        "x = 2\nif 1 < x < 3:\n    print(x)\n",
        "f = open('x.txt', errors='strict')\n",
        "class Box[T]:\n    def __init__(self, v: T) -> None:\n        self.v = v\ndef f[U](x: U) -> U:\n    b: Box[int] = Box(1)\n    print(b.v)\n    return x\nprint(f(2))\n",
        "type A = int\ntype B = list[A] | dict[A, A]\nx: B = [1]\nprint(x)\n",
    ];
    for source in cases {
        assert_eq!(app(source), Vec::<String>::new(), "{source}");
    }
}

#[test]
fn refused_forms_give_their_codes() {
    let cases: &[(&str, &str)] = &[
        ("missing()\n", "E0205"),
        ("print(round(1.5, 2, 3))\n", "E0404"),
        ("print(type(1, 2))\n", "E0404"),
        (
            "class A:\n    def __iter__(self) -> 'A':\n        return self\nfor x in A():\n    pass\n",
            "E0411",
        ),
        ("print(open('x', opener=len))\n", "E0201"),
        (
            "class B:\n    def __init__(self, n: int) -> None:\n        self.n: int = n\n        self.n: str = 'a'\n",
            "E0402",
        ),
        ("s: set[int] = set()\nt: set[str] = s\n", "E0402"),
        ("def f() -> None:\n    missing()\n", "E0205"),
        ("s: list[int] = {x for x in range(3)}\n", "E0402"),
        ("s: set[int] = [x for x in range(3)]\n", "E0402"),
        ("print(int.fromkeys([1]))\n", "E0412"),
        (
            "d: dict[int, int | None] = {}\nd.setdefault(1, default=None)\n",
            "E0404",
        ),
        ("xs = [1]\nxs.clear(1)\n", "E0404"),
        ("print(''.join([missing]))\n", "E0205"),
        ("class A:\n    pass\nprint(next(A()))\n", "E0404"),
        (
            "from collections.abc import Iterable\ndef f(xs: Iterable[int]) -> None:\n    print(xs)\nf(['a'])\n",
            "E0409",
        ),
        ("d: dict[int, int | str] = {}\nd[1] = 2.5\n", "E0402"),
        (
            "class A:\n    def __iter__(self) -> Missing:\n        return self\nfor x in A():\n    pass\n",
            "E0205",
        ),
        ("for i in range(missing):\n    pass\n", "E0205"),
        ("import math\nprint(math.sqrt(missing))\n", "E0205"),
        ("x = b'ab' * 1.5\n", "E0401"),
        ("print('%d' % \"it's\")\n", "E0427"),
        ("print('%d' % 'a\\\\b')\n", "E0427"),
        ("print(round())\n", "E0404"),
        ("s: set[Missing] = {1}\n", "E0205"),
        ("print(type(1, a=2))\n", "E0404"),
        ("class A:\n    pass\nx: A[int] = A()\n", "E0202"),
        ("x: Missing[int] = 1\n", "E0202"),
        ("def f(x: int = 1 + 1) -> int:\n    return x\n", "E0201"),
        ("from typing import Optional\nx: Optional = None\n", "E0408"),
        ("from typing import Union\ny: Union = 1\n", "E0408"),
        // Only `typing` gives `Protocol`.
        ("import math\nclass P(math.Protocol):\n    pass\n", "E0201"),
    ];
    for (source, code) in cases {
        let found = app(source);
        assert_eq!(
            found.first().map(String::as_str),
            Some(*code),
            "{source}\n{found:?}"
        );
    }
}
