//! Tests of `registry`.

use super::{CODES, lookup};
use crate::code::DiagnosticCode;

#[test]
fn codes_are_sorted_and_unique() {
    for pair in CODES.windows(2) {
        assert!(
            pair[0].code < pair[1].code,
            "{} then {}",
            pair[0].code,
            pair[1].code
        );
    }
}

#[test]
fn every_code_is_found_by_lookup() {
    for info in CODES {
        assert_eq!(lookup(info.code), Some(info));
    }
}

#[test]
fn unknown_code_is_not_found() {
    assert_eq!(lookup(DiagnosticCode::error(9999)), None);
    assert_eq!(lookup(DiagnosticCode::warning(9999)), None);
}

#[test]
fn summaries_start_in_lower_case_unless_with_a_proper_name_and_have_no_final_period() {
    const PROPER_NAMES: [&str; 2] = ["C", "Python"];
    for info in CODES {
        let first_word = info.summary.split(' ').next().unwrap();
        let first = first_word.chars().next().unwrap();
        if first.is_uppercase() {
            assert!(PROPER_NAMES.contains(&first_word), "{}", info.code);
        }
        assert!(!info.summary.ends_with('.'), "{}", info.code);
    }
}
