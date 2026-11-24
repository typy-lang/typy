//! Tests of `warning_level`.

use super::WarningLevel;

#[test]
fn a_warning_is_reported_as_one_unless_the_command_line_says_otherwise() {
    assert_eq!(WarningLevel::default(), WarningLevel::Warn);
}

#[test]
fn each_level_is_written_as_the_word_of_its_option() {
    let written = [WarningLevel::Warn, WarningLevel::Allow, WarningLevel::Deny]
        .map(|level| level.to_string());
    assert_eq!(written, ["warn", "allow", "deny"]);
}
