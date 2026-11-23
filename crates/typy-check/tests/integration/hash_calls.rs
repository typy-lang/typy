//! Calls of the builtin `hash` (LIB-34, ADR-0122).

use crate::common::messages;

#[test]
fn hash_takes_one_argument_by_position() {
    let cases = [
        (
            "print(hash())\n",
            "hash() takes exactly one argument (0 given)",
        ),
        (
            "print(hash(1, 2))\n",
            "hash() takes exactly one argument (2 given)",
        ),
        ("print(hash(obj=1))\n", "hash() takes no keyword arguments"),
    ];
    for (source, message) in cases {
        let found = messages(source);
        assert_eq!(found.len(), 1, "{source}: {found:?}");
        assert_eq!(found[0].0, "E0404", "{source}");
        assert_eq!(
            found[0].1,
            format!("{message}: CPython raises `TypeError`"),
            "{source}"
        );
        assert_eq!(found[0].2, "LIB-34", "{source}");
    }
}

#[test]
fn hash_of_a_value_without_a_hash_is_refused() {
    for (source, ty) in [
        ("print(hash([1]))\n", "a list[int]"),
        ("print(hash((1, {2})))\n", "a tuple[int, set[int]]"),
        ("print(hash({'a': 1}.keys()))\n", "a dict_keys[str, int]"),
    ] {
        let found = messages(source);
        assert_eq!(found.len(), 1, "{source}: {found:?}");
        assert_eq!(found[0].0, "E0413", "{source}");
        assert_eq!(
            found[0].1,
            format!("`hash` of {ty}, which has no hash: CPython raises `TypeError`"),
            "{source}"
        );
    }
}

#[test]
fn hash_of_a_value_with_a_hash_is_accepted() {
    let source = "\
class Point:
    def __init__(self, x: int) -> None:
        self.x = x

    def __eq__(self, other: \"Point\") -> bool:
        return self.x == other.x

    def __hash__(self) -> int:
        return hash(self.x)


print(hash(Point(1)), hash((1, 'a', None, 2.5, b'x')), hash(True))
";
    assert_eq!(messages(source), Vec::new());
}
