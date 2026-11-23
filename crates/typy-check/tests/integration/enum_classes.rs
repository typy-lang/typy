//! Tests of enum classes through the public API (CLS-44, CLS-47): the members and methods an
//! enum class gets, what the class does as a value, and the forms refused.

use crate::common::{app, codes, messages};

const COLOR: &str = "from enum import Enum, auto\nclass Color(Enum):\n    RED = 1\n    GREEN = auto()\n    CRIMSON = 1\n    def label(self) -> str:\n        return self.name.lower()\n";

#[test]
fn an_enum_class_has_its_members_and_what_cpython_gives_it() {
    let cases = [
        "c: Color = Color.RED\nprint(repr(c), str(c), c.name, c.value + 1, c.label())\n",
        "print(Color.CRIMSON is Color.RED, Color.RED == Color.GREEN, Color.RED != Color.GREEN)\n",
        "print(Color(2), Color(value=1), Color['CRIMSON'], len(Color), Color.RED in Color)\n",
        "for member in Color:\n    print(member)\nprint(list(Color), sorted(m.value for m in Color))\n",
        "print([c.name for c in Color], {Color.RED: 'r'}[Color.CRIMSON], isinstance(Color.RED, Color))\n",
        "match Color.RED:\n    case Color.GREEN:\n        print('green')\n    case Color.RED:\n        print('red')\n",
        "def pick(c: Color) -> int:\n    return c.value\nprint(pick(Color.GREEN))\n",
    ];
    for source in cases {
        assert_eq!(
            app(&format!("{COLOR}{source}")),
            Vec::<String>::new(),
            "{source}"
        );
    }
}

#[test]
fn members_take_constants_of_several_types_and_auto() {
    let cases = [
        "from enum import Enum, auto\nclass Mode(Enum):\n    READ = auto()\n    WRITE = 'w'\n    RAW = b'r'\n    ON = True\nprint(list(Mode), Mode('w'), Mode(True))\n",
        "import enum\nclass Level(enum.Enum):\n    LOW = -1\n    HIGH = enum.auto()\nprint(Level.HIGH.value)\n",
        // A class with no members is one too.
        "from enum import Enum\nclass Empty(Enum):\n    pass\nprint(list(Empty), len(Empty))\n",
    ];
    for source in cases {
        assert_eq!(app(source), Vec::<String>::new(), "{source}");
    }
}

#[test]
fn an_enum_class_of_another_module_is_one_there_too() {
    assert_eq!(
        codes(&[
            (
                "app.py",
                "import colors\nfrom colors import Color\nprint(colors.Color(1), Color['RED'], list(colors.Color), len(Color))\n"
            ),
            (
                "colors.py",
                "import enum\nclass Color(enum.Enum):\n    RED = 1\n    GREEN = enum.auto()\n"
            ),
        ]),
        Vec::<String>::new()
    );
}

#[test]
fn what_cpython_refuses_when_the_class_statement_runs_is_reported() {
    let cases = [
        (
            "from enum import Enum\nclass C(Enum):\n    X = 1\n    X = 2\n",
            "E0404",
            "'X' already defined as 1, as CPython raises `TypeError` when the class statement \
             runs",
        ),
        (
            "from enum import Enum\nclass C(Enum):\n    X = 1\nclass D(C):\n    Y = 2\n",
            "E0404",
            "<enum 'D'> cannot extend <enum 'C'>, as CPython raises `TypeError` when the class \
             statement runs",
        ),
        (
            "from enum import Enum, auto\nclass C(Enum):\n    X = 'x'\n    Y = auto()\n",
            "E0202",
            "`auto()` after the value 'x' gives `1` with a warning before Python 3.13 and raises \
             `TypeError` from 3.13",
        ),
        (
            "from enum import Enum\nclass C(Enum):\n    X = 1\nprint(C())\n",
            "E0404",
            "`C` is called with one value, which finds the member that has it",
        ),
    ];
    for (source, code, message) in cases {
        let found = messages(source);
        assert_eq!(found.len(), 1, "{source}: {found:?}");
        assert_eq!(
            (found[0].0.as_str(), found[0].1.as_str()),
            (code, message),
            "{source}"
        );
    }
}

#[test]
fn forms_of_a_later_release_are_planned() {
    let cases = [
        ("class C(Enum):\n    X = 1.5\n", "CLS-47"),
        ("class C(Enum):\n    X: int = 1\n", "CLS-47"),
        ("class C(Enum):\n    _x_ = 1\n", "CLS-47"),
        ("class C(Enum):\n    name = 1\n", "CLS-47"),
        (
            "class C(Enum):\n    X = 1\n    def __init__(self, value: int) -> None:\n        pass\n",
            "CLS-47",
        ),
        (
            "class C(Enum):\n    X = 1\n    def value(self) -> int:\n        return 1\n",
            "CLS-47",
        ),
        ("class C(str, Enum):\n    X = 'x'\n", "CLS-47"),
        ("class C[T](Enum):\n    X = 1\n", "CLS-47"),
        (
            "class B(Enum):\n    pass\nclass C(B):\n    X = 1\n",
            "CLS-47",
        ),
        ("from enum import unique\n", "CLS-47"),
        (
            "from enum import IntEnum\nclass C(IntEnum):\n    X = 1\n",
            "CLS-44",
        ),
    ];
    for (body, rule) in cases {
        let source = format!("from enum import Enum\n{body}");
        let found = messages(&source);
        assert!(
            found
                .iter()
                .any(|(code, _, each)| code == "E0201" && each == rule),
            "{source}\n{found:?}"
        );
    }
}

#[test]
fn names_of_enum_outside_bases_and_values_are_refused() {
    let cases = [
        ("from enum import Enum\nx: Enum\n", "E0408"),
        ("from enum import auto\nprint(auto)\n", "E0202"),
        (
            "from enum import Enum\nclass C(Enum):\n    X = 1\nprint(C.X < C.X)\n",
            "E0407",
        ),
        (
            "from enum import Enum\nclass C(Enum):\n    X = 1\nC.X.name = 'y'\n",
            "E0414",
        ),
    ];
    for (source, code) in cases {
        assert_eq!(app(source), [code], "{source}");
    }
}
