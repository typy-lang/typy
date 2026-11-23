//! `typing.Self`: in a method's parameters and result, the class the method is called
//! through (TYP-30, ADR-0113).

use crate::common::{app, messages};

const SHAPE: &str = "from typing import Self\n\n\nclass Shape:\n    \
                     def __init__(self, size: float) -> None:\n        self.size = size\n\n    \
                     def grown(self, factor: float) -> Self:\n        \
                     self.size = self.size * factor\n        return self\n\n    \
                     def same(self, other: Self) -> bool:\n        \
                     return self.size == other.size\n\n    @classmethod\n    \
                     def unit(cls) -> Self:\n        return cls(1.0)\n\n\n\
                     class Circle(Shape):\n    def label(self) -> str:\n        \
                     return 'circle'\n\n\n";

#[test]
fn typ_30_a_call_gives_the_class_it_is_made_through() {
    let accepted = [
        format!("{SHAPE}c: Circle = Circle(1.0).grown(2.0)\nprint(c.label())\n"),
        format!("{SHAPE}u: Circle = Circle.unit()\ns: Shape = Shape.unit()\nprint(u.label(), s)\n"),
        format!("{SHAPE}print(Circle(1.0).same(Circle(2.0)), Shape.same(Shape(1.0), Circle(1.0)))\n"),
        // `Self | None`, `self` through another method, and a conditional of two.
        "from typing import Self\n\n\nclass Node:\n    def first(self, keep: bool) -> Self | None:\n        \
         if keep:\n            return self\n        return None\n\n    \
         def again(self) -> Self:\n        return self.chained()\n\n    \
         def chained(self) -> Self:\n        return self\n\n    \
         def either(self, other: Self, mine: bool) -> Self:\n        \
         return self if mine else other\n\n\nnode = Node()\nprint(node.first(False), node.again(), node.either(node, True))\n"
            .to_owned(),
    ];
    for source in accepted {
        assert_eq!(app(&source), Vec::<String>::new(), "{source}");
    }
}

#[test]
fn typ_30_an_instance_of_another_class_is_no_self() {
    let found = messages(
        &format!("{SHAPE}c: Circle = Circle(1.0).grown(2.0)\n")
            .replace("        return self\n", "        return Shape(1.0)\n"),
    );
    assert!(
        found
            .iter()
            .any(|(code, _, rule)| code == "E0433" && rule == "TYP-30"),
        "{found:?}"
    );
    let found = messages(&format!("{SHAPE}print(Circle(1.0).same(Shape(2.0)))\n"));
    assert_eq!(found.len(), 1, "{found:?}");
    assert_eq!(found[0].0, "E0404", "{found:?}");
    // A call of a method returning `Self` with an argument it refuses gives that error alone.
    let found = messages(&format!("{SHAPE}print(Circle(1.0).grown('2'))\n"));
    assert_eq!(found.len(), 1, "{found:?}");
    assert_eq!(found[0].0, "E0404", "{found:?}");
    // Either branch of a conditional must be the instance.
    let found = messages(
        &format!("{SHAPE}c: Circle = Circle(1.0).grown(2.0)\n").replace(
            "        return self\n",
            "        return Shape(1.0) if factor else self\n",
        ),
    );
    assert!(
        found.iter().any(|(code, _, _)| code == "E0433"),
        "{found:?}"
    );
}

#[test]
fn typ_30_self_elsewhere_is_planned() {
    let cases = [
        "from typing import Self\n\n\nclass Node:\n    def listed(self) -> list[Self]:\n        \
         return [self]\n",
        "from typing import Self\n\n\nclass Node:\n    @staticmethod\n    def make() -> Self:\n        \
         return Node()\n",
        "from typing import Self\n\nvalue: Self = 1\n",
        "from typing import Self\n\n\nclass Box[T]:\n    def same(self) -> Self:\n        \
         return self\n",
    ];
    for source in cases {
        let found = messages(source);
        assert!(
            found
                .iter()
                .any(|(code, _, rule)| code == "E0201" && rule == "TYP-36"),
            "{source}\n{found:?}"
        );
    }
}
