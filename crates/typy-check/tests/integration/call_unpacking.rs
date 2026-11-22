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
    let planned = "`*` of a value only the run knows in a call of a function of a built-in \
                   module, and `**` of one in a call of a builtin other than `print`, are \
                   planned for TyPy 0.11 (EXP-22)";
    for call in [
        "import math\nprint(math.floor(*[1.5]))\n",
        "options = {\"key\": len}\nprint(max(*[\"a\"], **options))\n",
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
    // A display that names a key twice keeps its last value, which binds when the call runs.
    let twice = "print(**{\"sep\": \"-\", \"sep\": \"+\"})\n";
    assert_eq!(app(twice), Vec::<String>::new());
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

#[test]
fn exp_22_print_takes_star_star_of_a_dict_of_str_keys() {
    for call in [
        "options = {\"sep\": \"-\", \"end\": \"!\\n\"}\nprint(1, 2, **options)\n",
        "flags = {\"flush\": True}\nprint(1, **flags)\n",
        "maybe: dict[str, str | None] = {\"sep\": None}\nprint(1, sep=\"-\", **maybe)\n",
        "numbers = {\"sep\": 3}\nprint(1, **numbers)\n",
    ] {
        assert_eq!(messages(call), Vec::new(), "{call}");
    }
    let found = crate::common::dump("options = {\"sep\": \"-\"}\nprint(1, **options)\n");
    assert!(found.contains("Print"), "{found}");
    for (call, message) in [
        (
            "items = [1]\nprint(1, **items)\n",
            "`**` in a call of `print` takes a dict of `str` keys, and this is a list[int]",
        ),
        (
            "texts = {\"sep\": \"-\"}\nflags = {\"flush\": True}\nprint(1, **texts, **flags)\n",
            "the dicts `**` gives a call of `print` hold values of one type; this holds bool, \
             and the first holds str",
        ),
    ] {
        let found = messages(call);
        assert!(
            found
                .iter()
                .any(|(code, text, _)| code == "E0404" && text == message),
            "{call}: {found:?}"
        );
    }
}

#[test]
fn exp_22_a_method_through_a_protocol_binds_star_and_star_star_when_it_runs() {
    let source = "from typing import Protocol\n\n\n\
                  class Shape(Protocol):\n    def scaled(self, by: int, extra: int = 0) -> int: ...\n\n\n\
                  class Square:\n    def scaled(self, by: int, extra: int = 0) -> int:\n        \
                  return by + extra\n\n\n\
                  def run(shape: Shape, values: list[int], options: dict[str, int]) -> int:\n    \
                  return shape.scaled(*values, **options) + shape.scaled(1, *values)\n\n\n\
                  print(run(Square(), [1], {}))\n";
    assert_eq!(app(source), Vec::<String>::new());
    let found = dump(source);
    assert!(found.contains("Spread List : list[int]"), "{found}");
}

#[test]
fn exp_22_an_exception_class_takes_star_and_star_star() {
    let failure = "class Failure(Exception):\n    pass\n\n\n";
    for call in [
        "names = [\"a\"]\nerror = ValueError(*names)\n",
        "names = [\"a\"]\noptions = {\"code\": 1}\nerror = Failure(\"b\", *names, **options)\n",
        "options = {\"code\": 1}\nerror = KeyError(\"k\", 2, **options)\n",
        "error = Failure(*(1, \"a\"))\n",
        "codes = [1]\nerror = SystemExit(*codes)\n",
    ] {
        let source = format!("{failure}{call}");
        assert_eq!(app(&source), Vec::<String>::new(), "{source}");
    }
    let found = dump(&format!(
        "{failure}names = [\"a\"]\nerror = Failure(*names)\n"
    ));
    assert!(found.contains("NewExceptionUnpacked Failure"), "{found}");
    assert!(found.contains("Spread List : list[str]"), "{found}");
}

#[test]
fn exp_22_an_exception_class_refuses_what_its_arguments_cannot_hold() {
    for (call, code, message) in [
        (
            "names = [\"a\"]\nerror = OSError(*names)\n",
            "E0202",
            "`OSError` reads `errno`, `strerror` and the file names from its arguments by \
             their position, so `*` of a value only the run knows cannot give them",
        ),
        (
            "numbers = [1]\nerror = ValueError(\"x\", *numbers)\n",
            "E0409",
            "the arguments of an exception class called with `*` have one type; this unpacks \
             items of type int, and the first is a str",
        ),
        (
            "numbers = [1]\nerror = ValueError(*numbers, 2.5)\n",
            "E0409",
            "the arguments of an exception class called with `*` have one type; this is a \
             float, and the first is an int",
        ),
        (
            "numbers = [1]\nerror = ValueError(*numbers, code=1)\n",
            "E0404",
            "`ValueError` takes positional arguments only",
        ),
        (
            "numbers = [1]\nerror = ValueError(*numbers, **numbers)\n",
            "E0404",
            "`**` in a call takes a dict of `str` keys, and this is a list[int]",
        ),
    ] {
        let found = messages(call);
        assert!(
            found
                .iter()
                .any(|(each, text, _)| each == code && text == message),
            "{call}: {found:?}"
        );
    }
}

#[test]
fn exp_22_args_of_an_exception_made_with_star_is_a_tuple_of_any_length() {
    let source = "def check(values: list[int]) -> None:\n    \
                  try:\n        raise ValueError(*values)\n    \
                  except ValueError as error:\n        \
                  for item in error.args:\n            print(item)\n        \
                  first = error.args[0]\n        print(first)\n\n\n\
                  def stop(values: list[int]) -> None:\n    \
                  try:\n        raise SystemExit(*values)\n    \
                  except SystemExit as error:\n        code = error.code\n        \
                  print(code)\n\n\ncheck([1])\nstop([2])\n";
    assert_eq!(app(source), Vec::<String>::new());
    let found = dump(source);
    for line in [
        "ExceptionArguments : tuple[str] | tuple[int, ...]",
        "Comprehension : list[int | str]",
        "IfExp : int | tuple[int, ...] | None",
    ] {
        assert!(found.contains(line), "{line} in {found}");
    }
}

#[test]
fn exp_22_an_exception_class_with_init_binds_star_and_star_star_when_it_runs() {
    let classes = "class Coded(Exception):\n    def __init__(self, code: int, *more: int) -> None:\n        \
                   self.code = code\n\n\n\
                   class Missing(OSError):\n    def __init__(self, name: str) -> None:\n        \
                   super().__init__(2, \"missing\", name)\n\n\n";
    for call in [
        "codes = [1]\nerror = Coded(*codes)\n",
        "codes = [1]\nerror = Coded(1, *(2, 3), *codes)\n",
        "options = {\"code\": 1}\nerror = Coded(**options)\n",
        "names = [\"a\"]\nerror = Missing(*names)\n",
    ] {
        let source = format!("{classes}{call}");
        assert_eq!(app(&source), Vec::<String>::new(), "{source}");
    }
    let found = dump(&format!("{classes}codes = [1]\nerror = Coded(*codes)\n"));
    assert!(found.contains("NewException Coded"), "{found}");
    assert!(found.contains("Spread List : list[int]"), "{found}");
    let refused = "class Pair(Exception):\n    def __init__(self, name: str, *codes: int) -> None:\n        \
                   self.name = name\n\n\ncodes = [1]\npair = Pair(\"x\", *codes)\n";
    let message = "the arguments of an exception class called with `*` have one type, as `args` \
                   keeps them; these are str and int";
    let found = messages(refused);
    assert!(
        found
            .iter()
            .any(|(code, text, _)| code == "E0409" && text == message),
        "{found:?}"
    );
    let unbound = "class Pair(Exception):\n    def __init__(self, name: str) -> None:\n        \
                   self.name = name\n\n\ncodes = [1]\npair = Pair(*codes)\n";
    assert!(
        messages(unbound).iter().any(|(code, ..)| code == "E0404"),
        "{:?}",
        messages(unbound)
    );
}

#[test]
fn exp_22_a_value_of_an_unknown_type_in_an_unpacking_call_reports_only_its_own_error() {
    let pair = "class Pair(Exception):\n    def __init__(self, name: str, *codes: int) -> None:\n        \
                self.name = name\n\n\n";
    for call in [
        "numbers = [1]\nerror = ValueError(undefined, *numbers)\n".to_owned(),
        "numbers = [1]\nerror = ValueError(*numbers, undefined)\n".to_owned(),
        format!("{pair}codes = [1]\nerror = Pair(undefined, *codes)\n"),
        "from collections.abc import Callable\n\n\n\
         def apply(f: Callable[[int], int]) -> int:\n    return f(*undefined)\n"
            .to_owned(),
    ] {
        let found = messages(&call);
        assert!(
            !found.is_empty() && found.iter().all(|(code, ..)| code == "E0205"),
            "{call}: {found:?}"
        );
    }
}

#[test]
fn exp_22_unpacking_reaches_each_kind_of_callee_and_argument() {
    let accepted = [
        "class Box:\n    def cast(self, value: int) -> int:\n        return value\n\n\n\
         print(Box().cast(*[1]))\n",
        "key = \"sep\"\nprint(1, 2, **{key: \"-\"})\n",
        "from collections.abc import Callable\n\n\n\
         def apply(f: Callable[[int, int], int], values: list[int]) -> int:\n    \
         return f(1, 2, *values, 3)\n",
        "class Coded(Exception):\n    def __init__(self, code: int, *more: int) -> None:\n        \
         self.code = code\n\n\ncodes = [1]\nerror = Coded(*(1,), *codes)\n",
    ];
    for call in accepted {
        assert_eq!(messages(call), Vec::new(), "{call}");
    }
    let planned = [
        "import math\nprint(math.floor(*[1.5]))\n",
        "import math\nprint(math.floor(*(1.5, *[2.5])))\n",
    ];
    for call in planned {
        let found = messages(call);
        assert!(
            found
                .iter()
                .any(|(code, _, rule)| code == "E0201" && rule == "EXP-22"),
            "{call}: {found:?}"
        );
    }
    for (call, code, rule) in [
        (
            "options = {\"a\": 1}\nerror = OSError(1, \"x\", 3, **options)\n",
            "E0201",
            "EXC-18",
        ),
        (
            "from collections.abc import Callable\n\n\n\
             def apply(f: Callable[[int], int], values: list[int]) -> int:\n    \
             return f(1, 2, *values)\n",
            "E0404",
            "FUN-05",
        ),
    ] {
        let found = messages(call);
        assert!(
            found.iter().any(|(each, _, at)| each == code && at == rule),
            "{call}: {found:?}"
        );
    }
}

#[test]
fn exp_22_a_built_in_function_takes_star_in_each_form_it_has() {
    for call in [
        "values = [1, 4]\nprint(list(range(*values)))\n",
        "texts = [\"abc\"]\nprint(len(*texts), repr(*texts))\n",
        "words = [[\"b\", \"a\"]]\nprint(sorted(*words, reverse=True))\n",
        "def first(values: list[float]) -> float:\n    return abs(*values)\n",
        "numbers = [65]\nprint(chr(*numbers), bool(*numbers))\n",
    ] {
        assert_eq!(messages(call), Vec::new(), "{call}");
    }
    let found = dump("values = [1, 4]\nprint(list(range(*values)))\n");
    for line in ["Sequence", "IfExp", "Raise", "Named"] {
        assert!(found.contains(line), "{line} in {found}");
    }
}

#[test]
fn exp_22_a_built_in_function_refuses_what_a_form_of_it_cannot_take() {
    for (call, code, message) in [
        (
            "from collections.abc import Iterator\n\n\n\
             def take(sources: list[Iterator[int]]) -> int:\n    return next(*sources)\n",
            "E0404",
            "the default of `next` must be an int, and this is an Iterator[int]",
        ),
        (
            "values = [1]\nprint(type(*values))\n",
            "E0202",
            "making a class with `type(name, bases, namespace)` cannot be compiled by TyPy with \
             CPython's meaning",
        ),
        (
            "rows = [[1, 2]]\nprint(tuple(*rows))\n",
            "E0202",
            "`tuple` gives tuple[()] or tuple[int, ...] as the number of values `*` gives \
             decides, so its result has no one type",
        ),
        (
            "texts = [\"a\"]\nprint(len(\"a\", \"b\", *texts))\n",
            "E0404",
            "`len` takes at most 1 argument, and more come before `*`",
        ),
        (
            "numbers = [1]\nprint(list(range(*numbers, \"a\")))\n",
            "E0409",
            "the positional arguments from the first `*` on have one type; this is a str, \
             and the first is an int",
        ),
        (
            "numbers = [7]\nprint(str(*numbers))\n",
            "E0404",
            "`str` takes 0 to 1 arguments, and 2 are given",
        ),
    ] {
        let found = messages(call);
        assert!(
            found
                .iter()
                .any(|(each, text, _)| each == code && text == message),
            "{call}: {found:?}"
        );
    }
}

#[test]
fn exp_22_max_and_min_take_star_of_values_that_one_value_iterates_alike() {
    for call in [
        "values = [3, 9]\nprint(max(*values), min(2, *values))\n",
        "words = [\"ab\"]\nprint(max(*words), min(*words, key=str.upper))\n",
        "values = [1.5]\nprint(max(*values, default=0.0))\n",
    ] {
        assert_eq!(messages(call), Vec::new(), "{call}");
    }
    let found = messages("rows = [[1, 2]]\nprint(max(*rows))\n");
    let message = "`max` gives list[int] or int as the number of values `*` gives decides, so \
                   its result has no one type";
    assert!(
        found
            .iter()
            .any(|(code, text, _)| code == "E0202" && text == message),
        "{found:?}"
    );
}

#[test]
fn exp_22_zip_takes_star_of_iterables_of_one_type() {
    for call in [
        "rows = [[1, 2], [3, 4]]\nfor column in zip(*rows, strict=True):\n    print(sum(column))\n",
        "pairs = [(1, 2)]\ncolumns: list[tuple[int, ...]] = list(zip(*pairs))\n",
        "words = [\"ab\"]\ncolumns = zip(*words)\nprint(next(columns))\n",
    ] {
        assert_eq!(messages(call), Vec::new(), "{call}");
    }
    let found = dump("rows = [[1]]\nprint(list(zip(*rows)))\n");
    assert!(
        found.contains("ZipMany : Iterator[tuple[int, ...]]"),
        "{found}"
    );
    for (call, code, message) in [
        (
            "numbers = [1]\nprint(list(zip(*numbers)))\n",
            "E0411",
            "`zip` takes iterables, and `*` gives values of type int",
        ),
        (
            "rows = [[1]]\nprint(list(zip(*rows, strict=1)))\n",
            "E0404",
            "`strict` must be a bool, and this is an int",
        ),
    ] {
        let found = messages(call);
        assert!(
            found
                .iter()
                .any(|(each, text, _)| each == code && text == message),
            "{call}: {found:?}"
        );
    }
    assert!(
        messages("print(list(zip(*undefined)))\n")
            .iter()
            .all(|(code, ..)| code == "E0205")
    );
}

#[test]
fn exp_22_a_method_of_a_built_in_type_takes_star_in_each_form_it_has() {
    for call in [
        "words = [\"a\"]\nwords.append(*[\"b\"])\nprint(\"-\".join(*[words]))\n",
        "places = [1]\nprint(\"abcb\".find(\"b\", *places), [1, 2].pop(*places))\n",
        "seen = {1}\nothers = [[2]]\nprint(seen.union(*others), seen.difference(*others))\n\
         seen.update(*others)\nseen.intersection_update(*others)\n",
    ] {
        assert_eq!(messages(call), Vec::new(), "{call}");
    }
    for (call, code, message) in [
        (
            "names = {\"a\": \"b\"}\nkeys = [\"a\"]\nprint(names.get(*keys))\n",
            "E0202",
            "`dict.get` gives str | None or str as the number of values `*` gives decides, \
             so its result has no one type",
        ),
        (
            "words = [\"a\"]\nwords.append(\"b\", \"c\", *words)\n",
            "E0404",
            "`list.append` takes at most 1 argument, and more come before `*`",
        ),
    ] {
        let found = messages(call);
        assert!(
            found
                .iter()
                .any(|(each, text, _)| each == code && text == message),
            "{call}: {found:?}"
        );
    }
}
