//! Tests of checking `type` statements through the public API (SMT-10, SMT-12): where an
//! alias stands for its value, and the aliases TyPy refuses.

mod common;

use common::{app, codes};

#[test]
fn an_alias_stands_for_its_value_in_every_annotation() {
    let source = concat!(
        "type Score = int\n",
        "type Scores = list[Score]\n",
        "type Named = 'tuple[str, Score]'\n",
        "type Found = Node | None\n",
        "type Table = dict[str, Scores]\n",
        "class Node:\n",
        "    best: Score = 0\n",
        "    def __init__(self, value: Score) -> None:\n",
        "        self.value: Score = value\n",
        "    def pair(self) -> Named:\n",
        "        return ('n', self.value)\n",
        "def first(nodes: list[Node]) -> Found:\n",
        "    if nodes:\n",
        "        found: Found = nodes[0]\n",
        "    else:\n",
        "        found = None\n",
        "    for node in nodes:\n",
        "        last: Score = node.value\n",
        "    while False:\n",
        "        more: Scores = []\n",
        "    try:\n",
        "        tried: Score = 1\n",
        "    except ValueError:\n",
        "        failed: Score = 2\n",
        "    finally:\n",
        "        done: Score = 3\n",
        "    with open('x') as handle:\n",
        "        held: Score = 4\n",
        "    return found\n",
        "table: Table = {'a': [1]}\n",
        "print(first([Node(2)]) is None, table)\n",
    );
    assert_eq!(app(source), Vec::<String>::new());
}

#[test]
fn a_generic_alias_stands_for_its_value_with_its_type_arguments() {
    let source = concat!(
        "type Pair[T] = tuple[T, T]\n",
        "type Rows[T] = list[Pair[T]]\n",
        "type Maybe[T] = 'T | None'\n",
        "type Table[K, V] = dict[K, list[V]]\n",
        "type Named = Table[str, int]\n",
        "type T = str\n",
        "type Shadowed[T] = list[T]\n",
        "class Box[T]:\n",
        "    def __init__(self, value: T) -> None:\n",
        "        self.pair: Pair[T] = (value, value)\n",
        "def swap[T](pair: Pair[T]) -> Pair[T]:\n",
        "    return (pair[1], pair[0])\n",
        "rows: Rows[int] = [(1, 2)]\n",
        "table: Named = {'a': [1]}\n",
        "found: Maybe[int] = None\n",
        "numbers: Shadowed[int] = [1]\n",
        "print(swap(rows[0]), table, found, numbers, Box(1).pair)\n",
    );
    assert_eq!(app(source), Vec::<String>::new());
}

#[test]
fn an_alias_takes_only_values_of_its_type() {
    assert_eq!(
        app("type Score = int\ndef f(s: Score) -> None:\n    pass\nf('a')\n"),
        vec!["E0404".to_string()]
    );
}

#[test]
fn aliases_typy_does_not_compile_are_refused() {
    let cases: &[(&str, &str)] = &[
        ("type Score = int\ntype Score = str\n", "E0414"),
        ("type Tree = list[Tree]\n", "E0201"),
        ("type A = B\ntype B = dict[str, A]\nx: A = {}\n", "E0201"),
        ("type Pair[T] = tuple[T, T]\nx: Pair = (1, 2)\n", "E0408"),
        ("type Pair[K, V] = dict[K, V]\nx: Pair = {}\n", "E0408"),
        // A type parameter hides the alias of its name, and is not subscripted.
        (
            "type Box[T] = list[T]\ndef f[Box](x: Box[int]) -> None:\n    pass\n",
            "E0205",
        ),
        (
            "type Pair[T] = tuple[T, T]\nx: Pair[int, int] = (1, 2)\n",
            "E0408",
        ),
        ("type Some[T: int] = list[T]\n", "E0201"),
        (
            "type Pair[T] = tuple[T, int]\nx: Pair[str] = ('a', 'b')\n",
            "E0402",
        ),
        ("def f() -> None:\n    type Inner = int\n", "E0201"),
        ("class C:\n    type Inner = int\n", "E0201"),
        ("if True:\n    type Inner = int\n", "E0201"),
        ("type Score = int\nprint(Score)\n", "E0201"),
        ("type Score = int\nScore = 3\n", "E0202"),
        ("type Score = int\nclass Score:\n    pass\n", "E0202"),
        (
            "type Score = int\ndef Score() -> None:\n    pass\n",
            "E0202",
        ),
        ("type Score = frozenset[int]\n", "E0202"),
    ];
    for (source, code) in cases {
        assert_eq!(app(source), vec![code.to_string()], "{source}");
    }
}

#[test]
fn an_alias_of_another_module_is_planned() {
    let library = ("lib.py", "type Score = int\n");
    let cases: &[&str] = &[
        "import lib\nx: lib.Score = 1\n",
        "import lib\nprint(lib.Score)\n",
        "import lib\nlib.Score()\n",
        "from lib import Score\n",
    ];
    for source in cases {
        let found = codes(&[("app.py", source), library]);
        assert_eq!(found.first().map(String::as_str), Some("E0201"), "{source}");
    }
}
