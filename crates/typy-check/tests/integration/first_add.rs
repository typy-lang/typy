//! A variable of a function assigned an empty collection takes its types from the first
//! statement that adds to it; a read before that statement, or no such statement, is E0410
//! (TYP-27, ADR-0092).

use crate::common::{app, messages};

#[test]
fn typ_27_each_statement_that_adds_gives_the_types() {
    let accepted = [
        // `append` in a loop.
        "def squares(n: int) -> list[int]:\n    found = []\n    for i in range(n):\n        \
         found.append(i * i)\n    return found\n\n\nprint(squares(4))\n",
        // `insert` and `list()`.
        "def names() -> list[str]:\n    found = list()\n    found.insert(0, \"ada\")\n    \
         return found\n\n\nprint(names())\n",
        // `extend` with a list.
        "def pair() -> list[float]:\n    found = []\n    found.extend([1.5, 2.5])\n    \
         return found\n\n\nprint(pair())\n",
        // `add` and `update` of a set.
        "def seen(words: list[str]) -> set[str]:\n    found = set()\n    found.add(\"a\")\n    \
         found.update(words)\n    return found\n\n\nprint(len(seen([\"b\"])))\n",
        "def letters() -> set[str]:\n    found = set()\n    found.update(\"abc\")\n    \
         return found\n\n\nprint(len(letters()))\n",
        // `items[key] = value`, `setdefault` and `update` of a dict.
        "def counts(words: list[str]) -> dict[str, int]:\n    found = {}\n    for word in words:\n        \
         found[word] = 1\n    return found\n\n\nprint(counts([\"a\"]))\n",
        "def groups() -> dict[str, list[int]]:\n    found = dict()\n    \
         found.setdefault(\"a\", [1])\n    return found\n\n\nprint(groups())\n",
        "def copied(source: dict[str, int]) -> dict[str, int]:\n    found = {}\n    \
         found.update(source)\n    return found\n\n\nprint(copied({\"a\": 1}))\n",
        // A type found gives another collection its type.
        "def rows() -> list[list[int]]:\n    row = []\n    row.append(1)\n    table = []\n    \
         table.append(row)\n    return table\n\n\nprint(rows())\n",
        // An instance method, a nested function and a method with an inferred result.
        "class Bag:\n    def __init__(self) -> None:\n        self.size = 0\n\n    \
         def items(self):\n        found = []\n        found.append(self.size)\n        \
         return found\n\n\nprint(Bag().items())\n",
        "def outer() -> int:\n    def inner() -> list[str]:\n        found = []\n        \
         found.append(\"x\")\n        return found\n\n    return len(inner())\n\n\nprint(outer())\n",
        // A module variable a function declares `global`, and a variable of the function
        // around it declared `nonlocal`, keep their own types.
        "items: list[int] = [1]\n\n\ndef reset() -> None:\n    global items\n    \
         items = []\n\n\nreset()\nprint(items)\n",
        "def outer() -> list[int]:\n    items: list[int] = [1]\n\n    def reset() -> None:\n        \
         nonlocal items\n        items = []\n\n    reset()\n    return items\n\n\nprint(outer())\n",
        // The variable assigned again keeps the type the first add gave.
        "def reset() -> list[int]:\n    found = []\n    found.append(1)\n    found = []\n    \
         return found\n\n\nprint(reset())\n",
    ];
    for source in accepted {
        assert_eq!(app(source), Vec::<String>::new(), "{source}");
    }
}

#[test]
fn typ_27_the_first_add_decides_the_type() {
    let found = messages(
        "def mixed() -> None:\n    found = []\n    found.append(1)\n    found.append(\"a\")\n\n\n\
         mixed()\n",
    );
    let codes: Vec<&str> = found.iter().map(|(code, ..)| code.as_str()).collect();
    assert_eq!(codes, ["E0404"], "{found:?}");
}

#[test]
fn typ_27_a_collection_whose_types_are_not_known_is_e0410() {
    let cases = [
        // Read before a statement adds to it.
        (
            "def total() -> int:\n    found = []\n    size = len(found)\n    found.append(1)\n    \
             return size\n",
            "an empty list needs an annotation that names its item type",
        ),
        // The value of `items[key] = value` reads the dict before the statement adds to it.
        (
            "def counts(words: list[str]) -> None:\n    found = {}\n    for word in words:\n        \
             found[word] = found.get(word, 0) + 1\n",
            "an empty dict needs an annotation that names its key and value types",
        ),
        // The value added reads the list first.
        (
            "def grow() -> None:\n    found = []\n    found.append(len(found))\n",
            "an empty list needs an annotation that names its item type",
        ),
        // A call that does not add to it, such as one with keyword arguments, reads it.
        (
            "def named() -> None:\n    found = {}\n    found.update(a=1)\n",
            "an empty dict needs an annotation that names its key and value types",
        ),
        (
            "def spread(rows: list[list[int]]) -> None:\n    found = []\n    \
             found.extend(*rows)\n",
            "an empty list needs an annotation that names its item type",
        ),
        // No statement adds to it.
        (
            "def nothing() -> None:\n    found = set()\n    print(\"no\")\n",
            "an empty set needs an annotation that names its item type",
        ),
        // A nested function adds to it, which the function's own statements do not.
        (
            "def outer() -> None:\n    found = []\n\n    def add() -> None:\n        \
             found.append(1)\n\n    add()\n",
            "an empty list needs an annotation that names its item type",
        ),
    ];
    for (source, message) in cases {
        let found = messages(source);
        assert!(
            found
                .iter()
                .any(|(code, text, rule)| code == "E0410" && text == message && rule == "TYP-05"),
            "{source}\n{found:?}"
        );
        let codes: Vec<&str> = found.iter().map(|(code, ..)| code.as_str()).collect();
        assert_eq!(codes, ["E0410"], "{source}\n{found:?}");
    }
}

#[test]
fn typ_27_an_added_value_of_another_kind_leaves_the_type_unknown() {
    let sources = [
        "def pairs() -> None:\n    found = {}\n    found.update([(\"a\", 1)])\n",
        "def numbers() -> None:\n    found = []\n    found.extend(1)\n",
    ];
    for source in sources {
        let found = messages(source);
        assert!(
            found.iter().any(|(code, ..)| code == "E0410"),
            "{source}\n{found:?}"
        );
    }
}

#[test]
fn typ_27_a_module_variable_still_needs_its_annotation() {
    assert_eq!(app("found = []\nfound.append(1)\n"), ["E0410"]);
}

#[test]
fn typ_27_an_added_value_without_a_type_leaves_the_type_unknown() {
    let found = messages(
        "def index(words: list[str]) -> None:\n    found = {}\n    for word in words:\n        \
         found.setdefault(word, []).append(1)\n\n\nindex([])\n",
    );
    let codes: Vec<&str> = found.iter().map(|(code, ..)| code.as_str()).collect();
    assert_eq!(codes, ["E0410"], "{found:?}");
}
