//! Tests of `module`.

use super::ModuleName;

#[test]
fn dotted_identifiers_are_module_names() {
    for text in ["__main__", "app", "app.models", "_private.x1", "caf\u{e9}"] {
        assert_eq!(ModuleName::new(text).unwrap().as_str(), text);
    }
}

#[test]
fn texts_with_an_empty_or_invalid_part_are_refused() {
    for text in ["", ".", "app.", ".app", "app..x", "1app", "app-x", "app x"] {
        let error = ModuleName::new(text).unwrap_err();
        assert_eq!(error.text, text);
        assert!(error.to_string().contains("not a module name"));
    }
}

#[test]
fn the_main_module_is_named_dunder_main() {
    assert_eq!(ModuleName::main().to_string(), "__main__");
}
