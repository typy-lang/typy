//! The type object `type(x)` gives for an instance whose class has subclasses, which the
//! program reads when it runs (CLS-12, CLS-39, ADR-0084).

use crate::common::{app, codes, messages};

const SHAPES: &str = "class Shape:\n    kind = 'shape'\n    @classmethod\n    def make(cls) -> int:\n        return 1\n\
                      class Circle(Shape):\n    kind = 'circle'\n\
                      shape: Shape = Circle()\n";

const PLANNED: &str = "calling a class known only when the program runs, such as `type(x)` \
                       of an instance whose class has subclasses, is planned for TyPy 0.14 \
                       (CLS-39)";

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
fn class_members_through_type_of_such_an_instance_are_refused() {
    for statement in ["print(type(shape).kind)", "print(type(shape).make())"] {
        let source = format!("{SHAPES}{statement}\n");
        assert_eq!(app(&source), ["E0412"], "{statement}");
    }
}

#[test]
fn type_of_such_an_instance_is_no_type_of_its_static_class() {
    let source =
        format!("{SHAPES}def build(kind: type[Shape]) -> None:\n    pass\nbuild(type(shape))\n");
    assert_eq!(app(&source), ["E0404"]);
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
