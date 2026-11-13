//! Tests of checking `Final` through the public API (TYP-08): where it declares a name, and
//! every assignment it refuses.

mod common;

use common::{app, codes};

#[test]
fn final_declares_variables_class_variables_and_attributes() {
    let source = concat!(
        "from typing import Final\n",
        "import typing\n",
        "LIMIT: Final = 10\n",
        "NAME: Final[str] = 'box'\n",
        "RATE: typing.Final[float] = 0.5\n",
        "class Box:\n",
        "    SIZE: Final = 3\n",
        "    KIND: typing.Final[str] = 'plain'\n",
        "    def __init__(self, width: int) -> None:\n",
        "        self.width: Final = width\n",
        "        self.label: Final[str] = NAME\n",
        "        self.other = 1\n",
        "    def grow(self) -> None:\n",
        "        self.other = self.width + Box.SIZE\n",
        "def main() -> None:\n",
        "    total: Final[int] = LIMIT + 1\n",
        "    print(total, RATE, Box(2).label, Box.KIND)\n",
        "main()\n",
    );
    assert_eq!(app(source), Vec::<String>::new());
}

#[test]
fn a_final_name_is_not_assigned_again() {
    let cases: &[(&str, &str)] = &[
        ("LIMIT: Final = 1\nLIMIT = 2\n", "E0414"),
        ("LIMIT: Final = 1\nLIMIT += 2\n", "E0414"),
        (
            "LIMIT: Final = 1\nfor LIMIT in range(2):\n    pass\n",
            "E0414",
        ),
        (
            "LIMIT: Final = 1\ndef f() -> None:\n    global LIMIT\n    LIMIT = 2\n",
            "E0414",
        ),
        ("def f() -> None:\n    n: Final = 1\n    n = 2\n", "E0414"),
        ("class C:\n    X: Final = 1\nC.X = 2\n", "E0414"),
        ("class C:\n    X: Final[int] = 1\nC.X += 2\n", "E0414"),
        (
            "class C:\n    def __init__(self) -> None:\n        self.x: Final = 1\n    def f(self) -> None:\n        self.x = 2\n",
            "E0414",
        ),
        (
            "class C:\n    def f(self) -> None:\n        self.x = 2\n    def __init__(self) -> None:\n        self.x: Final[int] = 1\n",
            "E0414",
        ),
        (
            "class C:\n    def __init__(self) -> None:\n        self.x: Final = 1\n        self.x = 2\n",
            "E0414",
        ),
        (
            "class C:\n    def __init__(self) -> None:\n        self.x: Final = 1\nclass D(C):\n    def f(self) -> None:\n        self.x = 3\n",
            "E0414",
        ),
        (
            "class C:\n    def __init__(self) -> None:\n        self.x: Final = 1\n    def f(self) -> None:\n        self.x += 2\n",
            "E0414",
        ),
        (
            "class C:\n    def __init__(self) -> None:\n        self.x = 1\n    def f(self, other: 'C') -> None:\n        other.x: Final = 2\n",
            "E0408",
        ),
        (
            "class C:\n    def __init__(self, big: bool) -> None:\n        if big:\n            self.x: Final = 1\n        else:\n            self.x = 2\n",
            "E0408",
        ),
        ("LIMIT: Final[int]\n", "E0408"),
        ("def f(n: Final[int]) -> None:\n    pass\n", "E0408"),
        ("def f() -> Final:\n    return 1\n", "E0408"),
        ("class C:\n    x: Final[int]\n", "E0408"),
    ];
    for (body, code) in cases {
        let source = format!("from typing import Final\n{body}");
        assert_eq!(app(&source), vec![code.to_string()], "{body}");
    }
}

#[test]
fn a_final_variable_of_another_module_is_not_assigned() {
    let found = codes(&[
        (
            "app.py",
            "import limits\nlimits.LIMIT = 3\nlimits.LIMIT += 1\n",
        ),
        ("limits.py", "from typing import Final\nLIMIT: Final = 1\n"),
    ]);
    assert_eq!(found, vec!["E0414".to_string(), "E0414".to_string()]);
}
