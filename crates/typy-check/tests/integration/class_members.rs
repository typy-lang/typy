//! Members of classes and instances: methods reached through bases and generic copies, class
//! variables, overrides and values of protocols.

use crate::common::{app, messages};

#[test]
fn members_reached_through_bases_copies_and_overrides_are_accepted() {
    let accepted = [
        "class A:\n    def __init__(self) -> None:\n        self.x = 1\n\n\nclass B(A):\n    \
        def __init__(self) -> None:\n        other = A()\n        A.__init__(other)\n        \
        A.__init__(self)\n\n\nprint(B().x)\n",
        "class A:\n    @staticmethod\n    def f[T](x: T) -> T:\n        return x\n\n\n\
        class B(A):\n    @staticmethod\n    def f[T](x: T) -> T:\n        return x\n\n\n\
        a: A = B()\nprint(a.f(1))\n",
        "class A:\n    def __init__(self, flag: bool) -> None:\n        self.x = 1\n        \
        if flag:\n            raise ValueError(\"no\")\n            self.x = 2\n\n\n\
        print(A(False).x)\n",
        "class Shelter:\n    def find(self) -> int | str | None:\n        return None\n\n\n\
        class Kennel(Shelter):\n    def find(self) -> int | None:\n        return 1\n\n\n\
        s: Shelter = Kennel()\nprint(s.find())\n",
        "class Base:\n    count = 0\n\n    def show(self) -> int:\n        \
        return self.count\n\n\nclass Derived(Base):\n    pass\n\n\nclass Other:\n    pass\n\
        \n\nDerived.count = 5\nprint(Base().show(), Derived().show(), Other() is not None)\n",
        "class Base:\n    pass\n\n\nclass Mid(Base):\n    \
        def __radd__(self, other: int) -> int:\n        return 1\n\n\nclass Leaf(Mid):\n    \
        def __radd__(self, other: int) -> int:\n        return 2\n\n\n\
        def f(b: Base) -> None:\n    print(1 + b)\n\n\nf(Leaf())\nf(Mid())\n",
    ];
    for source in accepted {
        assert_eq!(app(source), Vec::<String>::new(), "{source}");
    }
}

#[test]
fn a_member_its_class_does_not_give_is_refused_with_the_reason() {
    let refused = [
        (
            "class Box[T]:\n    def __init__(self, v: T) -> None:\n        self.v = v\n\n    \
            @classmethod\n    def label(cls) -> str:\n        return \"box\"\n\n    \
            def show(self) -> str:\n        return self.label()\n\n\nprint(Box(1).show())\n",
            "E0201",
            "calling the class method `label` of a generic class through an instance is \
            planned for TyPy 0.14 (CLS-30)",
        ),
        (
            "class A:\n    @staticmethod\n    def s(x: int) -> int:\n        return x\n\n\n\
            print(A().s())\n",
            "E0404",
            "`A.s` is missing the argument `x`",
        ),
        (
            "class A:\n    def __init__(self) -> None:\n        self.x = None\n\n    \
            def reset(self) -> None:\n        self.x: int | None = 1\n",
            "E0410",
            "`x` is first assigned `None`, which does not give it a type",
        ),
        (
            "class Base:\n    count = 0\n\n\nclass Derived(Base):\n    @classmethod\n    \
            def bump(cls) -> None:\n        cls.count += 1\n\n\nDerived.bump()\n\
            print(Base.count)\n",
            "E0201",
            "assigning `count`, a class variable of `Base`, through a variable that holds \
            a class derived from it, which gives that class one of its own in CPython, is \
            planned for TyPy 0.14 (CLS-31)",
        ),
        (
            "class A:\n    x = 1\n\n\nA.nope = 2\n",
            "E0412",
            "a type[A] has no attributes in TyPy",
        ),
        (
            "class A:\n    @classmethod\n    def m(cls) -> int:\n        return 1\n\n\n\
            print(A[int].m())\n",
            "E0401",
            "a type[A] cannot be indexed",
        ),
        (
            "class A:\n    def __init__(self) -> None:\n        self.f = \"x\".nope()\n\n    \
            def g(self) -> None:\n        self.f()\n",
            "E0412",
            "a str has no method `nope` in TyPy",
        ),
        (
            "from typing import Protocol\n\n\nclass P(Protocol):\n    \
            def a(self) -> int: ...\n\n\nclass Q(Protocol):\n    def b(self) -> int: ...\n\n\
            \nclass A:\n    def a(self) -> int:\n        return 1\n\n\nitem: P = A()\n\
            q: Q = item\n",
            "E0402",
            "`q` was declared as Q and is assigned a P",
        ),
        (
            "class A:\n    def __radd__(self, other: str) -> int:\n        return 1\n\n\n\
            print(1 + A())\n",
            "E0401",
            "`+` is not defined for int and A",
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
