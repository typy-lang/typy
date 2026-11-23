//! Tests of checking generic functions through the public API (GEN-01 to GEN-03, GEN-07):
//! how calls solve type parameters, and what is refused.

use crate::common::{app, codes};

#[test]
fn calls_solve_type_parameters_from_every_kind_of_argument() {
    let source = concat!(
        "from collections.abc import Iterable\n",
        "def keys[K, V](table: dict[K, V]) -> list[K]:\n",
        "    return list(table)\n",
        "def swap[A, B](pair: tuple[A, B]) -> tuple[B, A]:\n",
        "    return (pair[1], pair[0])\n",
        "def head[T](items: tuple[T, ...]) -> T:\n",
        "    return items[0]\n",
        "def total[T](items: Iterable[T]) -> list[T]:\n",
        "    return [item for item in items]\n",
        "def chosen[T](value: T | None, fallback: T) -> T:\n",
        "    return fallback if value is None else value\n",
        "def labelled[T](value: T | str) -> str:\n",
        "    return str(value)\n",
        "def outer[T](value: T) -> list[T]:\n",
        "    held: list[T] = [value]\n",
        "    return held + keep(held)\n",
        "def keep[T](items: list[T]) -> list[T]:\n",
        "    return items\n",
        "def mixed[N: (int, float), T](number: N, other: T) -> T:\n",
        "    print(number + number)\n",
        "    return other\n",
        "class Shape:\n",
        "    def area(self) -> float:\n",
        "        return 1.0\n",
        "def measure(shape: Shape) -> float:\n",
        "    return shape.area()\n",
        "def bounded[S: Shape](shape: S) -> float:\n",
        "    return measure(shape)\n",
        "print(keys({'a': 1}), swap((1, 'x')), head((1, 2)), total({3}), total(range(2)))\n",
        "print(chosen(None, 4), chosen(fallback='b', value='a'), labelled(1))\n",
        "print(outer(1), outer(2), mixed(1.5, 'z'), bounded(Shape()))\n",
    );
    assert_eq!(app(source), Vec::<String>::new());
}

#[test]
fn type_parameters_are_solved_and_replaced_through_callable_and_generator_types() {
    let source = concat!(
        "from collections.abc import Callable, Generator, Iterator\n",
        "def apply[T](value: T, change: Callable[[T], T]) -> T:\n",
        "    return change(value)\n",
        "def first_of[T](changes: list[Callable[[T], T]], value: T) -> T:\n",
        "    return changes[0](value)\n",
        "def call_with_one[R](make: Callable[[int], R]) -> R:\n",
        "    return make(1)\n",
        "def repeat[T](value: T, times: int) -> Generator[T, None, int]:\n",
        "    for _ in range(times):\n",
        "        yield value\n",
        "    return times\n",
        "def drain[T](items: Generator[T, None, int]) -> list[T]:\n",
        "    return list(items)\n",
        "def head[T](items: Iterator[T]) -> T:\n",
        "    return next(items)\n",
        "def increment(number: int) -> int:\n",
        "    return number + 1\n",
        "def describe(number: int) -> str:\n",
        "    return str(number)\n",
        "a: int = apply(1, increment)\n",
        "b: int = first_of([increment], 2)\n",
        "c: str = call_with_one(describe)\n",
        "d: list[str] = drain(repeat('x', 2))\n",
        "e: float = head(repeat(1.5, 1))\n",
    );
    assert_eq!(app(source), Vec::<String>::new());
}

#[test]
fn a_callable_that_does_not_fit_the_solved_types_is_refused() {
    let source = concat!(
        "from collections.abc import Callable\n",
        "def apply[T](value: T, change: Callable[[T], T]) -> T:\n",
        "    return change(value)\n",
        "def increment(number: int) -> int:\n",
        "    return number + 1\n",
        "print(apply('a', increment))\n",
    );
    assert_eq!(app(source), ["E0404"]);
}

#[test]
fn calls_that_give_no_fitting_type_are_refused() {
    let cases: &[(&str, &str)] = &[
        (
            "def f[T](x: T) -> T:\n    return x\nprint(f(missing))\n",
            "E0205",
        ),
        ("def f[T](x: T) -> T:\n    return x\nprint(f())\n", "E0404"),
        (
            "def f[T](x: T) -> T:\n    return x\nprint(f(1, 2))\n",
            "E0404",
        ),
        (
            "def f[T](x: list[T]) -> T:\n    return x[0]\nprint(f(1))\n",
            "E0404",
        ),
        (
            "def f[T](x: T) -> T:\n    return x\ny: str = f(1)\n",
            "E0402",
        ),
        // An operation other than a comparison of a type parameter stays refused (GEN-18).
        ("def f[T](x: T) -> T:\n    return x + x\n", "E0401"),
        // A tuple of another length, a value that is not iterable, a union with two open
        // members, and a member that takes the argument itself solve nothing.
        (
            "def f[A, B](x: tuple[A, B]) -> A:\n    return x[0]\nprint(f((1, 2, 3)))\n",
            "E0404",
        ),
        (
            "from collections.abc import Iterable\ndef f[T](x: Iterable[T]) -> int:\n    return 0\nprint(f(1))\n",
            "E0404",
        ),
        (
            "def f[A, B](x: A | B) -> int:\n    return 0\nprint(f(1))\n",
            "E0404",
        ),
        (
            "def f[T](x: T | str) -> int:\n    return 0\nprint(f('s'))\n",
            "E0404",
        ),
        // A generic method overrides or is overridden by one with as many type parameters
        // alone (GEN-09, GEN-16).
        (
            "class B:\n    def m(self, x: int) -> int:\n        return x\nclass C(B):\n    def m[T](self, x: T) -> T:\n        return x\n",
            "E0201",
        ),
        (
            "class B:\n    def m[T](self, x: T) -> T:\n        return x\nclass C(B):\n    def m(self, x: int) -> int:\n        return x\n",
            "E0201",
        ),
        (
            "class C:\n    @classmethod\n    def m[T](cls, x: T) -> T:\n        return x\nprint(C().m(1))\n",
            "E0201",
        ),
        (
            "class C:\n    def __eq__[T](self, other: T) -> bool:\n        return True\n",
            "E0201",
        ),
        (
            "class C:\n    def m[T](self, x: T) -> T:\n        return x\nprint(C().m())\n",
            "E0404",
        ),
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

#[test]
fn a_class_method_of_a_generic_class_takes_its_type_arguments_from_the_call() {
    let class = concat!(
        "class Box[T]:\n",
        "    def __init__(self, item: T) -> None:\n",
        "        self.item = item\n",
        "    @classmethod\n",
        "    def name(cls) -> str:\n",
        "        return 'box'\n",
        "    @staticmethod\n",
        "    def note() -> int:\n",
        "        local: list[T] = []\n",
        "        return len(local)\n",
        "    @classmethod\n",
        "    def make(cls, item: T) -> 'Box[T]':\n",
        "        return cls(item)\n",
        "    def again(self) -> 'Box[T]':\n",
        "        return Box.make(self.item)\n",
    );
    let cases: &[(&str, &[&str])] = &[
        ("print(Box[int].name(), Box[str].note())\n", &[]),
        ("print(Box.name())\n", &["E0404"]),
        ("print(Box.note())\n", &["E0404"]),
        // Through an instance of a copy, a class method has no bridge (CLS-30).
        ("print(Box(1).name())\n", &["E0201"]),
        // A method of the generic class through the class takes the instance's copy.
        ("print(Box.__init__(Box(1), 2))\n", &[]),
        ("print(Box.__init__(5, 2))\n", &["E0404"]),
        (
            "class Other:\n    pass\nprint(Box.__init__(Other(), 2))\n",
            &["E0404"],
        ),
    ];
    for (body, expected) in cases {
        assert_eq!(app(&format!("{class}{body}")), *expected, "{body}");
    }
}

#[test]
fn every_copy_of_a_generic_class_shares_its_class_variables() {
    let accepted = concat!(
        "from typing import ClassVar\n",
        "class Counter[T]:\n",
        "    made = 0\n",
        "    kind: ClassVar[str] = 'counter'\n",
        "    size: int = 3\n",
        "    def __init__(self, value: T) -> None:\n",
        "        self.value = value\n",
        "        Counter.made += 1\n",
        "a = Counter(1)\n",
        "b: Counter[str] = Counter('x')\n",
        "print(Counter.made + a.made + b.size, a.kind + b.kind)\n",
    );
    assert_eq!(app(accepted), Vec::<String>::new());
    let refused = [
        "from typing import ClassVar\nclass Box[T]:\n    shared: ClassVar[list[T]] = []\n",
        "class Box[T]:\n    last: T | None = None\n    def __init__(self, item: T) -> None:\n        self.item = item\nprint(Box(1).last)\n",
        "class Box[T]:\n    last: 'list[T]' = []\n",
    ];
    for source in refused {
        assert_eq!(app(source), vec!["E0201".to_string()], "{source}");
    }
}

#[test]
fn a_generic_class_derives_from_a_class_of_the_program() {
    let base = concat!(
        "class Base:\n",
        "    def __init__(self, name: str) -> None:\n",
        "        self.name = name\n",
        "    def get(self) -> int:\n",
        "        return 1\n",
        "class Box[T](Base):\n",
        "    def __init__(self, item: T) -> None:\n",
        "        super().__init__('box')\n",
        "        self.item = item\n",
        "    def held(self) -> T:\n",
        "        return self.item\n",
    );
    let cases: &[(&str, &[&str])] = &[
        (
            "def name(b: Base) -> str:\n    return b.name\nprint(name(Box(1)), Box('s').get())\n",
            &[],
        ),
        ("items: list[Base] = [Box(1), Box('a'), Base('b')]\n", &[]),
        // The variable keeps its type, as the object's copy is known only when it runs.
        (
            "def f(b: Base) -> None:\n    if isinstance(b, Box):\n        print(b.held())\n",
            &["E0414"],
        ),
        (
            "class Wrong[T](Base):\n    def __init__(self, item: T) -> None:\n        super().__init__('w')\n        self.item = item\n    def get(self) -> T:\n        return self.item\n",
            &["E0404"],
        ),
        ("class Failure[T](Exception):\n    pass\n", &["E0201"]),
        (
            "from typing import Protocol\nclass Getter[T](Protocol):\n    def get(self) -> T: ...\n",
            &["E0201"],
        ),
        (
            "from dataclasses import dataclass\n@dataclass\nclass P:\n    x: int\n@dataclass\nclass Q[T](P):\n    y: T\n",
            &["E0201"],
        ),
        (
            "from dataclasses import dataclass\n@dataclass\nclass P:\n    x: int\n@dataclass(eq=False)\nclass Q[T](P):\n    y: T\nprint(Q(1, 'a').y)\n",
            &[],
        ),
    ];
    for (body, expected) in cases {
        assert_eq!(app(&format!("{base}{body}")), *expected, "{body}");
    }
}

#[test]
fn a_method_of_a_generic_class_takes_type_parameters_of_its_own() {
    let classes = concat!(
        "class Box[T]:\n",
        "    def __init__(self, value: T) -> None:\n",
        "        self.value = value\n",
        "    def paired[U](self, other: U) -> tuple[T, U]:\n",
        "        return (self.value, other)\n",
        "    @staticmethod\n",
        "    def both[U](first: U, second: U) -> list[U]:\n",
        "        return [first, second]\n",
        "    @staticmethod\n",
        "    def tagged[U](value: T, tag: U) -> tuple[T, U]:\n",
        "        return (value, tag)\n",
    );
    let cases: &[(&str, &[&str])] = &[
        ("x: tuple[int, str] = Box(1).paired('a')\n", &[]),
        ("print(Box.both(1, 2), Box[str].both(1.5, 2.5))\n", &[]),
        ("x: tuple[int, str] = Box.tagged(1, 'a')\n", &[]),
        ("y: tuple[str, int] = Box[str].tagged('a', 1)\n", &[]),
        ("print(Box(1).paired())\n", &["E0404"]),
        ("print(Box[str].tagged(1, 'a'))\n", &["E0404"]),
    ];
    for (body, expected) in cases {
        assert_eq!(app(&format!("{classes}{body}")), *expected, "{body}");
    }
}

#[test]
fn a_generic_method_overrides_one_with_as_many_type_parameters() {
    let base = concat!(
        "class B:\n",
        "    def m[T](self, x: T) -> list[T]:\n",
        "        return [x]\n",
    );
    let cases: &[(&str, &[&str])] = &[
        (
            "class C(B):\n    def m[U](self, x: U) -> list[U]:\n        return super().m(x) + [x]\ndef f(b: B) -> int:\n    return len(b.m(1)) + len(b.m('a'))\nprint(f(C()))\n",
            &[],
        ),
        (
            "class C(B):\n    def m[T](self, x: T) -> T:\n        return x\n",
            &["E0404"],
        ),
        (
            "class C(B):\n    def m(self, x: int) -> list[int]:\n        return [x]\n",
            &["E0201"],
        ),
        (
            "class C(B):\n    def m[T, U](self, x: T) -> list[T]:\n        return [x]\n",
            &["E0201"],
        ),
    ];
    for (body, expected) in cases {
        assert_eq!(app(&format!("{base}{body}")), *expected, "{body}");
    }
}

#[test]
fn a_method_of_a_copy_made_early_assigns_its_attributes() {
    // `Box[int]` in a signature makes the copy before `Box.__init__` is checked.
    let source = concat!(
        "class Box[T]:\n",
        "    def __init__(self, item: T) -> None:\n",
        "        self.item = item\n",
        "    def put(self, item: T) -> None:\n",
        "        self.item = item\n",
        "def make(b: Box[int]) -> int:\n",
        "    b.put(2)\n",
        "    return b.item\n",
        "print(make(Box(5)))\n",
    );
    assert_eq!(app(source), Vec::<String>::new());
}

#[test]
fn a_class_derives_from_a_copy_of_a_generic_class() {
    let classes = concat!(
        "class Box[T]:\n",
        "    def __init__(self, item: T) -> None:\n",
        "        self.item = item\n",
        "    def get(self) -> T:\n",
        "        return self.item\n",
        "    def put(self, item: T) -> None:\n",
        "        self.item = item\n",
    );
    let cases: &[(&str, &[&str])] = &[
        (
            "class IntBox(Box[int]):\n    def get(self) -> int:\n        return super().get() + 1\nb: Box[int] = IntBox(1)\nprint(b.get() + IntBox(2).item)\n",
            &[],
        ),
        (
            "class Pair[T](Box[T]):\n    def __init__(self, item: T, other: T) -> None:\n        super().__init__(item)\n        self.other = other\n    def put(self, item: T) -> None:\n        self.other = item\nprint(Pair(1, 2).get() + Pair(3, 4).other)\n",
            &[],
        ),
        (
            "class Many[T](Box[list[T]]):\n    def first(self) -> T:\n        return self.get()[0]\nprint(Many([1.5]).first() + 1.0)\n",
            &[],
        ),
        // An override fits the method the copy has, with its type arguments.
        (
            "class IntBox(Box[int]):\n    def get(self) -> str:\n        return ''\n",
            &["E0404"],
        ),
        (
            "class IntBox(Box[int]):\n    def put(self, item: str) -> None:\n        pass\n",
            &["E0404"],
        ),
        (
            "class StrBox(Box[str]):\n    pass\nprint(StrBox(1))\n",
            &["E0404"],
        ),
        ("class Later(Box[Missing]):\n    pass\n", &["E0205"]),
    ];
    for (body, expected) in cases {
        assert_eq!(app(&format!("{classes}{body}")), *expected, "{body}");
    }
}

#[test]
fn a_generic_of_another_module_is_copied_in_its_own_names() {
    let tools = concat!(
        "SCALE = 10\n",
        "class Box[T]:\n",
        "    def __init__(self, item: T) -> None:\n",
        "        self.item = item\n",
        "def _twice[T](item: T) -> list[T]:\n",
        "    return [item, item]\n",
        "def doubled[T](item: T) -> list[T]:\n",
        "    return _twice(item)\n",
        "def scaled[N: (int, float)](value: N) -> N:\n",
        "    return value * SCALE\n",
    );
    let uses = [
        "from tools import doubled\nprint(doubled(1)[0] + 1)\n",
        "import tools\nprint(tools.scaled(2.5) + 1.0)\n",
        "from tools import Box\nb: Box[str] = Box('x')\nprint(b.item.upper())\n",
        "import tools\ndef show(box: tools.Box[int]) -> int:\n    return box.item\nprint(show(tools.Box(3)))\n",
    ];
    for app in uses {
        assert_eq!(
            codes(&[("app.py", app), ("tools.py", tools)]),
            Vec::<String>::new(),
            "{app}"
        );
    }
    let refused = codes(&[
        ("app.py", "import tools\nprint(tools.scaled('x'))\n"),
        ("tools.py", tools),
    ]);
    assert_eq!(refused, vec!["E0404".to_string()]);
}

#[test]
fn generic_classes_check_their_type_arguments() {
    let classes = concat!(
        "class Shape:\n    pass\n",
        "class Box[T]:\n    def __init__(self, value: T) -> None:\n        self.value = value\n",
        "class Held[S: Shape]:\n    def __init__(self, shape: S) -> None:\n        self.shape = shape\n",
        "class Either[N: (int, str)]:\n    def __init__(self, value: N) -> None:\n        self.value = value\n",
    );
    let accepted = [
        "def keep[S: Shape](held: Held[S]) -> S:\n    return held.shape\nprint(keep(Held(Shape())))\n",
        "x: Either[int] = Either(1)\nprint(x.value)\n",
        "def f[U](b: Box[U]) -> Box[U]:\n    return b\nprint(f(Box(1)).value, Box[str]('a').value)\n",
    ];
    for body in accepted {
        assert_eq!(
            app(&format!("{classes}{body}")),
            Vec::<String>::new(),
            "{body}"
        );
    }
    let refused: &[(&str, &str)] = &[
        ("x: Box[int, str] = Box(1)\n", "E0408"),
        ("x: Box[missing] = Box(1)\n", "E0205"),
        ("x: Held[int] = Held(Shape())\n", "E0408"),
        ("x: Either[float] = Either(1)\n", "E0408"),
        ("x: Box = Box(1)\n", "E0408"),
        ("class Box2[T]:\n    count: T | None = None\n", "E0201"),
        ("class Sub(Box):\n    pass\n", "E0408"),
        ("class Sub(list[int]):\n    pass\n", "E0201"),
        ("@decorated\nclass Box3[T]:\n    pass\n", "E0201"),
    ];
    for (body, code) in refused {
        let found = app(&format!("{classes}{body}"));
        assert_eq!(
            found.first().map(String::as_str),
            Some(*code),
            "{body}\n{found:?}"
        );
    }
}

#[test]
fn fun_10_nested_code_of_a_generic_function_is_checked_in_each_copy() {
    let source = concat!(
        "from collections.abc import Callable, Iterator\n",
        "def pair_with[T](first: T) -> Callable[[T], tuple[T, T]]:\n",
        "    def make(second: T) -> tuple[T, T]:\n",
        "        return (first, second)\n",
        "    return make\n",
        "def doubled[T](values: list[T]) -> Iterator[tuple[T, T]]:\n",
        "    return ((value, value) for value in values)\n",
        "def same[T](value: T) -> Callable[[], T]:\n",
        "    return lambda: value\n",
        "print(pair_with(1)(2), pair_with('a')('b'), list(doubled([1])), same(2.5)())\n",
    );
    assert_eq!(app(source), Vec::<String>::new());
    // An error in the nested code is reported once, from the check with opaque types.
    let wrong = concat!(
        "def pair_with[T](first: T) -> int:\n",
        "    def make(second: T) -> T:\n",
        "        return first + second\n",
        "    return 1\n",
        "print(pair_with(1), pair_with('a'))\n",
    );
    assert_eq!(app(wrong), ["E0401"]);
}

#[test]
fn gen_18_a_type_parameter_compares_in_each_copy() {
    let source = "def first_over[T](items: list[T], limit: T) -> int:\n    \
                  for index, item in enumerate(items):\n        if limit < item:\n            \
                  return index\n    return len(items)\n\n\n\
                  def largest[T](items: list[T]) -> T:\n    return max(sorted(items))\n\n\n\
                  print(first_over([1, 5], 4), first_over([(1, 'a')], (0, 'b')), largest(['b', 'a']))\n";
    assert_eq!(app(source), Vec::<String>::new());
    // A copy whose types do not order is refused at the comparison.
    let refused = "def first_over[T](items: list[T], limit: T) -> bool:\n    \
                   return limit < items[0]\n\n\nprint(first_over([{'a': 1}], {'b': 2}))\n";
    assert_eq!(app(refused), ["E0407"]);
}
