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
        "from typing import Any, cast, Literal, overload, Self, TypeGuard, TypeIs\n\
         from typing import TypedDict, Required, NotRequired, ReadOnly\n",
    );
    assert_eq!(
        found,
        [
            planned("typing.Any", "0.13", "TYP-25"),
            planned("typing.cast", "0.11", "TYP-24"),
            planned("typing.Literal", "0.12", "TYP-28"),
            planned("typing.overload", "0.12", "TYP-29"),
            planned("typing.Self", "0.12", "TYP-30"),
            planned("typing.TypeGuard", "0.12", "TYP-31"),
            planned("typing.TypeIs", "0.12", "TYP-31"),
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
fn object_as_a_type_is_planned_for_0_11() {
    let found = messages("def show(value: object) -> None:\n    pass\n");
    assert_eq!(
        found,
        [(
            "E0201".to_owned(),
            "`object` as a type is planned for TyPy 0.11 (TYP-23)".to_owned(),
            "TYP-23".to_owned(),
        )]
    );
}

#[test]
fn the_values_of_a_literal_are_not_read_as_annotations() {
    let found = messages(
        "from typing import Literal\ndef open_as(mode: Literal['r', 'w']) -> str:\n    \
         return mode\n",
    );
    assert!(
        found.contains(&planned("typing.Literal", "0.12", "TYP-28")),
        "{found:?}"
    );
    assert!(
        found
            .iter()
            .all(|(_, message, _)| !message.contains("written as strings")),
        "{found:?}"
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
