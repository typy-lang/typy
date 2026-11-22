//! Tests of the object header.

use super::ObjectHeader;

#[test]
fn the_header_takes_eight_bytes() {
    assert_eq!(core::mem::size_of::<ObjectHeader>(), 8);
    assert_eq!(ObjectHeader::new(1).reference_count, 1);
}
