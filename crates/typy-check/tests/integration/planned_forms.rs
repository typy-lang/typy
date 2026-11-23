//! The forms of typed Python a later release gives: each refusal names the release and the
//! requirement that plan it (ADR-0090 to ADR-0097).

use typy_source::PythonFloor;

use crate::common::{messages, messages_at};

fn planned(name: &str, release: &str, rule: &str) -> (String, String, String) {
    (
        "E0201".to_owned(),
        format!("`{name}` is planned for TyPy {release} ({rule})"),
        rule.to_owned(),
    )
}

fn planned_module(name: &str, release: &str, rule: &str) -> (String, String, String) {
    (
        "E0201".to_owned(),
        format!("the module `{name}` is planned for TyPy {release} ({rule})"),
        rule.to_owned(),
    )
}

#[test]
fn each_typing_form_of_a_later_release_names_it() {
    let found = messages(
        "from typing import Any\n\
         from typing import TypedDict, Required, NotRequired, ReadOnly\n",
    );
    assert_eq!(
        found,
        [
            planned("typing.Any", "0.13", "TYP-25"),
            planned("typing.TypedDict", "0.13", "TYP-32"),
            planned("typing.Required", "0.13", "TYP-32"),
            planned("typing.NotRequired", "0.13", "TYP-32"),
            planned("typing.ReadOnly", "0.13", "TYP-32"),
        ]
    );
}

#[test]
fn each_standard_library_module_of_a_later_release_names_it() {
    let cases = [
        ("enum", "0.12", "CLS-44"),
        ("abc", "0.12", "CLS-45"),
        ("textwrap", "0.12", "LIB-27"),
        ("base64", "0.12", "LIB-27"),
        ("heapq", "0.12", "LIB-27"),
        ("bisect", "0.12", "LIB-27"),
        ("statistics", "0.12", "LIB-27"),
        ("time", "0.12", "LIB-27"),
        ("random", "0.12", "LIB-27"),
        ("copy", "0.12", "LIB-27"),
        ("string", "0.12", "LIB-27"),
        ("os", "1.0", "LIB-07"),
    ];
    for (module, release, rule) in cases {
        assert_eq!(
            messages(&format!("import {module}\n")),
            [planned_module(module, release, rule)],
            "{module}"
        );
    }
}

#[test]
fn doctest_is_planned_for_after_1_0() {
    assert_eq!(
        messages("import doctest\n"),
        [(
            "E0201".to_owned(),
            "the module `doctest` is planned for a release after TyPy 1.0".to_owned(),
            "LIB-08".to_owned(),
        )]
    );
}

#[test]
fn any_as_a_type_is_planned_for_0_13() {
    let found = messages("def show(value: Any) -> None:\n    pass\n");
    assert_eq!(
        found,
        [(
            "E0201".to_owned(),
            "`Any` as a type is planned for TyPy 0.13 (TYP-25)".to_owned(),
            "TYP-25".to_owned(),
        )]
    );
}

#[test]
fn frozendict_and_math_integer_are_planned_from_the_floor_3_15() {
    assert_eq!(
        messages_at("value = frozendict()\n", PythonFloor::V3_15),
        [planned("frozendict", "0.12", "DAT-18")]
    );
    assert_eq!(
        messages_at("import math.integer\n", PythonFloor::V3_15),
        [planned_module("math.integer", "0.12", "LIB-26")]
    );
    // Below 3.15 CPython has neither, so TyPy says what CPython says.
    assert_eq!(messages("value = frozendict()\n")[0].0, "E0205");
    assert_eq!(messages("import math.integer\n")[0].0, "E0206");
}

#[test]
fn a_typing_name_of_a_later_release_is_planned_only_from_typing() {
    let found =
        messages("from collections.abc import Literal, overload, Self, TypeIs, Any, TypedDict\n");
    assert_eq!(found.len(), 6, "{found:?}");
    assert!(
        found
            .iter()
            .all(|(code, text, _)| code == "E0425" && text.starts_with("TyPy's `collections.abc`")),
        "{found:?}"
    );
}
