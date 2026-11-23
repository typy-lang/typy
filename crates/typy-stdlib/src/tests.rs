//! Tests of `typy-stdlib`.

use super::{names, source};

#[test]
fn each_module_has_its_source_by_its_name() {
    for name in names() {
        let text = source(name).expect("a module of the list has a source");
        assert!(
            text.starts_with("\"\"\""),
            "{name} opens with its docstring"
        );
    }
    assert_eq!(source("no_such_module"), None);
}
