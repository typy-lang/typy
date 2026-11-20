//! The names of `typing` and `collections.abc` a later release gives: each one an import
//! names is reported with the requirement and the release that plan it (LIB-20, LIB-24,
//! CLS-27, ADR-0085, ADR-0086).

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
        messages("from typing import MutableSequence, Collection, AbstractSet\n"),
        [
            planned("typing.MutableSequence", "0.10", "LIB-20"),
            planned("typing.Collection", "0.10", "LIB-20"),
            planned("typing.AbstractSet", "0.10", "LIB-20"),
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
    let found = messages("from typing import Sequence, Frobnicate\n");
    assert_eq!(found.len(), 2, "{found:?}");
    assert_eq!(found[0], planned("typing.Sequence", "0.10", "LIB-20"));
    assert_eq!(found[1].0, "E0425");
}

#[test]
fn a_class_derived_from_named_tuple_is_planned() {
    let found = messages("import typing\nclass Point(typing.NamedTuple):\n    x: int\n");
    assert_eq!(
        found,
        [(
            "E0201".to_owned(),
            "a class derived from `typing.NamedTuple` is planned for TyPy 0.10 (CLS-27)".to_owned(),
            "CLS-27".to_owned(),
        )]
    );
}
