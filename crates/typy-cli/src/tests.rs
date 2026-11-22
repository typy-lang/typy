//! Tests of the crate root.

use super::asks_for_codegen_help;

#[test]
fn cli_30_codegen_help_is_asked_for_in_each_spelling_before_two_dashes() {
    let asks = |line: &[&str]| asks_for_codegen_help(line.iter().map(|word| (*word).to_owned()));
    for line in [
        &["-C", "help"][..],
        &["build", "-Chelp"],
        &["build", "app.py", "-C=help"],
    ] {
        assert!(asks(line), "{line:?}");
    }
    for line in [
        &["run", "app.py", "--", "-C", "help"][..],
        &["build", "help"],
        &["-C", "opt-level=2"],
    ] {
        assert!(!asks(line), "{line:?}");
    }
}
