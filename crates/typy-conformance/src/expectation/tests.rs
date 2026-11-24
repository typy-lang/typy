//! Tests of `expectation`.

use super::Expectation;

#[test]
fn both_forms_parse_and_print_back() {
    for text in ["accepted", "rejected E0302 at 4:12"] {
        assert_eq!(Expectation::parse(text).unwrap().to_string(), text);
    }
}

#[test]
fn malformed_expectations_are_refused() {
    for text in [
        "",
        "accept",
        "rejected E0302",
        "rejected E0302 at 4",
        "rejected E302 at 4:12",
        "rejected E0302 at 0:1",
        "rejected E0302 at 1:x",
    ] {
        assert_eq!(Expectation::parse(text), None, "{text:?}");
    }
}
