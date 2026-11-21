//! Calls that unpack with `*` and `**` (EXP-17, EXP-19, ADR-0073).

use crate::common::{app, dump, messages};

const PAIR: &str = "def pair(a: int, b: int = 0) -> int:\n    return a + b\n\n\n";

#[test]
fn exp_17_a_call_of_the_program_s_code_takes_unpacked_arguments() {
    let calls = [
        "print(pair(*[1, 2]), pair(*(1, 2)), pair(1, *[2]), pair(**{\"a\": 1}))\n",
        "values = [1]\nprint(pair(*values, **{\"b\": 2}), pair(*values, *values))\n",
        "point = (1, 2)\nprint(pair(*point), pair(*(), a=1))\n",
        "print(*[1, 2], *\"ab\", sep=\"-\")\n",
        "def sink(*args: int, **kwargs: int) -> int:\n    return len(args) + len(kwargs)\n\n\n\
         def forward(*args: int, **kwargs: int) -> int:\n    return sink(*args, **kwargs)\n\n\n\
         print(forward(1, x=2))\n",
        "class Box:\n    def __init__(self, a: int, *rest: int) -> None:\n        \
         self.a = a\n\n    def put(self, a: int) -> int:\n        return a\n\n\n\
         box = Box(*[1, 2])\nprint(box.put(*[3]), Box.put(box, **{\"a\": 4}))\n",
        "def first[T](*items: T) -> T:\n    return items[0]\n\n\n\
         print(first(*[1]) + 1, first(*\"ab\").upper())\n",
    ];
    for call in calls {
        let source = format!("{PAIR}{call}");
        assert_eq!(app(&source), Vec::<String>::new(), "{source}");
    }
    let found = dump(&format!(
        "{PAIR}values = [1]\nx = pair(*values, **{{\"b\": 2}})\n"
    ));
    for line in ["Spread List : list[int]", "Dict"] {
        assert!(found.contains(line), "{line} missing from\n{found}");
    }
}

#[test]
fn exp_17_an_unpacked_argument_that_cannot_fit_is_refused() {
    let refusals = [
        (
            "print(pair(*(1, \"a\")))\n",
            "E0404",
            "item 1 of the tuple `*` unpacks goes to `b`, which takes an int, and it is a str",
        ),
        (
            "print(pair(*(1, 2, 3)))\n",
            "E0404",
            "`pair` takes 2 positional arguments, and the tuple `*` unpacks gives more",
        ),
        (
            "words = [\"a\"]\nprint(pair(*words))\n",
            "E0404",
            "the parameters `*` may fill take int, and this unpacks items of type str",
        ),
        (
            "def mixed(a: int, b: str) -> None:\n    pass\n\n\nwords = [\"a\"]\nmixed(*words)\n",
            "E0404",
            "`a` takes an int, and `*` may give it values of type str",
        ),
        (
            "print(pair(**{\"a\": \"x\"}))\n",
            "E0404",
            "`a` takes an int, and `**` may give it values of type str",
        ),
        (
            "print(pair(**[1]))\n",
            "E0404",
            "`**` in a call takes a dict of `str` keys, and this is a list[int]",
        ),
        (
            "print(pair(**{\"a\": 1}, **{\"b\": 1.5}))\n",
            "E0409",
            "the dicts `**` unpacks in a call have one value type; this one holds float, and \
             the first int",
        ),
        (
            "numbers = [1]\nprint(pair(*numbers, \"a\"))\n",
            "E0404",
            "the parameters `*` may fill take int, and this is a str",
        ),
        (
            "def mixed(a: int, b: str) -> None:\n    pass\n\n\nnumbers = [1]\nmixed(*numbers, 2.5)\n",
            "E0409",
            "the positional arguments from the first `*` on have one type; this is a float, and \
             they are int",
        ),
    ];
    for (call, code, message) in refusals {
        let source = format!("{PAIR}{call}");
        let found = messages(&source);
        assert!(
            found
                .iter()
                .any(|(each, text, _)| each == code && text == message),
            "{source}: {found:?}"
        );
    }
}

#[test]
fn exp_22_unpacking_a_value_only_the_run_knows_in_any_other_callee_is_planned() {
    let planned = "`*` and `**` of a value only the run knows in a call of a builtin other \
                   than `print`, of a protocol method or of an exception class are planned for \
                   TyPy 0.11 (EXP-22)";
    for call in [
        "print(max(*[1, 2]))\n",
        "options = {\"sep\": \"-\"}\nprint(**options)\n",
        "for each in zip(*[[1]]):\n    pass\n",
        "class Failure(Exception):\n    pass\n\n\nerror = Failure(*[\"a\"])\n",
    ] {
        let source = format!("{PAIR}{call}");
        let found = messages(&source);
        assert!(
            found
                .iter()
                .any(|(code, text, rule)| code == "E0201" && text == planned && rule == "EXP-22"),
            "{source}: {found:?}"
        );
    }
}

#[test]
fn exp_22_a_callee_refused_itself_is_reported_and_not_the_unpacking() {
    for (call, code) in [
        ("print(undefined_name(*[1, 2]))\n", "E0205"),
        ("print(divmod(*[7, 2]))\n", "E0202"),
        ("print(divmod(*(7, 2)))\n", "E0202"),
    ] {
        let found = messages(call);
        assert!(
            found.iter().any(|(found_code, ..)| found_code == code)
                && !found.iter().any(|(.., rule)| rule == "EXP-22"),
            "{call}: {found:?}"
        );
    }
}

#[test]
fn exp_19_unpacking_written_out_in_full_binds_as_the_values_it_gives() {
    let source = format!(
        "{PAIR}from collections.abc import Callable\n\
         def apply(f: Callable[[int, int], int], pair: tuple[int, int]) -> int:\n    \
         return f(*pair)\n\
         point = (1, 2)\n\
         print(max(*point), min(*(3, 4)), apply(pair, point), **{{\"sep\": \"-\"}})\n\
         error = ValueError(*(\"bad\", 2))\nwords = [\"a\"]\nwords.append(*(\"b\",))\n"
    );
    assert_eq!(app(&source), Vec::<String>::new());
    let twice = "print(**{\"sep\": \"-\", \"sep\": \"+\"})\n";
    assert_eq!(app(twice), ["E0201"]);
}

#[test]
fn exp_17_each_kind_of_unpacked_argument_reaches_the_parameters_it_may() {
    let accepted = [
        "def rest(a: int, *more: int) -> int:\n    return a + len(more)\n\n\n\
         print(rest(*(1, 2, 3)))\n",
        "def keyed(*, k: int) -> int:\n    return k\n\n\noptions = {\"k\": 1}\nprint(keyed(**options))\n",
        "def two(**named: int) -> int:\n    return len(named)\n\n\n\
         print(two(**{\"a\": 1}, **{\"b\": 2}))\n",
        "def later(a: int, b: int | None, *, k: int = 0) -> int:\n    return a\n\n\n\
         numbers = [1]\nprint(later(*numbers))\n",
        "def first[T](*items: T) -> T:\n    return items[0]\n\n\n\
         def pick[T](a: T, b: T) -> T:\n    return b\n\n\n\
         def named[T](**options: T) -> list[T]:\n    return list(options.values())\n\n\n\
         print(pick(*(1, 2)), first(*[1], 2), named(**{\"x\": 1.5}), pick(1, b=2))\n",
    ];
    for program in accepted {
        assert_eq!(app(program), Vec::<String>::new(), "{program}");
    }
}

#[test]
fn exp_17_an_argument_that_cannot_be_unpacked_is_refused() {
    let refusals = [
        (
            "def one(a: int) -> int:\n    return a\n\n\nprint(one(*(1, 2)))\n",
            "E0404",
            "`one` takes 1 positional argument, and the tuple `*` unpacks gives more",
        ),
        (
            "def pair(a: int, b: int) -> int:\n    return a\n\n\nprint(pair(**{1: 2}))\n",
            "E0404",
            "`**` in a call takes a dict of `str` keys, and this is a dict[int, int]",
        ),
        (
            "def mixed(a: int, b: int | None) -> int:\n    return a\n\n\n\
             print(mixed(*[1], *[\"a\"]))\n",
            "E0409",
            "the positional arguments from the first `*` on have one type; this unpacks items \
             of type str, and they are int",
        ),
        (
            "def rest(a: int, *more: str) -> int:\n    return a\n\n\nprint(rest(*[1]))\n",
            "E0404",
            "`*more` takes a str, and `*` may give it values of type int",
        ),
        (
            "def kw(**named: str) -> int:\n    return 0\n\n\nprint(kw(**{\"a\": 1}))\n",
            "E0404",
            "`**named` takes a str, and `**` may give it values of type int",
        ),
    ];
    for (program, code, message) in refusals {
        let found = messages(program);
        assert!(
            found
                .iter()
                .any(|(each, text, _)| each == code && text == message),
            "{program}: {found:?}"
        );
    }
    // An unpacked value whose type is unknown is reported once, where it is named.
    for program in [
        "def pair(a: int, b: int) -> int:\n    return a\n\n\nprint(pair(*missing))\n",
        "def pair(a: int, b: int) -> int:\n    return a\n\n\nprint(pair(*(missing, 1)))\n",
        "def pair(a: int, b: int) -> int:\n    return a\n\n\nprint(pair(**missing))\n",
        "def first[T](*items: T) -> T:\n    return items[0]\n\n\nprint(first(*5))\n",
    ] {
        assert_ne!(messages(program).len(), 0, "{program}");
    }
}

#[test]
fn exp_17_a_generic_call_solves_what_it_can_from_unpacked_arguments() {
    let program = "def named[T](**options: T) -> list[T]:\n    return list(options.values())\n\n\n\
                   def first[T](*items: T) -> T:\n    return items[0]\n\n\n\
                   print(named(**[1]))\nprint(first(*[1], unknown=1))\n";
    assert_ne!(messages(program).len(), 0, "{program}");
}

#[test]
fn exp_22_a_function_value_takes_star_and_checks_the_count_when_it_runs() {
    for call in [
        "f = pair\nvalues = [1, 2]\nprint(f(*values), f(1, *values[1:]), f(*(i for i in range(2))))\n",
        "from collections.abc import Callable\n\n\n\
         def run(g: Callable[[int, int], int], xs: set[int]) -> int:\n    return g(*xs, *xs)\n",
    ] {
        let source = format!("{PAIR}{call}");
        assert_eq!(messages(&source), Vec::new(), "{source}");
    }
    let found = crate::common::dump(&format!("{PAIR}f = pair\nprint(f(*[1, 2]))\n"));
    assert!(found.contains("CallValueUnpacked"), "{found}");
}

#[test]
fn exp_22_a_function_value_refuses_what_one_list_cannot_bind() {
    let mixed = "from collections.abc import Callable\n\n\n\
                 def run(g: Callable[[int, str], int], xs: list[int]) -> int:\n    return g(*xs)\n";
    let refusals = [
        (
            mixed,
            "E0404",
            "`*` in a call of a value of type Callable[[int, str], int] may give a value to \
             parameters of more than one type, which one list cannot hold",
        ),
        (
            "f = pair\nprint(f(*[\"a\"]))\n",
            "E0404",
            "`*` gives values of type str, and the parameters take int",
        ),
        (
            "f = pair\nprint(f(1, 2, 3, *[4]))\n",
            "E0404",
            "a value of type Callable[[int, int], int] takes 2 arguments, and more are given",
        ),
        (
            "f = pair\nprint(f(*[1], 2.5))\n",
            "E0404",
            "a value after `*` must be an int, and this is a float",
        ),
        (
            "f = pair\noptions = {\"a\": 1}\nprint(f(**options))\n",
            "E0201",
            "`**` in a call of a function value, which binds by the names of the function it \
             holds, is planned for TyPy 1.0 (EXP-21)",
        ),
    ];
    for (body, code, message) in refusals {
        let source = format!("{PAIR}{body}");
        let found = messages(&source);
        assert!(
            found
                .iter()
                .any(|(each, text, _)| each == code && text == message),
            "{source}: {found:?}"
        );
    }
}
