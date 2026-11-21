//! The names of `typing` and `collections.abc` a later release gives: each one an import
//! names is reported with the requirement and the release that plan it (LIB-20, LIB-24,
//! ADR-0086).

use crate::common::messages;

fn planned(name: &str, release: &str, rule: &str) -> (String, String, String) {
    (
        "E0201".to_owned(),
        format!("`{name}` is planned for TyPy {release} ({rule})"),
        rule.to_owned(),
    )
}

#[test]
fn lib_20_the_views_of_a_dict_are_types() {
    let source = concat!(
        "from collections.abc import KeysView, ValuesView, ItemsView\n",
        "def parts(keys: KeysView[str], values: ValuesView[int], items: ItemsView[str, int]) -> int:\n",
        "    return len(keys) + len(values) + len(items)\n",
        "table = {'a': 1}\n",
        "print(parts(table.keys(), table.values(), table.items()))\n",
    );
    assert_eq!(crate::common::app(source), Vec::<String>::new());
}

#[test]
fn the_names_without_an_item_type_are_planned_for_a_later_release() {
    assert_eq!(
        messages("from collections.abc import Sized, Hashable, MappingView\n"),
        [
            planned("collections.abc.Sized", "0.14", "LIB-24"),
            planned("collections.abc.Hashable", "0.14", "LIB-24"),
            planned("collections.abc.MappingView", "0.14", "LIB-24"),
        ]
    );
}

#[test]
fn a_name_neither_module_gives_is_named_with_those_it_gives() {
    let found = messages("from typing import Sized, Frobnicate\n");
    assert_eq!(found.len(), 2, "{found:?}");
    assert_eq!(found[0], planned("typing.Sized", "0.14", "LIB-24"));
    assert_eq!(found[1].0, "E0425");
}

#[test]
fn lib_20_an_abstract_collection_holds_its_built_in_collections() {
    let source = concat!(
        "from collections.abc import Collection, Mapping, MutableSequence, Sequence\n",
        "from typing import AbstractSet\n",
        "def total(values: Sequence[int]) -> int:\n",
        "    return sum(values) + len(values) + values[0]\n",
        "def first(table: Mapping[str, int]) -> int:\n",
        "    return table['a']\n",
        "def grow(items: MutableSequence[int]) -> None:\n",
        "    items.append(1)\n",
        "def size(items: Collection[str]) -> int:\n",
        "    return len(items) + (1 if 'a' in items else 0)\n",
        "def count(items: AbstractSet[int]) -> int:\n",
        "    return len(items)\n",
        "print(total([1]), total((1, 2)), total(range(3)), total(b'a'), first({'a': 1}))\n",
        "print(size('abc'), size({'a'}), count({1}))\n",
    );
    assert_eq!(crate::common::app(source), Vec::<String>::new());
    let wrong = [
        "from collections.abc import Sequence\ndef f(values: Sequence) -> None:\n    pass\n",
        "from collections.abc import Mapping\ndef f(values: Mapping[str]) -> None:\n    pass\n",
    ];
    for source in wrong {
        assert_eq!(crate::common::app(source), ["E0408"], "{source}");
    }
}

#[test]
fn lib_20_a_collection_that_reads_its_items_takes_a_derived_class() {
    let shapes = "from collections.abc import MutableSequence, Reversible, Sequence\n\
                  class Shape:\n    pass\nclass Square(Shape):\n    pass\n";
    let accepted = format!(
        "{shapes}def count(items: Sequence[Shape]) -> int:\n    return len(items)\n\
         def back(items: Reversible[int]) -> list[int]:\n    return [item for item in reversed(items)]\n\
         print(count([Square()]), count((Square(), Shape())), back({{1: 'a'}}), back(range(3)))\n\
         print(isinstance([1], Sequence), isinstance({{1}}, Sequence))\n"
    );
    assert_eq!(crate::common::app(&accepted), Vec::<String>::new());
    let refused = [
        (
            "def grow(items: MutableSequence[Shape]) -> None:\n    items.append(Shape())\n\
             squares = [Square()]\ngrow(squares)\n",
            "argument `items` must be a MutableSequence[Shape], and this is a list[Square]",
        ),
        (
            "def total(items: Sequence[int]) -> int:\n    return len(items)\n\
             flags = [True]\ntotal(flags)\n",
            "argument `items` must be a Sequence[int], and this is a list[bool]",
        ),
    ];
    for (source, message) in refused {
        let found = messages(&format!("{shapes}{source}"));
        assert_eq!(found.len(), 1, "{found:?}");
        assert_eq!(found[0].1, message);
    }
}
