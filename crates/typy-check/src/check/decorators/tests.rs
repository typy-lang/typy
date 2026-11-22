//! Tests of `check::decorators`.

use super::{UNDECORATED, python_name};

#[test]
fn a_hidden_name_shows_as_the_functions_own() {
    assert_eq!(python_name(&format!("inc{UNDECORATED}")), "inc");
    assert_eq!(
        python_name(&format!("outer{UNDECORATED}.<locals>.step")),
        "outer.<locals>.step"
    );
    assert_eq!(python_name("plain"), "plain");
}
