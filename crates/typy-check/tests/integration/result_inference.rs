//! A function of the module without a result annotation takes the result its `return`
//! statements give, and keeps the annotation where it must (TYP-26, ADR-0092).

use crate::common::{app, messages};

#[test]
fn typ_26_a_result_is_the_union_of_what_the_returns_give() {
    let accepted = [
        // One type, a type through another inferred function, and a module variable.
        "SCALE = 3\n\n\ndef double(n: int):\n    return n * 2\n\n\n\
         def scaled(n: int):\n    return double(n) * SCALE\n\n\n\
         total: int = scaled(2) + 1\nprint(total)\n",
        // A union with `None`, read as an optional.
        "def first(values: list[int]):\n    for value in values:\n        return value\n\n\n\
         found: int | None = first([])\nprint(found)\n",
        // No `return` with a value: `None`.
        "def show(n: int):\n    print(n)\n\n\nshow(1)\n",
        // A variable assigned a call before the function's `def` in the file.
        "def name_of(code: int):\n    return \"one\" if code == 1 else \"many\"\n\n\n\
         label = name_of(1)\nprint(label.upper())\n",
    ];
    for source in accepted {
        assert_eq!(app(source), Vec::<String>::new(), "{source}");
    }
}

#[test]
fn typ_26_an_instance_method_returns_what_its_returns_give() {
    let source = "class Account:\n    def __init__(self, owner: str) -> None:\n        \
                  self.owner = owner\n\n    def summary(self):\n        \
                  return self.label() + \"!\"\n\n    def label(self):\n        \
                  return self.owner.title()\n\n\ntext: str = Account(\"ada\").summary()\n\
                  print(text)\n";
    assert_eq!(app(source), Vec::<String>::new());
}

#[test]
fn typ_26_a_call_is_checked_against_the_inferred_result() {
    let found =
        messages("def name_of(code: int):\n    return str(code)\n\n\nprint(name_of(1) + 1)\n");
    assert!(
        found
            .iter()
            .any(|(code, text, _)| code == "E0401" && text == "`+` is not defined for str and int"),
        "{found:?}"
    );
}

#[test]
fn typ_26_a_function_that_must_keep_its_annotation_is_e0204_with_the_reason() {
    let cases = [
        (
            "def countdown(n: int):\n    return countdown(n - 1) if n else 0\n",
            "function `countdown` has no return annotation, and it calls itself, directly or \
             through other functions",
        ),
        (
            "def ping(n: int):\n    return pong(n)\n\n\ndef pong(n: int):\n    return ping(n)\n",
            "function `ping` has no return annotation, and it calls itself, directly or \
             through other functions",
        ),
        (
            "def numbers(n: int):\n    yield n\n",
            "function `numbers` has no return annotation, and a generator's type depends on \
             what it yields and is sent",
        ),
        (
            "class Counter:\n    @staticmethod\n    def value():\n        return 1\n",
            "function `value` has no return annotation",
        ),
        (
            "class Shape:\n    def area(self):\n        return 0\n\n\n\
             class Square(Shape):\n    def area(self) -> int:\n        return 1\n",
            "function `area` has no return annotation, and `Square.area` overrides it",
        ),
        (
            "class Shape:\n    def area(self) -> int:\n        return 0\n\n\n\
             class Square(Shape):\n    def area(self):\n        return 1\n",
            "function `area` has no return annotation, and it overrides `Shape.area`",
        ),
        (
            "class Box:\n    def __len__(self):\n        return 1\n",
            "function `__len__` has no return annotation",
        ),
        (
            "class Loop:\n    def forever(self):\n        return self.forever()\n",
            "function `forever` has no return annotation, and it calls itself, directly or \
             through other methods",
        ),
    ];
    for (source, message) in cases {
        let found = messages(source);
        assert!(
            found
                .iter()
                .any(|(code, text, rule)| code == "E0204" && text == message && rule == "TYP-01"),
            "{source}\n{found:?}"
        );
    }
}

#[test]
fn typ_26_an_error_in_the_body_is_reported_once() {
    let found = messages("def broken(n: int):\n    return n + missing\n\n\nprint(broken(1))\n");
    let codes: Vec<&str> = found.iter().map(|(code, ..)| code.as_str()).collect();
    assert_eq!(codes, ["E0205"], "{found:?}");
}
