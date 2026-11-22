//! Tests of `error`.

use super::CodegenError;

#[test]
fn errors_show_their_message() {
    assert_eq!(CodegenError("x".to_owned()).to_string(), "x");
}
