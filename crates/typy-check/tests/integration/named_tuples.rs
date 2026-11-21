//! Classes derived from `typing.NamedTuple`: their fields read by name and as a tuple, the
//! errors CPython raises when the class statement runs, and the forms planned for 1.0
//! (CLS-27, CLS-41, ADR-0088).

use crate::common::{app, messages};

/// A named tuple with a default, for the cases below.
const POINT: &str = "\
from typing import NamedTuple


class Point(NamedTuple):
    x: int
    y: int = 0


";

/// Returns the one error of `POINT` followed by `source`: its code, message and rule.
fn only_error(source: &str) -> (String, String, String) {
    let found = messages(&format!("{POINT}{source}"));
    assert_eq!(found.len(), 1, "{found:?}");
    found.into_iter().next().unwrap()
}

/// Returns the message of the one error of the class `body` derived from `NamedTuple`.
fn class_error(body: &str) -> (String, String) {
    let found = messages(&format!(
        "from typing import NamedTuple, ClassVar\n\n\nclass P(NamedTuple):\n{body}"
    ));
    assert_eq!(found.len(), 1, "{found:?}");
    let (code, message, _) = found.into_iter().next().unwrap();
    (code, message)
}

#[test]
fn cls_27_a_named_tuple_reads_its_fields_by_name_and_as_a_tuple() {
    let source = format!(
        "{POINT}p = Point(3)\nq = Point(x=1, y=5)\nx, y = p\n\
         print(p.x, q.y, p[0], p[-1], p[0:1], len(p), 3 in p, list(p), tuple(p))\n\
         for item in p:\n    print(item)\n\
         print(p == (3, 0), (3, 0) != p, p < q, p == q, p + (9,), Point._fields)\n\
         print(sorted([p, q]), {{p: 1}}, isinstance(p, tuple), max(p))\n\
         match p:\n    case (a, b):\n        print(a, b)\n\
         match p:\n    case Point(x=3, y=b):\n        print(b)\n"
    );
    assert_eq!(app(&source), Vec::<String>::new());
}

#[test]
fn cls_27_a_named_tuple_takes_members_subclasses_and_type_parameters() {
    let source = format!(
        "{POINT}class Labelled(Point):\n    def label(self) -> str:\n        return str(self.x)\n\n\n\
         class Pair[T](NamedTuple):\n    first: T\n    second: T\n\n\n\
         print(Labelled(1).label(), Labelled(1, 2) == (1, 2), Pair(1, 2).first, Pair('a', 'b'))\n"
    );
    assert_eq!(app(&source), Vec::<String>::new());
    let through_module =
        "import typing\n\n\nclass P(typing.NamedTuple):\n    x: int\n\n\nprint(P(1))\n";
    assert_eq!(app(through_module), Vec::<String>::new());
}

#[test]
fn cls_27_each_error_of_the_class_statement_has_cpythons_text() {
    let cases = [
        (
            "    x: int = 0\n    y: int\n",
            "Non-default namedtuple field y cannot follow default field x, as CPython raises \
             `TypeError` when the class statement runs",
        ),
        (
            "    _x: int\n",
            "Field names cannot start with an underscore: '_x', as CPython raises `ValueError` \
             when the class statement runs",
        ),
        (
            "    x: int\n\n    def _replace(self) -> None:\n        pass\n",
            "Cannot overwrite NamedTuple attribute _replace, as CPython raises `AttributeError` \
             when the class statement runs",
        ),
        (
            "    x: int\n\n    def __init__(self) -> None:\n        pass\n",
            "Cannot overwrite NamedTuple attribute __init__, as CPython raises `AttributeError` \
             when the class statement runs",
        ),
        (
            "    o: ClassVar[int] = 7\n",
            "the field `o` cannot be a `ClassVar`, as CPython raises `TypeError` when the class \
             statement runs",
        ),
    ];
    for (body, message) in cases {
        assert_eq!(
            class_error(body),
            ("E0404".to_owned(), message.to_owned()),
            "{body}"
        );
    }
}

#[test]
fn cls_27_a_second_base_is_reported_once() {
    let found =
        messages("from typing import NamedTuple\n\n\nclass P(NamedTuple, object):\n    x: int\n");
    assert_eq!(
        found,
        [(
            "E0404".to_owned(),
            "can only inherit from a NamedTuple type and Generic, as CPython raises `TypeError` \
             when the class statement runs"
                .to_owned(),
            "CLS-27".to_owned(),
        )]
    );
}

#[test]
fn cls_27_a_field_is_not_assigned() {
    let expected = (
        "E0414".to_owned(),
        "`Point` is a named tuple, so its field `x` is not assigned, as CPython raises \
         `AttributeError: can't set attribute`"
            .to_owned(),
        "CLS-27".to_owned(),
    );
    assert_eq!(only_error("p = Point(1)\np.x = 5\n"), expected);
    let in_a_method = "class Moved(Point):\n    def move(self) -> None:\n        self.x = 2\n";
    let mut moved = expected;
    moved.1 = moved.1.replace("`Point`", "`Moved`");
    assert_eq!(only_error(in_a_method), moved);
}

#[test]
fn cls_27_named_tuple_is_no_type() {
    let found = messages(
        "from typing import NamedTuple\n\n\ndef f(value: NamedTuple) -> None:\n    pass\n",
    );
    assert_eq!(found.len(), 1, "{found:?}");
    assert_eq!(found[0].0, "E0408");
    assert_eq!(found[0].2, "CLS-27");
}

#[test]
fn cls_27_a_named_tuple_is_no_plain_tuple() {
    let (code, _, _) =
        only_error("def f(pair: tuple[int, int]) -> None:\n    pass\n\n\nf(Point(1))\n");
    assert_eq!(code, "E0404");
}

#[test]
fn cls_41_what_a_derived_class_adds_is_planned() {
    let cases = [
        (
            "class Q(Point):\n    def __init__(self) -> None:\n        pass\n",
            "a class derived from a named tuple that defines `__init__` is planned for TyPy 1.0 \
             (CLS-41)",
        ),
        (
            "class Q(Point):\n    z: int\n",
            "a class derived from a named tuple with an attribute of its own is planned for TyPy \
             1.0 (CLS-41)",
        ),
        (
            "class Q(Point):\n    def __iter__(self) -> int:\n        return 1\n",
            "a class derived from a named tuple that defines `__iter__` is planned for TyPy 1.0 \
             (CLS-41)",
        ),
    ];
    for (source, message) in cases {
        assert_eq!(
            only_error(source),
            ("E0201".to_owned(), message.to_owned(), "CLS-41".to_owned()),
            "{source}"
        );
    }
    let found = class_error("    x: int\n\n    def __len__(self) -> int:\n        return 1\n");
    assert_eq!(
        found,
        (
            "E0201".to_owned(),
            "a named tuple that defines `__len__` is planned for TyPy 1.0 (CLS-41)".to_owned()
        )
    );
}
