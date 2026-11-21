//! `typing.cast(T, value)` (TYP-24, ADR-0091): a cast the checker proves gives the value, any
//! other is checked when the program runs, and a cast no test decides is refused.

use crate::common::{app, messages};

#[test]
fn typ_24_a_cast_of_an_object_a_union_an_optional_or_a_base_is_accepted() {
    let source = concat!(
        "import typing\n",
        "from typing import cast\n",
        "class Shape:\n",
        "    pass\n",
        "class Square(Shape):\n",
        "    side = 2\n",
        "record: dict[str, object] = {'age': 36}\n",
        "maybe: str | None = 'x'\n",
        "mixed: int | str = 4\n",
        "shape: Shape = Square()\n",
        "age: int = cast(int, record['age'])\n",
        "text: str = cast(str, maybe)\n",
        "number: int = cast(int, mixed) + typing.cast(int, 1)\n",
        "square: Square = cast(Square, shape)\n",
        "either: int | str = cast(int | str, record['age'])\n",
        "print(age, text, number, square.side, either)\n",
    );
    assert_eq!(app(source), Vec::<String>::new());
}

#[test]
fn typ_24_a_cast_no_test_decides_or_that_always_fails_is_refused() {
    for (statement, wanted) in [
        ("items = cast(list[int], value)", "cannot be checked"),
        ("text = cast(str, 3)", "always raises"),
        ("text = cast(str)", "takes a type and a value"),
        (
            "text = cast(typ=str, val=value)",
            "takes a type and a value",
        ),
    ] {
        let found = messages(&format!(
            "from typing import cast\nvalue: object = 1\n{statement}\n"
        ));
        assert_eq!(found.len(), 1, "{statement}: {found:?}");
        assert_eq!(
            (found[0].0.as_str(), found[0].2.as_str()),
            ("E0404", "TYP-24")
        );
        assert!(found[0].1.contains(wanted), "{statement}: {found:?}");
    }
}

#[test]
fn typ_24_cast_is_a_function_not_a_type() {
    let found = messages("from typing import cast\nx: cast = 1\n");
    assert_eq!(found[0].0, "E0408", "{found:?}");
}

#[test]
fn typ_24_a_cast_to_an_optional_an_exception_or_a_union_of_classes_is_checked() {
    let source = concat!(
        "from typing import cast\n",
        "class Shape:\n",
        "    pass\n",
        "class Circle(Shape):\n",
        "    pass\n",
        "class Square(Shape):\n",
        "    pass\n",
        "value: object = None\n",
        "maybe: str | None = cast(str | None, value)\n",
        "error: Exception = ValueError('x')\n",
        "wrong: ValueError = cast(ValueError, error)\n",
        "shapes: list[Shape] = [Circle()]\n",
        "first: Circle | Square = cast(Circle | Square, shapes[0])\n",
        "mixed: int | str | None = 3\n",
        "number: int = cast(int, mixed)\n",
        "scale: float = cast(float, value) if value is not None else 1.5\n",
        "print(maybe, wrong, first, number, scale)\n",
    );
    assert_eq!(app(source), Vec::<String>::new());
}

#[test]
fn typ_24_a_cast_between_unrelated_classes_always_fails_and_is_refused() {
    let found = messages(concat!(
        "from typing import cast\n",
        "class Shape:\n",
        "    pass\n",
        "class Text:\n",
        "    pass\n",
        "shape: Shape = Shape()\n",
        "text = cast(Text, shape)\n",
        "error: ValueError = ValueError()\n",
        "other = cast(KeyError, error)\n",
    ));
    assert_eq!(found.len(), 2, "{found:?}");
    assert!(
        found
            .iter()
            .all(|(code, message, _)| code == "E0404" && message.contains("always raises"))
    );
}
