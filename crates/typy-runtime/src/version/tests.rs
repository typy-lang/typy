//! Tests of the runtime's version.

use super::TYPY_RUNTIME_VERSION;

#[test]
fn the_marker_names_the_package_version() {
    let text = core::str::from_utf8(&TYPY_RUNTIME_VERSION).unwrap();
    assert_eq!(
        text,
        format!("typy-runtime-version:{}\n", env!("CARGO_PKG_VERSION"))
    );
}
