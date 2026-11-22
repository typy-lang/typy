//! Tests of `emission`.

use super::Emission;

#[test]
fn both_kinds_give_their_bytes() {
    assert_eq!(Emission::Text("ir".to_owned()).as_bytes(), b"ir");
    assert_eq!(Emission::Bytes(vec![0x7f, b'E']).as_bytes(), [0x7f, b'E']);
}
