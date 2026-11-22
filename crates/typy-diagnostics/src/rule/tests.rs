//! Tests of `rule`.

use super::RuleId;

fn specification() -> String {
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../docs/requirements/srs.md"
    );
    std::fs::read_to_string(path).expect("the specification is part of the repository")
}

#[test]
fn every_rule_is_an_id_defined_in_the_specification() {
    let specification = specification();
    for rule in RuleId::ALL {
        let row = format!("\n| {rule} |");
        assert!(
            specification.contains(&row),
            "{rule} is not defined in srs.md"
        );
    }
}

#[test]
fn the_list_of_rules_is_sorted_and_has_no_duplicates() {
    let ids: Vec<&str> = RuleId::ALL.iter().map(|rule| rule.as_str()).collect();
    let mut sorted = ids.clone();
    sorted.sort_unstable();
    sorted.dedup();
    assert_eq!(ids, sorted);
}
