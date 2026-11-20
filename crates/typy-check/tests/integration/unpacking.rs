//! Displays and targets that unpack (SMT-02, EXP-17).

use crate::common::{app, dump, messages};

#[test]
fn exp_17_a_display_and_a_target_unpack_any_iterable() {
    for source in [
        "a = [1, 2]\nb = (3, 4)\nprint([*a, *b, 5], (*a, 6), {*a, *b}, [*range(2)])\n",
        "d = {\"x\": 1}\nprint({**d, \"y\": 2})\n",
        "values: list[int | None] = [None, *[1, 2]]\nprint(values)\n",
        "first, *rest = [1, 2, 3]\nprint(first + 1, rest)\n",
        "*init, last = (1, 2, 3)\nhead, *middle, tail = \"abcd\"\nprint(init, last, head)\n",
        "left, right = [1, 2]\nfor key, *values in [[\"k\", \"v\"]]:\n    print(key, values)\n",
    ] {
        assert_eq!(app(source), Vec::<String>::new(), "{source}");
    }
    let found = dump("first, *rest = [1, 2, 3]\nnumbers = [*rest, first]\n");
    for line in [
        "Assign unpack (first: int, *rest: list[int])",
        "Spread List : list[int]",
    ] {
        assert!(found.contains(line), "{line} missing from\n{found}");
    }
}

#[test]
fn exp_17_unpacking_refuses_what_cpython_or_typing_refuses() {
    let cases = [
        (
            "a, *b, *c = [1, 2, 3]\n",
            "E0103",
            "multiple starred expressions in assignment",
        ),
        (
            "*a = [1, 2]\n",
            "E0103",
            "starred assignment target must be in a list or tuple",
        ),
        (
            "x = [1]\ny = *x\n",
            "E0103",
            "can't use starred expression here",
        ),
        (
            "words = [\"a\"]\nnumbers = [1, *words]\n",
            "E0409",
            "the items of a list have one type; this unpacks items of type str, and the list \
             holds int",
        ),
        (
            "pair = (1, \"a\", 2.5)\nfirst, *rest = pair\n",
            "E0308",
            "the starred target takes items of types str and float, which a list of one type \
             cannot hold",
        ),
        (
            "pair = (1,)\nfirst, *rest, last = pair\n",
            "E0308",
            "this unpacks 1 values into at least 2 targets",
        ),
        (
            "x = 5\na, *b = x\n",
            "E0308",
            "this unpacks an int into 2 targets",
        ),
        (
            "d = {\"a\": 1}\ne = {**d, \"b\": \"x\"}\n",
            "E0409",
            "the entries of a dict have one key type and one value type; these are str and \
             str, and the dict holds str and int",
        ),
        (
            "e = {**[1], \"b\": 1}\n",
            "E0404",
            "`**` in a dict display takes a dict, and this is a list[int]",
        ),
    ];
    for (source, code, message) in cases {
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
fn exp_17_a_display_refuses_an_item_of_another_type_after_an_unpacked_one() {
    let cases = [
        (
            "values = [*[1], \"a\"]\n",
            "the items of a list have one type; this is a str, and the list holds int",
        ),
        (
            "merged = {**{\"a\": 1}, **{\"b\": \"x\"}}\n",
            "the entries of a dict have one key type and one value type; these are str and \
             str, and the dict holds str and int",
        ),
        (
            "pairs: list[tuple[int, int] | None] = [*[(1, 2)]]\n",
            "the items of a list have one type; this unpacks items of type tuple[int, int], and \
             the list holds tuple[int, int] | None",
        ),
    ];
    for (source, message) in cases {
        let found = messages(source);
        assert!(
            found.iter().any(|(_, text, _)| text == message),
            "{source}: {found:?}"
        );
    }
    assert_ne!(messages("values = [*missing]\n").len(), 0);
}
