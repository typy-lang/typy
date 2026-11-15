//! Tests of classes: the attributes `__init__` declares, the methods a body defines, and calls
//! through `super()`, each against what CPython runs or refuses.

mod common;

use common::app;

/// Asserts that each program is accepted.
fn accepted(cases: &[&str]) {
    for source in cases {
        assert_eq!(app(source), Vec::<String>::new(), "{source}");
    }
}

/// Asserts that each program is refused with its code first.
fn refused(cases: &[(&str, &str)]) {
    for (source, code) in cases {
        let found = app(source);
        assert_eq!(
            found.first().map(String::as_str),
            Some(*code),
            "{source}\n{found:?}"
        );
    }
}

#[test]
fn init_assigns_attributes_as_cpython_does() {
    accepted(&[
        // An attribute of an attribute of the instance.
        "class B:\n    def __init__(self) -> None:\n        self.v: int = 0\n\
         class A:\n    def __init__(self) -> None:\n        self.b: B = B()\n        self.b.v = 3\n\
         print(A().b.v)\n",
        // The same declaration twice.
        "class A:\n    def __init__(self) -> None:\n        self.x: int = 1\n        self.x: int = 2\n\
         print(A().x)\n",
        // A bare `Final` takes the type of its value.
        "from typing import Final\nclass A:\n    def __init__(self) -> None:\n        self.x: Final = 1\n\
         print(A().x)\n",
    ]);
}

#[test]
fn a_dataclass_without_eq_and_an_exit_without_annotations_are_accepted() {
    accepted(&[
        "from dataclasses import dataclass\n@dataclass(eq=False)\nclass P:\n    x: int\n\
         print(P(1) == P(1))\n",
        "class C:\n    def __enter__(self) -> 'C':\n        return self\n\
         \x20   def __exit__(self, a, b, c) -> None:\n        pass\nwith C():\n    print(1)\n",
    ]);
}

#[test]
fn super_reaches_the_methods_of_the_bases() {
    accepted(&[
        // Another method of the base, from `__init__`.
        "class A:\n    def __init__(self) -> None:\n        self.x: int = 1\n\
         \x20   def f(self) -> int:\n        return 1\n\
         class B(A):\n    def __init__(self) -> None:\n        super().__init__()\n\
         \x20       print(super().f())\nB()\n",
        // The `__init__` of a base's base, through a base without one.
        "class A:\n    def __init__(self) -> None:\n        self.x: int = 1\n\
         class B(A):\n    pass\n\
         class C(B):\n    def __init__(self) -> None:\n        super().__init__()\n\
         \x20       self.y: int = 2\nprint(C().x)\n",
    ]);
}

#[test]
fn unrelated_classes_keep_their_own_special_methods() {
    accepted(&[
        "class A:\n    pass\n\
         class B:\n    def __bool__(self) -> bool:\n        return True\n\
         class C:\n    def __len__(self) -> int:\n        return 0\n\
         a = A()\nif a:\n    print(1)\n",
        "class A:\n    def __eq__(self, other: 'A') -> bool:\n        return True\n\
         class B:\n    def __eq__(self, other: 'B') -> bool:\n        return False\n\
         print(A() == A())\n",
    ]);
}

#[test]
fn a_base_orders_against_an_instance_of_a_derived_class() {
    accepted(&[
        "class A:\n    def __lt__(self, other: 'A') -> bool:\n        return True\n\
         class B(A):\n    pass\nprint(A() < B())\n",
    ]);
}

#[test]
fn an_isinstance_test_of_an_unrelated_class_is_accepted() {
    accepted(&[
        "def f(e: ValueError) -> None:\n    if isinstance(e, KeyError):\n        print('k')\n\
         f(ValueError())\n",
        "class Box[T]:\n    def __init__(self, v: T) -> None:\n        self.v = v\n\
         class Base:\n    pass\n\
         def f(x: Base) -> None:\n    if isinstance(x, Box):\n        print(1)\nf(Base())\n",
    ]);
}

#[test]
fn class_bodies_and_methods_out_of_reach_give_their_codes() {
    refused(&[
        (
            "class A:\n    def __init__(self) -> None:\n        self.x: int = 1\n\
             \x20   def m(self) -> None:\n        self.y: int = 2\n",
            "E0414",
        ),
        (
            "class Box[T]:\n    def __init__(self, v: T) -> None:\n        self.v = v\n\
             b: Box[int, str] = Box(1)\n",
            "E0408",
        ),
        ("class A:\n    a = b = 1\nprint(A.a)\n", "E0202"),
        ("class A:\n    a, b = 1, 2\nprint(A.a)\n", "E0202"),
        (
            "class C:\n    def __enter__(self) -> 'C':\n        return self\n\
             \x20   def __exit__(self) -> None:\n        pass\n",
            "E0404",
        ),
        (
            "class A:\n    def __init__() -> None:\n        pass\n",
            "E0404",
        ),
        (
            "from typing import Final\nclass A:\n\
             \x20   def __init__(self, other: 'A | None') -> None:\n\
             \x20       self.x: Final[int] = 1\n        if other is not None:\n\
             \x20           other.y: Final[int] = 2\n",
            "E0408",
        ),
        (
            "class A:\n    @staticmethod\n    def f() -> int:\n        return 1\n\
             class B(A):\n    @staticmethod\n    def f() -> int:\n        return 2\n",
            "E0201",
        ),
        (
            "class A:\n    def f(self) -> int:\n        return 1\n\
             class B(A):\n    def f(self) -> int:\n        return super(x=1).f()\n",
            "E0201",
        ),
        (
            "class A:\n    def __init__(self) -> None:\n        print(self)\n\
             \x20       print(self)\n        self.x: int = 1\n",
            "E0309",
        ),
        (
            "class A:\n    @staticmethod\n    def f(x) -> None:\n        pass\n",
            "E0203",
        ),
        (
            "class A:\n    def f(self, x) -> None:\n        pass\n",
            "E0203",
        ),
        (
            "class C:\n    def __exit__(self, a, b, c, d) -> None:\n        pass\n",
            "E0203",
        ),
        (
            "class A:\n    @cache\n    def f(self) -> None:\n        pass\n",
            "E0201",
        ),
        (
            "import functools\nclass A:\n    @functools.cache\n    def f(self) -> int:\n\
             \x20       return 1\n",
            "E0201",
        ),
        (
            "class A:\n    def __lt__(self, other: 'A') -> bool:\n        return True\n\
             class C:\n    pass\nprint(A() < C())\n",
            "E0407",
        ),
        ("x: int = IndexError()\n", "E0402"),
        (
            "class Box[T]:\n    def __init__(self, v: T) -> None:\n        self.v = v\n\
             class A:\n    pass\ndef f[T](b: Box[T]) -> T:\n    return b.v\nprint(f(A()))\n",
            "E0404",
        ),
    ]);
}

#[test]
fn class_statements_typy_cannot_compile_are_refused() {
    refused(&[
        // An assignment to several names in the class body.
        ("class A:\n    a, b = 1, 2\nprint(A.a)\n", "E0202"),
        // An `__init__` without the instance, which CPython defines and cannot call.
        (
            "class A:\n    def __init__() -> None:\n        pass\nprint(1)\n",
            "E0404",
        ),
        // `Final` on an attribute of another object than the instance.
        (
            "from typing import Final\n\
             class B:\n    def __init__(self) -> None:\n        self.v: int = 0\n\
             class A:\n    def __init__(self, b: B) -> None:\n        b.v: Final[int] = 1\n",
            "E0408",
        ),
    ]);
}
