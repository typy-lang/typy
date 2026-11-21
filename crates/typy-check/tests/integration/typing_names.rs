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
fn every_planned_name_of_an_import_is_reported() {
    assert_eq!(
        messages("from collections.abc import KeysView, ValuesView, ItemsView\n"),
        [
            planned("collections.abc.KeysView", "0.10", "LIB-20"),
            planned("collections.abc.ValuesView", "0.10", "LIB-20"),
            planned("collections.abc.ItemsView", "0.10", "LIB-20"),
        ]
    );
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
    let found = messages("from typing import KeysView, Frobnicate\n");
    assert_eq!(found.len(), 2, "{found:?}");
    assert_eq!(found[0], planned("typing.KeysView", "0.10", "LIB-20"));
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
