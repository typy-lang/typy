//! Functions of the program as values of `Callable` types (FUN-05), and the rule that a
//! function fits a `Callable` only when no argument or result is converted (NFR-19).

use crate::common::{app, messages};

/// Two classes, the second derived from the first, and a function of each kind the cases use.
const DEFINITIONS: &str = "\
from typing import Callable


class Shape:
    pass


class Circle(Shape):
    pass


def of_shape(shape: Shape) -> None:
    pass


def of_circle(circle: Circle) -> None:
    pass


def make_shape() -> Shape:
    return Shape()


def make_circle() -> Circle:
    return Circle()


def of_float(value: float) -> float:
    return value


def of_optional(value: int | None) -> int:
    return 0


def of_int(value: int) -> int:
    return value


";

#[test]
fn nfr_19_a_function_fits_a_callable_when_nothing_is_converted() {
    for statement in [
        "f: Callable[[Circle], None] = of_shape",
        "f: Callable[[], Shape] = make_circle",
        "f: Callable[[int], int] = of_int",
        "f: Callable[[int | None], int] = of_optional",
        "f: list[Callable[[Circle], None]] = [of_shape, of_circle]",
        "def g(apply: Callable[[Callable[[int], int]], int]) -> int:\n    return apply(of_int)",
    ] {
        let source = format!("{DEFINITIONS}{statement}\n");
        assert_eq!(app(&source), Vec::<String>::new(), "{statement}");
    }
}

#[test]
fn nfr_19_a_function_whose_values_would_be_converted_is_refused() {
    for (statement, code) in [
        // Each pair below is held differently, so the code would read a value as another
        // type; a call by name converts or refuses each.
        ("f: Callable[[int], float] = of_float", "E0402"),
        ("f: Callable[[int], int] = of_optional", "E0402"),
        ("f: Callable[[int], int | None] = of_int", "E0402"),
        // A function of circles may not be given any shape, nor a shape taken as a circle.
        ("f: Callable[[Shape], None] = of_circle", "E0402"),
        ("f: Callable[[], Circle] = make_shape", "E0402"),
        ("f: Callable[[int, int], int] = of_int", "E0402"),
        (
            "def g(apply: Callable[[Shape], None]) -> None:\n    pass\ng(of_circle)",
            "E0404",
        ),
        (
            "def g() -> Callable[[int], float]:\n    return of_float",
            "E0405",
        ),
    ] {
        let source = format!("{DEFINITIONS}{statement}\n");
        assert_eq!(app(&source), [code], "{statement}");
    }
}

#[test]
fn fun_05_a_call_through_a_value_passes_one_argument_for_each_parameter_by_position() {
    let cases = [
        (
            "f = of_int\nprint(f(value=1))",
            "E0404",
            "a value of type Callable[[int], int] names no parameters, so `value=` cannot be passed",
        ),
        (
            "f = of_int\nprint(f())",
            "E0404",
            "a value of type Callable[[int], int] takes 1 argument, and 0 are given",
        ),
        (
            "f = make_shape\nprint(f(1, 2))",
            "E0404",
            "a value of type Callable[[], Shape] takes 0 arguments, and 2 are given",
        ),
        (
            "f = of_int\nprint(f('one'))",
            "E0404",
            "argument 1 must be an int, and this is a str",
        ),
    ];
    for (statement, code, message) in cases {
        let found = messages(&format!("{DEFINITIONS}{statement}\n"));
        assert_eq!(
            found,
            [(code.to_owned(), message.to_owned(), "FUN-05".to_owned())]
                .into_iter()
                .map(|(code, message, rule)| if message.starts_with("argument") {
                    (code, message, "TYP-01".to_owned())
                } else {
                    (code, message, rule)
                })
                .collect::<Vec<_>>(),
            "{statement}"
        );
    }
}

#[test]
fn fun_05_a_function_without_one_callable_type_is_not_a_value() {
    let cases = [
        (
            "def first[T](items: list[T]) -> T:\n    return items[0]\npick = first",
            "E0202",
            "the generic function `first` is a copy for each set of types, so it has no one value",
        ),
        (
            "def clamp(value: int, *, low: int) -> int:\n    return value\nlimit = clamp",
            "E0202",
            "`clamp` takes `low` by keyword only, and a `Callable` passes every argument by \
             position, so `clamp` has no `Callable` type",
        ),
        (
            "print(len)",
            "E0201",
            "using `len` as a value is planned for TyPy 0.10 (FUN-08)",
        ),
    ];
    for (statement, code, message) in cases {
        let found = messages(&format!("{DEFINITIONS}{statement}\n"));
        assert_eq!(found.len(), 1, "{statement}: {found:?}");
        assert_eq!((found[0].0.as_str(), found[0].1.as_str()), (code, message));
    }
}

#[test]
fn fun_05_a_callable_annotation_lists_its_parameter_types_and_its_result() {
    let cases = [
        (
            "h: Callable[..., int]",
            "`Callable[..., R]` takes any arguments, which TyPy cannot check",
        ),
        (
            "h: Callable",
            "a `Callable` annotation names its parameter types and its result, such as \
             `Callable[[int, str], bool]`",
        ),
        (
            "h: Callable[int, int]",
            "a `Callable` annotation names its parameter types and its result, such as \
             `Callable[[int, str], bool]`",
        ),
        (
            "h: Callable[[int]]",
            "a `Callable` annotation names its parameter types and its result, such as \
             `Callable[[int, str], bool]`",
        ),
    ];
    for (statement, message) in cases {
        let found = messages(&format!("{DEFINITIONS}{statement}\n"));
        assert_eq!(
            found,
            [("E0408".to_owned(), message.to_owned(), "FUN-05".to_owned())],
            "{statement}"
        );
    }
    let missing = messages(&format!("{DEFINITIONS}h: Callable[[Missing], int]\n"));
    assert_eq!(missing.len(), 1, "{missing:?}");
    assert_eq!(missing[0].0, "E0205");
}

#[test]
fn fun_05_only_a_function_value_is_called() {
    let cases = [
        (
            "count = 3\ncount()",
            "`count` is an int and cannot be called",
        ),
        ("values = [1]\nvalues[0]()", "an int cannot be called"),
        (
            "class Box:\n    def __init__(self) -> None:\n        self.size = 1\nBox().size()",
            "`Box.size` is an int and cannot be called",
        ),
        (
            "f: Callable[[int], int] | None = None\nf(1)",
            "`f` is a Callable[[int], int] | None and cannot be called",
        ),
    ];
    for (statement, message) in cases {
        let found = messages(&format!("{DEFINITIONS}{statement}\n"));
        assert_eq!(
            found,
            [("E0401".to_owned(), message.to_owned(), "EXP-02".to_owned())],
            "{statement}"
        );
    }
}

#[test]
fn fun_05_an_attribute_called_before_its_type_is_known_is_e0310() {
    // `A.__init__` is checked before `B.__init__`, which gives `w` its type.
    let source = "def f(x: int) -> int:\n    return x\n\n\nclass A:\n    \
                  def __init__(self, b: \"B\") -> None:\n        self.v = b.w(1)\n\n\n\
                  class B:\n    def __init__(self) -> None:\n        self.w = f\n";
    let found = messages(source);
    assert_eq!(found.len(), 1, "{found:?}");
    assert_eq!(
        (found[0].0.as_str(), found[0].1.as_str()),
        (
            "E0310",
            "the type of `B.w` is not known here, because `B.__init__` gives it and is checked \
             later"
        )
    );
}

#[test]
fn fun_05_a_function_value_compares_and_hashes_by_identity_and_never_orders() {
    let accepted = format!(
        "{DEFINITIONS}print(of_int == of_int, of_int is of_int, {{of_int: 1}}, \
         [of_int] == [of_int], of_int != of_float)\n"
    );
    assert_eq!(app(&accepted), Vec::<String>::new());
    let ordered = messages(&format!("{DEFINITIONS}print(of_int < of_int)\n"));
    assert_eq!(ordered.len(), 1, "{ordered:?}");
    assert_eq!(ordered[0].0, "E0407");
}

#[test]
fn lib_17_the_typing_names_a_later_release_gives_are_planned_for_it() {
    for (name, rule) in [("NamedTuple", "CLS-27"), ("Sequence", "LIB-20")] {
        let found = messages(&format!("from typing import {name}\n"));
        assert_eq!(
            found,
            [(
                "E0201".to_owned(),
                format!("`typing.{name}` is planned for TyPy 0.10 ({rule})"),
                rule.to_owned()
            )]
        );
    }
}

/// A class whose method a subclass overrides, and a static method, for the method cases.
const METHODS: &str = "\
class Shape:
    def area(self, scale: float) -> float:
        return scale

    @staticmethod
    def unit() -> int:
        return 1


class Square(Shape):
    def area(self, scale: float) -> float:
        return 4.0 * scale


";

#[test]
fn fun_05_a_method_read_through_an_instance_or_its_class_is_a_value() {
    for statement in [
        "f: Callable[[float], float] = Shape().area",
        "f: Callable[[float], float] = Square().area",
        "f: Callable[[Shape, float], float] = Shape.area",
        "f: Callable[[], int] = Shape.unit",
        "f: Callable[[], int] = Square().unit",
        "shapes = [Square(), Shape()]\nfs = [each.area for each in shapes]",
    ] {
        let source = format!("from typing import Callable\n\n\n{METHODS}{statement}\n");
        assert_eq!(app(&source), Vec::<String>::new(), "{statement}");
    }
}

#[test]
fn fun_05_a_bound_method_takes_the_parameters_after_self() {
    let source = format!(
        "from typing import Callable\n\n\n{METHODS}f: Callable[[Shape, float], float] = \
         Shape().area\n"
    );
    let found = messages(&source);
    assert_eq!(found.len(), 1, "{found:?}");
    assert_eq!(
        found[0].1,
        "`f` was declared as Callable[[Shape, float], float] and is assigned a \
         Callable[[float], float]"
    );
}

#[test]
fn fun_09_the_methods_without_one_bound_value_yet_are_planned() {
    let cases = [
        (
            "class C:\n    @classmethod\n    def make(cls) -> int:\n        return 1\nf = C.make\n",
            "`make` is a class method, and using one as a value is planned for TyPy 0.10 (FUN-09)",
        ),
        (
            "class B:\n    @staticmethod\n    def s() -> int:\n        return 1\n\
             class D(B):\n    @staticmethod\n    def s() -> int:\n        return 2\nf = B().s\n",
            "`s` is a static method a subclass defines again, and using one as a value is \
             planned for TyPy 0.10 (FUN-09)",
        ),
        (
            "class B:\n    def v(self) -> int | None:\n        return None\n\
             class D(B):\n    def v(self) -> int:\n        return 1\nf = B().v\n",
            "`v` is a method a subclass overrides with other parameters or a result held \
             otherwise, and using one as a value is planned for TyPy 0.10 (FUN-09)",
        ),
        (
            "class B:\n    def v(self) -> int:\n        return 0\n\
             class D(B):\n    @staticmethod\n    def v() -> int:\n        return 1\nf = B().v\n",
            "`v` is a method a subclass overrides with other parameters or a result held \
             otherwise, and using one as a value is planned for TyPy 0.10 (FUN-09)",
        ),
        (
            "from typing import Protocol\nclass P(Protocol):\n    def n(self) -> int: ...\n\
             def f(p: P) -> None:\n    g = p.n\n",
            "`n` is a method of the protocol `P`, and using a protocol value's method as a \
             value is planned for TyPy 0.10 (FUN-09)",
        ),
    ];
    for (source, message) in cases {
        let found = messages(source);
        assert_eq!(
            found,
            [("E0201".to_owned(), message.to_owned(), "FUN-09".to_owned())],
            "{source}"
        );
    }
}

#[test]
fn fun_05_a_method_without_one_callable_type_is_not_a_value() {
    let cases = [
        (
            "class B:\n    def pick[T](self, items: list[T]) -> T:\n        return items[0]\n\
             f = B().pick\n",
            "the method `B.pick` has type parameters of its own, so it is a copy for each set \
             of types and has no one value",
        ),
        (
            "class B:\n    def put(self, *, item: int) -> None:\n        pass\nf = B.put\n",
            "`B.put` takes `item` by keyword only, and a `Callable` passes every argument by \
             position, so `B.put` has no `Callable` type",
        ),
    ];
    for (source, message) in cases {
        let found = messages(source);
        assert_eq!(found.len(), 1, "{source}: {found:?}");
        assert_eq!(
            (found[0].0.as_str(), found[0].1.as_str()),
            ("E0202", message)
        );
    }
    let assigned = messages(&format!("{METHODS}Square().area = Shape().area\n"));
    assert_eq!(
        assigned,
        [(
            "E0414".to_owned(),
            "`Square.area` is a method, which an assignment does not replace".to_owned(),
            "CLS-02".to_owned()
        )]
    );
}

#[test]
fn fun_05_a_key_is_a_function_value_of_one_item_whose_results_order() {
    let accepted = "class Box:\n    def __init__(self, size: int) -> None:\n        \
                    self.size = size\n\n    def measure(self, other: int) -> int:\n        \
                    return self.size + other\n\n\ndef size(text: str) -> int:\n    \
                    return len(text)\n\n\nwords = [\"a\"]\nnumbers = [1, 2]\n\
                    print(sorted(words, key=size), min(words, key=size, default=\"x\"))\n\
                    print(max(numbers, key=Box(1).measure), max(1, 2, key=Box(2).measure))\n\
                    words.sort(reverse=True, key=size)\nwords.sort(key=None)\n\
                    print(min(1, 2, key=None), sorted(words, key=None))\n";
    assert_eq!(app(accepted), Vec::<String>::new());
    let cases = [
        (
            "print(sorted(words, key=3))",
            "E0404",
            "the `key` of `sorted` is called with each item, of type str, and this is an int",
        ),
        (
            "print(min(words, key=of_int))",
            "E0404",
            "the `key` of `min` is called with each item, of type str, and this is a \
             Callable[[int], int]",
        ),
        (
            "words.sort(key=of_int)",
            "E0404",
            "the `key` of `sort` is called with each item, of type str, and this is a \
             Callable[[int], int]",
        ),
        (
            "print(max(words, key=boxed))",
            "E0407",
            "Box and Box cannot be compared",
        ),
        (
            "words.sort(1, key=size)",
            "E0404",
            "`sort` takes no positional arguments",
        ),
        (
            "words.sort(key=size, other=True)",
            "E0404",
            "`sort` has no parameter named `other`",
        ),
        (
            "print(any(words, key=size))",
            "E0404",
            "`any` has no parameter named `key`",
        ),
        (
            "print(min(1, 2.0, key=of_int))",
            "E0404",
            "the arguments of `min` must have one type, because CPython returns one of them \
             unchanged; they are int and float",
        ),
    ];
    let definitions = "class Box:\n    pass\n\n\ndef size(text: str) -> int:\n    \
                       return len(text)\n\n\ndef boxed(text: str) -> Box:\n    return Box()\n\n\n\
                       def of_int(value: int) -> int:\n    return value\n\n\nwords = [\"a\"]\n";
    for (statement, code, message) in cases {
        let found = messages(&format!("{definitions}{statement}\n"));
        assert_eq!(found.len(), 1, "{statement}: {found:?}");
        assert_eq!(
            (found[0].0.as_str(), found[0].1.as_str()),
            (code, message),
            "{statement}"
        );
    }
}
