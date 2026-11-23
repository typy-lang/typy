//! Tests of abstract classes through the public API (CLS-45): what `ABC` and `@abstractmethod`
//! accept, the classes that have no instances, and the calls that would run an abstract
//! method's empty body.

use crate::common::{app, codes, dump, messages};

const SHAPE: &str = "from abc import ABC, abstractmethod\nclass Shape(ABC):\n    @abstractmethod\n    def area(self) -> float: ...\n    @abstractmethod\n    def name(self) -> str: ...\n";

#[test]
fn abstract_classes_in_the_forms_cpython_takes_are_accepted() {
    let cases = [
        // A subclass that overrides every abstract method has instances.
        "class Square(Shape):\n    def area(self) -> float:\n        return 4.0\n    def name(self) -> str:\n        return 'square'\nprint(Square().area())\n",
        // An abstract property, overridden by a property.
        "class Sided(ABC):\n    @property\n    @abstractmethod\n    def sides(self) -> int: ...\nclass Triangle(Sided):\n    @property\n    def sides(self) -> int:\n        return 3\nprint(Triangle().sides)\n",
        // An abstract method with a body runs through `super()`.
        "class Base(ABC):\n    @abstractmethod\n    def run(self) -> int:\n        return 1\nclass Runner(Base):\n    def run(self) -> int:\n        return super().run() + 1\n",
        // An empty body whose result holds `None` returns it, as CPython's does.
        "class Base(ABC):\n    @abstractmethod\n    def note(self) -> str | None: ...\n    @abstractmethod\n    def reset(self) -> None:\n        pass\nclass Runner(Base):\n    def note(self) -> str | None:\n        return super().note()\n    def reset(self) -> None:\n        super().reset()\n",
        // Abstract static and class methods that give a value.
        "class Base(ABC):\n    @staticmethod\n    @abstractmethod\n    def zero() -> int:\n        raise NotImplementedError\n    @classmethod\n    @abstractmethod\n    def make(cls) -> int:\n        raise NotImplementedError\n",
        // `abc.ABC` and `abc.abstractmethod` through the module, under another name.
        "import abc as a\nclass Base(a.ABC):\n    @a.abstractmethod\n    def run(self) -> int: ...\nclass Runner(Base):\n    def run(self) -> int:\n        return 2\nprint(Runner().run())\n",
        // `ABC` beside another base, and a class with no abstract methods.
        "class Plain:\n    pass\nclass Both(Plain, ABC):\n    pass\nprint(Both())\n",
        // A dataclass and a generic class derived from `ABC`.
        "from dataclasses import dataclass\n@dataclass\nclass Point(ABC):\n    x: int\n    @abstractmethod\n    def norm(self) -> int: ...\n@dataclass\nclass Flat(Point):\n    def norm(self) -> int:\n        return self.x\nclass Store[T](ABC):\n    @abstractmethod\n    def get(self) -> T: ...\nclass Ints(Store[int]):\n    def get(self) -> int:\n        return 1\nprint(Flat(1).norm(), Ints().get())\n",
        // Without `ABC`, `@abstractmethod` changes nothing, as without `ABCMeta`.
        "class Plain:\n    @abstractmethod\n    def run(self) -> None: ...\n@abstractmethod\ndef helper() -> None: ...\nPlain().run()\n",
    ];
    for source in cases {
        assert_eq!(
            app(&format!("{SHAPE}{source}")),
            Vec::<String>::new(),
            "{source}"
        );
    }
}

#[test]
fn a_class_with_an_abstract_method_left_has_no_instances() {
    let cases: &[&str] = &[
        "Shape()\n",
        "class Half(Shape):\n    def area(self) -> float:\n        return 1.0\nHalf()\n",
        // An abstract method made again in a subclass is abstract there.
        "class Full(Shape):\n    def area(self) -> float:\n        return 1.0\n    def name(self) -> str:\n        return ''\nclass Again(Full):\n    @abstractmethod\n    def area(self) -> float: ...\nAgain()\n",
        // A subclass of a copy of a generic abstract class.
        "class Store[T](ABC):\n    @abstractmethod\n    def get(self) -> T: ...\nclass Partial(Store[int]):\n    pass\nPartial()\n",
    ];
    for source in cases {
        assert_eq!(app(&format!("{SHAPE}{source}")), ["E0404"], "{source}");
    }
}

#[test]
fn the_refusal_names_the_methods_left_as_cpython_sorts_them() {
    assert_eq!(
        messages(&format!("{SHAPE}Shape()\n")),
        [(
            "E0404".to_owned(),
            "`Shape` leaves the abstract methods `area`, `name` without an override, so it has \
             no instances; CPython raises `TypeError`"
                .to_owned(),
            "CLS-45".to_owned(),
        )]
    );
    let half = "class Half(Shape):\n    def name(self) -> str:\n        return ''\nHalf()\n";
    assert_eq!(
        messages(&format!("{SHAPE}{half}"))[0].1,
        "`Half` leaves the abstract method `area` without an override, so it has no instances; \
         CPython raises `TypeError`"
    );
}

#[test]
fn abstract_methods_carry_into_classes_of_other_modules() {
    let base = "from abc import ABC, abstractmethod\nclass Shape(ABC):\n    @abstractmethod\n    def area(self) -> float: ...\n";
    assert_eq!(
        codes(&[
            (
                "app.py",
                "from shapes import Shape\nclass Half(Shape):\n    pass\nHalf()\n"
            ),
            ("shapes.py", base),
        ]),
        ["E0404"]
    );
    assert_eq!(
        codes(&[
            (
                "app.py",
                "import shapes\nclass Unit(shapes.Shape):\n    def area(self) -> float:\n        return 1.0\nprint(Unit().area())\n"
            ),
            ("shapes.py", base),
        ]),
        Vec::<String>::new()
    );
}

#[test]
fn a_direct_call_of_an_empty_abstract_method_is_refused() {
    let half = "class Half(Shape):\n    def name(self) -> str:\n        return ''\n";
    let cases = [
        "    def area(self) -> float:\n        return super().area()\n",
        "    def area(self) -> float:\n        return Shape.area(self)\n",
        "    def area(self) -> float:\n        f = Shape.area\n        return 1.0\n",
    ];
    for case in cases {
        let found = messages(&format!("{SHAPE}{half}{case}"));
        assert_eq!(found.len(), 1, "{case}");
        assert_eq!(found[0].0, "E0434", "{case}");
        assert_eq!(
            found[0].1,
            "`Shape.area` is abstract and its body gives no value, so running it directly \
             returns `None` in CPython where a float is declared",
            "{case}"
        );
    }
}

#[test]
fn abstractmethod_above_a_method_decorator_is_refused() {
    let cases = [
        (
            "@abstractmethod\n    @property\n    def size(self) -> int: ...\n",
            "property",
        ),
        (
            "@abstractmethod\n    @staticmethod\n    def size() -> int: ...\n",
            "staticmethod",
        ),
        (
            "@abstractmethod\n    @classmethod\n    def size(cls) -> int: ...\n",
            "classmethod",
        ),
    ];
    for (decorators, kind) in cases {
        let source =
            format!("from abc import ABC, abstractmethod\nclass Sized(ABC):\n    {decorators}");
        assert_eq!(
            messages(&source),
            [(
                "E0434".to_owned(),
                format!(
                    "`@abstractmethod` is applied to a `{kind}` object here, and CPython raises \
                     `AttributeError` when the class is defined"
                ),
                "CLS-45".to_owned(),
            )],
            "{source}"
        );
    }
    let setter = "from abc import ABC, abstractmethod\nclass Sized(ABC):\n    @property\n    def size(self) -> int:\n        return 1\n    @abstractmethod\n    @size.setter\n    def size(self, value: int) -> None: ...\n";
    assert_eq!(app(setter), ["E0434"]);
}

#[test]
fn an_empty_body_that_cpython_runs_still_needs_a_value() {
    let cases = [
        // A static method is called through its class.
        "from abc import ABC, abstractmethod\nclass Base(ABC):\n    @staticmethod\n    @abstractmethod\n    def zero() -> int: ...\n",
        // An exception class makes its instances without the test of abstract methods.
        "from abc import ABC, abstractmethod\nclass Failure(Exception, ABC):\n    @abstractmethod\n    def code(self) -> int: ...\n",
        // Without `ABC` the method is an ordinary one.
        "from abc import abstractmethod\nclass Plain:\n    @abstractmethod\n    def code(self) -> int: ...\n",
    ];
    for source in cases {
        assert_eq!(app(source), ["E0405"], "{source}");
    }
}

#[test]
fn names_of_abc_outside_bases_and_decorators_are_refused() {
    let cases = [
        ("from abc import ABCMeta\n", "E0425"),
        ("from abc import ABC\nx: ABC\n", "E0408"),
        ("from abc import ABC\nprint(ABC)\n", "E0202"),
        ("import abc\nprint(abc.abstractmethod)\n", "E0202"),
    ];
    for (source, code) in cases {
        assert_eq!(app(source), [code], "{source}");
    }
    assert_eq!(
        messages("from abc import get_cache_token\n")[0].1,
        "TyPy's `abc` gives `ABC` and `abstractmethod`, not `get_cache_token`"
    );
}

#[test]
fn a_call_through_the_class_value_raises_with_cpython_s_message() {
    let source = "from abc import ABC, abstractmethod\nclass Shape(ABC):\n    @abstractmethod\n    def area(self) -> float: ...\n    @classmethod\n    def make(cls) -> 'Shape':\n        return cls()\n";
    assert!(
        dump(source).contains(
            "Can't instantiate abstract class Shape without an implementation for abstract \
             method 'area'"
        ),
        "{source}"
    );
    // Arguments its `__init__` refuses are reported as for any class.
    let refused = source.replace("return cls()", "return cls(1)");
    assert_eq!(app(&refused), ["E0404"]);
}

#[test]
fn other_decorators_beside_abstractmethod_follow_their_own_rules() {
    let cases = [
        "from abc import ABC, abstractmethod\nimport functools\nclass Base(ABC):\n    @abstractmethod\n    @functools.cache\n    def run(self) -> int: ...\n",
        "from abc import ABC, abstractmethod\nclass Base(ABC):\n    @abstractmethod\n    @other(1)\n    def run(self) -> int: ...\n",
        "from abc import ABC, abstractmethod\nimport abc\nclass Base(ABC):\n    @abc.abc.abstractmethod\n    def run(self) -> int: ...\n",
    ];
    for source in cases {
        let found = app(source);
        assert!(!found.is_empty(), "{source}");
        assert!(!found.contains(&"E0434".to_owned()), "{source}: {found:?}");
    }
}
