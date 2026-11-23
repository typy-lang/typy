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
            "from enum import Flag\nclass C(Flag):\n    X = 3\n",
            "CLS-47",
        ),
        (
            "from enum import IntFlag\nclass C(IntFlag):\n    X = 0\n",
            "CLS-47",
        ),
        (
            "from enum import Flag\nclass C(Flag):\n    X = 'x'\n",
            "CLS-47",
        ),
        (
            "from enum import IntEnum\nclass C(IntEnum):\n    X = 'x'\n",
            "CLS-47",
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

const LEVEL: &str = "from enum import IntEnum, StrEnum, auto\nclass Level(IntEnum):\n    LOW = auto()\n    HIGH = 10\n    ZERO = 0\nclass Mode(StrEnum):\n    READ = auto()\n    WRITE = 'w'\n";

#[test]
fn a_member_of_an_int_or_str_enum_is_its_value_where_one_is_wanted() {
    let cases = [
        "def double(n: int) -> int:\n    return n * 2\nprint(double(Level.HIGH), Level.LOW + 1, -Level.HIGH, Level.HIGH // 3)\n",
        "print(Level.LOW == 1, Level.HIGH > 5, sorted([Level.HIGH, Level.LOW]), [1, 2][Level.LOW])\n",
        "print(abs(Level.LOW), int(Level.LOW), float(Level.HIGH), hex(Level.HIGH), round(Level.LOW))\n",
        "print(list(range(Level.LOW)), divmod(Level.HIGH, 3), bool(Level.ZERO), f'{Level.HIGH:03}')\n",
        "def shout(text: str) -> str:\n    return text + '!'\nprint(shout(Mode.WRITE), Mode.READ.upper(), len(Mode.READ), Mode.READ + 'x')\n",
        "x: int = Level.HIGH\ny: str = Mode.READ\nprint(x, y, 'ea' in Mode.READ, f'{Mode.WRITE:>3}', Mode('w'))\n",
        "if Level.ZERO:\n    print('true')\nprint(Level.LOW in [Level.LOW], Level.LOW is Level.LOW, {Level.LOW: 1})\n",
    ];
    for source in cases {
        assert_eq!(
            app(&format!("{LEVEL}{source}")),
            Vec::<String>::new(),
            "{source}"
        );
    }
}

#[test]
fn a_member_of_another_type_where_a_value_is_wanted_is_still_refused() {
    let cases = [
        // A plain enum's member is no `int`.
        (
            "from enum import Enum\nclass C(Enum):\n    X = 1\nprint(C.X + 1)\n",
            "E0401",
        ),
        (
            "{LEVEL}def f(text: str) -> str:\n    return text\nprint(f(Level.LOW))\n",
            "E0404",
        ),
        // A list of members is no list of their values.
        ("{LEVEL}print(', '.join(list(Mode)))\n", "E0404"),
        (
            "from enum import StrEnum\nclass C(StrEnum):\n    X = 1\n",
            "E0404",
        ),
    ];
    for (source, code) in cases {
        let source = source.replace("{LEVEL}", LEVEL);
        assert_eq!(app(&source), [code], "{source}");
    }
    assert_eq!(
        messages("from enum import StrEnum\nclass C(StrEnum):\n    X = 1\n")[0].1,
        "1 is not a string, as CPython raises `TypeError` when the class statement runs"
    );
}

const PERM: &str = "from enum import Flag, IntFlag, auto\nclass Perm(Flag):\n    R = auto()\n    W = auto()\n    X = 8\n    READ = 1\nclass Bits(IntFlag):\n    A = 1\n    B = 2\n";

#[test]
fn flags_combine_and_hold_their_bits() {
    let cases = [
        "rw = Perm.R | Perm.W\nprint(repr(rw), str(rw), rw.name, rw.value, list(rw), len(rw), Perm.R in rw)\n",
        "print(repr(~Perm.R), repr(Perm.R & Perm.W), repr(Perm.R ^ Perm.X), bool(Perm(0)), Perm(3))\n",
        "print((Perm.R | Perm.W) is Perm(3), Perm.READ is Perm.R, list(Perm), len(Perm), Perm['READ'])\n",
        "for bit in Perm.R | Perm.X:\n    print(bit)\nprint([p.name for p in Perm])\n",
        "print(repr(Bits.A | 4), repr(4 | Bits.A), repr(~Bits.A), Bits.A + 1, int(Bits.B), Bits.B == 2)\n",
        "print(repr(Bits(9)), str(Bits(9)), len(Bits(9)), list(Bits(9)), f'{Bits.B:03}', Bits.A < Bits.B)\n",
        "try:\n    Perm(4)\nexcept ValueError as error:\n    print(error)\n",
    ];
    for source in cases {
        assert_eq!(
            app(&format!("{PERM}{source}")),
            Vec::<String>::new(),
            "{source}"
        );
    }
}

#[test]
fn a_flag_combines_only_with_its_own_kind() {
    let cases = [
        // A `Flag` member is no `int`.
        ("print(Perm.R | 1)\n", "E0401"),
        ("print(Perm.R + 1)\n", "E0401"),
        ("print(Perm.R.name.lower())\n", "E0412"),
    ];
    for (source, code) in cases {
        let found = app(&format!("{PERM}{source}"));
        assert!(found.contains(&code.to_owned()), "{source}: {found:?}");
    }
}

#[test]
fn rare_forms_of_members_have_their_answers() {
    // A flag with no members has no bits; any value but `0` raises when the program runs.
    assert_eq!(
        app(
            "from enum import Flag\nclass Empty(Flag):\n    pass\nprint(repr(Empty(0)), len(Empty))\n"
        ),
        Vec::<String>::new()
    );
    // A flag of the 128th bit is one, and `auto()` past 128 bits is planned with the values
    // it would need.
    assert_eq!(
        app(
            "from enum import Flag\nclass Top(Flag):\n    A = 170141183460469231731687303715884105728\nprint(Top.A)\n"
        ),
        Vec::<String>::new()
    );
    let huge = "170141183460469231731687303715884105727";
    for (kind, value) in [
        ("Enum", huge),
        ("Flag", "170141183460469231731687303715884105728"),
    ] {
        let source = format!(
            "from enum import {kind}, auto\nclass Big({kind}):\n    A = {value}\n    B = auto()\n"
        );
        let found = messages(&source);
        assert!(
            found
                .iter()
                .any(|(code, _, rule)| code == "E0201" && rule == "CLS-47"),
            "{source}\n{found:?}"
        );
    }
}

#[test]
fn members_of_every_written_form_are_read_or_planned() {
    // A third member of one value is an alias of the first, and a name with one leading
    // underscore is a member.
    assert_eq!(
        app(
            "from enum import Enum\nclass C(Enum):\n    A = 1\n    B = 1\n    D = 1\n    _hidden = 2\nprint(C.D is C.A, list(C))\n"
        ),
        Vec::<String>::new()
    );
    let planned = [
        "import enum\nclass C(enum.ReprEnum):\n    X = 1\n",
        "from enum import Enum\nclass C(Enum, boundary=1):\n    X = 1\n",
        "from enum import Enum\ndef keep[T](cls: T) -> T:\n    return cls\n@keep\nclass C(Enum):\n    X = 1\n",
        "from enum import Enum\nclass C(Enum):\n    X, Y = 1, 2\n",
        "from enum import Enum\nclass C(Enum):\n    X = Y = 1\n",
        "from enum import Enum, auto\nclass C(Enum):\n    X = auto(1)\n",
        "from enum import Enum\nclass C(Enum):\n    X = int()\n",
    ];
    for source in planned {
        let found = messages(source);
        assert!(
            found
                .iter()
                .any(|(code, _, rule)| code == "E0201" && rule == "CLS-47"),
            "{source}\n{found:?}"
        );
    }
    // A keyword other than `value` is refused as any other call of the class.
    assert_eq!(app(&format!("{COLOR}print(Color(val=1))\n")), ["E0404"]);
}
