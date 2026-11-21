//! The type object `type(x)` gives for an instance whose class has subclasses, which the
//! program reads when it runs (CLS-12, CLS-39, ADR-0084).

use crate::common::{app, codes, messages};

const SHAPES: &str = "class Shape:\n    kind = 'shape'\n    @classmethod\n    def make(cls) -> int:\n        return 1\n\
                      class Circle(Shape):\n    kind = 'circle'\n\
                      shape: Shape = Circle()\n";

const PLANNED: &str = "calling a class known only when the program runs, such as `type(x)` \
                       of an instance whose class has subclasses or a `type[C]` value that may \
                       hold a class derived from `C`, is planned for TyPy 0.14 (CLS-39)";

#[test]
fn calling_type_of_an_instance_whose_class_has_subclasses_is_planned() {
    for statement in ["made = type(shape)()", "kind = type(shape)\nmade = kind()"] {
        let source = format!("{SHAPES}{statement}\n");
        assert_eq!(
            messages(&source),
            [("E0201".to_owned(), PLANNED.to_owned(), "CLS-39".to_owned())],
            "{statement}"
        );
    }
}

#[test]
fn class_members_through_type_of_such_an_instance_are_planned() {
    for (statement, member) in [
        ("print(type(shape).kind)", "kind"),
        ("print(type(shape).make())", "make"),
    ] {
        let source = format!("{SHAPES}{statement}\n");
        assert_eq!(
            messages(&source),
            [(
                "E0201".to_owned(),
                format!(
                    "`{member}` through a `type[Shape]` value that may hold a class derived from \
                     it is planned for TyPy 0.14 (CLS-39)"
                ),
                "CLS-39".to_owned()
            )],
            "{statement}"
        );
    }
}

#[test]
fn type_of_such_an_instance_is_a_type_of_its_static_class() {
    let source = format!(
        "{SHAPES}def build(kind: type[Shape]) -> str:\n    return kind.__name__\n\
         print(build(type(shape)), build(Circle))\n"
    );
    assert_eq!(app(&source), Vec::<String>::new());
}

#[test]
fn cls_18_a_class_is_held_where_a_base_is_and_calls_through_it_are_planned() {
    let held = format!(
        "{SHAPES}kinds: tuple[type[Shape], ...] = (Circle, Shape)\nchosen: type[Shape] = Circle\n\
         print(isinstance(shape, kinds), chosen is Circle, Circle.make(), Shape.make())\n"
    );
    assert_eq!(app(&held), Vec::<String>::new());
    for statement in [
        "def made(kind: type[Shape]) -> Shape:\n    return kind()\n",
        "chosen: type[Shape] = Circle\nprint(chosen())\n",
    ] {
        let source = format!("{SHAPES}{statement}");
        assert_eq!(
            messages(&source),
            [("E0201".to_owned(), PLANNED.to_owned(), "CLS-39".to_owned())],
            "{statement}"
        );
    }
    let in_class_method = "class Shape:\n    @classmethod\n    def make(cls) -> 'Shape':\n        \
                           return cls()\nclass Circle(Shape):\n    pass\nprint(Circle.make())\n";
    assert_eq!(app(in_class_method), Vec::<String>::new());
}

#[test]
fn type_of_such_an_instance_names_and_compares_its_class() {
    let source = format!(
        "{SHAPES}print(type(shape), type(shape).__name__, type(shape) is Circle, type(shape) == Shape)\n"
    );
    assert_eq!(app(&source), Vec::<String>::new());
}

#[test]
fn type_of_an_instance_whose_class_has_no_subclass_is_its_class() {
    let source = "class Shape:\n    kind = 'shape'\n    @classmethod\n    def make(cls) -> int:\n        return 1\n\
                  shape = Shape()\nkind = type(shape)\nprint(kind(), type(shape).kind, type(shape).make())\n";
    assert_eq!(app(source), Vec::<String>::new());
}

#[test]
fn a_subclass_in_a_module_checked_later_counts() {
    let found = codes(&[
        (
            "app.py",
            "import shapes\nimport circles\nprint(circles.Circle())\n",
        ),
        (
            "shapes.py",
            "class Shape:\n    pass\ndef copy(shape: Shape) -> Shape:\n    kind = type(shape)\n    return kind()\n",
        ),
        (
            "circles.py",
            "from shapes import Shape\nclass Circle(Shape):\n    pass\n",
        ),
    ]);
    assert_eq!(found, ["E0201"]);
}

#[test]
fn a_subclass_of_a_copy_of_a_generic_class_counts() {
    let source = "class Box[T]:\n    def __init__(self, item: T) -> None:\n        self.item = item\n\
                  class IntBox(Box[int]):\n    pass\n\
                  box: Box[int] = IntBox(1)\nkind = type(box)\nmade = kind(2)\n";
    assert_eq!(app(source), ["E0201"]);
}

#[test]
fn cls_18_a_class_pattern_takes_a_class_held_in_a_value() {
    let source = format!(
        "{SHAPES}def which(kind: type[Shape], value: Shape) -> str:\n    match value:\n        \
         case kind(kind='circle'):\n            return 'circle'\n        case kind():\n            \
         return 'kind'\n        case _:\n            return 'other'\nprint(which(Circle, shape))\n"
    );
    assert_eq!(app(&source), Vec::<String>::new());
    let cases = [
        (
            "kind = type(shape)\nmatch shape:\n    case kind(1):\n        pass\n",
            "E0201",
        ),
        (
            "kinds = (Shape,)\nmatch shape:\n    case kinds():\n        pass\n",
            "E0401",
        ),
    ];
    for (statement, code) in cases {
        let source = format!("{SHAPES}{statement}");
        assert_eq!(codes(&[("app.py", &source)]), [code], "{statement}");
    }
}
