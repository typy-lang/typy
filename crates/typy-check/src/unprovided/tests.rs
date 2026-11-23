//! Tests of `unprovided`.

use super::{CPYTHON_3_12_BUILTINS, Unprovided, unprovided};
use typy_diagnostics::RuleId;

#[test]
fn later_builtins_name_their_release_and_rule() {
    let cases = [
        ("super", "0.14", RuleId::CLS_36),
        ("property", "1.0", RuleId::FUN_17),
        ("map", "1.0", RuleId::LIB_25),
    ];
    for (name, release, rule) in cases {
        assert_eq!(
            unprovided(name),
            Some(Unprovided::Planned { release, rule }),
            "{name}"
        );
    }
}

#[test]
fn dynamic_and_unlisted_builtins_are_excluded() {
    for name in [
        "eval",
        "exec",
        "compile",
        "__import__",
        "globals",
        "getattr",
        "id",
        "vars",
        "object",
    ] {
        assert_eq!(unprovided(name), Some(Unprovided::Excluded), "{name}");
    }
}

#[test]
fn names_that_are_not_builtins_are_not_classified() {
    for name in ["frozendict", "sentinel", "total", "Print"] {
        assert_eq!(unprovided(name), None, "{name}");
    }
}

#[test]
fn the_name_list_is_sorted_without_duplicates() {
    let mut sorted = CPYTHON_3_12_BUILTINS.to_vec();
    sorted.sort_unstable();
    sorted.dedup();
    assert_eq!(sorted, CPYTHON_3_12_BUILTINS);
}

#[test]
fn every_builtin_of_0_1_is_a_cpython_builtin() {
    for builtin in [
        "abs", "bool", "chr", "float", "int", "len", "max", "min", "ord", "print", "range", "repr",
        "str",
    ] {
        assert!(CPYTHON_3_12_BUILTINS.contains(&builtin), "{builtin}");
    }
}
