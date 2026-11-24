//! Tests of `builtin`.

use super::Builtin;

#[test]
fn every_builtin_is_found_by_its_name() {
    for name in [
        "abs",
        "all",
        "any",
        "bool",
        "bytes",
        "callable",
        "chr",
        "dict",
        "enumerate",
        "float",
        "int",
        "isinstance",
        "issubclass",
        "len",
        "list",
        "max",
        "min",
        "ord",
        "print",
        "range",
        "repr",
        "reversed",
        "round",
        "set",
        "sorted",
        "str",
        "sum",
        "tuple",
        "zip",
    ] {
        let builtin = Builtin::named(name).unwrap();
        assert_eq!(builtin.name(), name);
    }
    assert_eq!(Builtin::named("eval"), None);
}
