//! Typing values: names of `typing` and `collections.abc`, generic aliases and unions read as
//! values, written only where every oracle from the floor writes them alike (TYP-18,
//! ADR-0087).

use typy_source::PythonFloor;

use crate::common::{dump, messages, messages_at};

/// Returns the codes of the diagnostics of `source` at the floor `floor`.
fn codes_at(source: &str, floor: PythonFloor) -> Vec<String> {
    messages_at(source, floor)
        .into_iter()
        .map(|(code, _, _)| code)
        .collect()
}

#[test]
fn typ_18_each_kind_of_typing_value_is_a_value() {
    for source in [
        "import typing\nimport collections.abc as abc\nfrom typing import Callable, \
         ClassVar, Final, AbstractSet, Sequence, Protocol, Iterable, Generator, Type\n\
         from types import TracebackType, ModuleType\n\
         print(typing.Iterator, abc.Iterator, Callable, abc.Callable, ClassVar, Final)\n\
         print(AbstractSet, abc.Set, Sequence, Protocol, Iterable, Generator, Type)\n\
         print(TracebackType, ModuleType, typing.List, typing.Dict, typing.Set, typing.Tuple)\n",
        "import collections.abc as abc\nfrom typing import Callable, Generator, List, Tuple\n\
         print(list[int], dict[str, None], set[int], tuple[int, ...], tuple[()], type[int])\n\
         print(List[None], Tuple[int, ...], Callable[[int], str], Callable[..., None])\n\
         print(abc.Callable[[], int], Generator[int, None, None], abc.Iterator[list[int]])\n",
        "class Box:\n    pass\n\n\nprint(Box | None, list[Box], None | int, int | str | int)\n",
        "def show[T](value: T) -> None:\n    print(list[T], value)\n\n\nshow(1)\nshow(\"a\")\n",
        "print(list[int] == list[int], (int | str) == (str | int), {list[int]: 1}[list[int]])\n",
        "kinds = [list[int], int | None]\nkinds.append(dict[str, int])\nprint(kinds)\n",
        "print(isinstance(3, int | str), isinstance(None, int | None))\n",
        "from typing import Union\nprint(Union[int], int | int)\n",
        "import typing\nprint(typing.List is typing.List, typing.Optional is not typing.Union)\n",
        "held: object = list[int]\nprint(held, \"%s\" % (set[int],), f\"{dict[str, int]!r}\")\n",
        // A literal type written with a value twice, or inside another, shows each value once.
        "from typing import Literal\nprint(Literal[1, 1], Literal[Literal[1], 2], Literal[-1])\n\
         print(list[tuple[()]], tuple[tuple[()], int])\n",
    ] {
        assert_eq!(messages(source), Vec::new(), "{source}");
    }
}

#[test]
fn typ_18_an_alias_s_value_type_parameters_and_arguments_are_read() {
    for source in [
        "type Score = int\ntype Pair[T] = tuple[T, T]\ntype Plain = int | None\n\
         type Again = Score\ntype Nothing = None\n\
         print(Score.__value__, Pair.__value__, Plain.__value__, Nothing.__value__)\n\
         print(Pair.__type_params__, Score.__type_params__, Again.__value__ is Score)\n\
         print(Pair[int] == Pair[str], {Pair[int]: 1}[Pair[int]])\n",
        "def make() -> list[str]:\n    type Local[T] = dict[str, T]\n    \
         return [Local.__name__, str(Local.__value__), str(Local.__type_params__)]\n\n\n\
         print(make())\n",
        "class Holder:\n    type Inner = str\n\n\n\
         print(Holder.Inner.__value__, Holder.Inner.__type_params__)\n",
    ] {
        assert_eq!(messages(source), Vec::new(), "{source}");
    }
    let found = dump("def make() -> str:\n    type Local = int\n    return Local.__name__\n");
    assert!(found.contains("NewTypeAlias"), "{found}");
    // An alias a function or a class defines is written alike from 3.15.
    let written = "def make() -> None:\n    type Local = int\n    print(Local)\n\n\n\
                   class Holder:\n    type Inner = str\n\n\nprint(Holder.Inner)\nmake()\n";
    assert_eq!(codes_at(written, PythonFloor::V3_12), ["E0202", "E0202"]);
    assert_eq!(codes_at(written, PythonFloor::V3_15), Vec::<String>::new());
}

#[test]
fn typ_18_a_value_is_a_constant_of_its_text() {
    let found = dump("print(list[int | None])\n");
    assert!(found.contains("TypingValue 0"), "{found}");
}

#[test]
fn typ_18_a_value_written_otherwise_below_its_floor_is_refused_only_where_written() {
    let cases = [
        (
            "from typing import Optional\nprint(Optional[int])\n",
            PythonFloor::V3_14,
        ),
        (
            "from typing import Union\nprint(str(Union[int, str]))\n",
            PythonFloor::V3_14,
        ),
        (
            "import typing\nprint(f\"{typing.Union}\")\n",
            PythonFloor::V3_14,
        ),
        (
            "from typing import Optional\nprint(\"%s\" % Optional[str])\n",
            PythonFloor::V3_14,
        ),
        (
            "type Score = int\nprint(repr(list[Score]))\n",
            PythonFloor::V3_15,
        ),
        (
            "type Score = int\nkinds = [Score | None]\nprint(kinds)\n",
            PythonFloor::V3_15,
        ),
        (
            "from typing import Optional\nheld: object = Optional[int]\n",
            PythonFloor::V3_14,
        ),
        (
            "from typing import Optional\ndef echo[T](value: T) -> None:\n    print(value)\n\n\n\
             echo(Optional[int])\n",
            PythonFloor::V3_14,
        ),
    ];
    for (source, alike) in cases {
        assert!(
            !codes_at(source, PythonFloor::V3_12).is_empty(),
            "refused at 3.12: {source}"
        );
        assert_eq!(codes_at(source, alike), Vec::<String>::new(), "{source}");
    }
    // Storing and comparing is accepted from any floor.
    let stored = "from typing import Optional\nkinds = [Optional[int]]\n\
                  print(Optional[int] == int | None, Optional[int] in kinds)\n";
    assert_eq!(codes_at(stored, PythonFloor::V3_12), Vec::<String>::new());
}

#[test]
fn typ_18_what_no_typing_value_gives_is_refused_with_the_reason() {
    let refusals = [
        (
            "print(list[int] is list[int])\n",
            "E0202",
            "`is` between typing values other than names of `typing` and `collections.abc` \
             is refused, since CPython keeps one object of some and makes others anew",
        ),
        (
            "from typing import cast\nf = cast\n",
            "E0202",
            "this name of `typing`, `dataclasses` or `abc` is read only where it is called, \
             decorates or a class names it, not as a value",
        ),
        (
            "import typing\nprint(typing)\n",
            "E0202",
            "the module `typing` is read through its names, such as `typing.Optional`, and is \
             no value itself",
        ),
        (
            "from typing import Optional\nx = Optional[int, str]\n",
            "E0408",
            "`Optional` takes one type, as CPython requires",
        ),
        (
            "tuple = [1]\nprint(tuple[1, ...])\n",
            "E0202",
            "`...` as a value cannot be compiled by TyPy with CPython's meaning",
        ),
        (
            "from typing import Optional\nprint(Optional[int])\n",
            "E0202",
            "this typing value is written alike only from Python 3.14, and an older CPython \
             from the program's floor, 3.12, writes it otherwise",
        ),
        (
            "type Score = int\nheld = Score\nprint(held.__type_params__)\n",
            "E0202",
            "`__type_params__` is read only through the name of an alias, whose `type` \
             statement TyPy reads",
        ),
        (
            "type Score = int\nprint(Score[int])\n",
            "E0401",
            "Only generic type aliases are subscriptable: CPython raises `TypeError`",
        ),
        (
            "def f() -> None:\n    type Local[T] = list[T]\n    print(Local[int])\n",
            "E0201",
            "an alias a function or a class defines, or one held in a variable, given type \
             arguments as a value is planned for TyPy 1.0 (TYP-22)",
        ),
        (
            "def f() -> None:\n    type Local = int\n    Local()\n",
            "E0401",
            "'typing.TypeAliasType' object is not callable: CPython raises `TypeError`",
        ),
        (
            "class Box[T]:\n    pass\n\n\ntype Boxed = Box[int]\nprint(Boxed.__value__)\n",
            "E0202",
            "this alias's value is no typing value TyPy reads, so `__value__` is not read",
        ),
    ];
    for (source, code, message) in refusals {
        let found = messages(source);
        assert!(
            found
                .iter()
                .any(|(each, text, _)| each == code && text == message),
            "{source}: {found:?}"
        );
    }
}

#[test]
fn typ_18_an_imported_alias_and_a_shadowed_name_are_read_as_cpython_reads_them() {
    let shapes = "class Point:\n    pass\n\n\ntype Pair[T] = tuple[T, T]\ntype Score = int\n";
    let accepted = "from shapes import Pair, Score\nkinds = [list[Score], Pair[int], Score | None]\n\
                    print(kinds == [list[Score], Pair[int], None | Score])\n";
    assert_eq!(
        crate::common::codes(&[("app.py", accepted), ("shapes.py", shapes)]),
        Vec::<String>::new()
    );
    // A variable of the function hides a class or an alias of the same name, so the
    // subscript is no typing value.
    for source in [
        "class Box:\n    pass\n\n\ndef f() -> None:\n    Box = [1]\n    print(Box[0])\n",
        "type Score = int\n\n\ndef f() -> None:\n    Score = [1]\n    print(Score[0])\n",
    ] {
        assert_eq!(messages(source), Vec::new(), "{source}");
    }
}

#[test]
fn typ_18_forms_that_give_no_typing_value_are_checked_as_they_are() {
    for (source, code) in [
        ("print(None | None)\n", "E0401"),
        ("import typing\nprint(typing.List is list[int])\n", "E0202"),
        (
            "x = 1\ntype A = list[int]\nprint(A.__value__)\nprint(list[x])\n",
            "E0401",
        ),
    ] {
        let found = messages(source);
        assert!(
            found.iter().any(|(each, _, _)| each == code),
            "{source}: {found:?}"
        );
    }
    // A tuple given as one argument is the arguments, and an alias's own type parameter is
    // its value, as CPython writes them.
    let written = "type Id[T] = T\nprint(list[(int, str)], Id.__value__, Id.__value__ == \
                   Id.__type_params__[0])\n";
    assert_eq!(messages(written), Vec::new());
}
