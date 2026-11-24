//! Tests of `protocol_fits`.

use super::{FitsScope, fits, note};
use crate::typed::ClassId;

#[test]
fn a_scope_holds_its_fits_and_puts_back_the_table_it_replaced() {
    let outer = FitsScope::new([(ClassId(1), ClassId(2))]);
    {
        let _inner = FitsScope::new([]);
        note(ClassId(3), ClassId(2));
        assert!(fits(ClassId(3), ClassId(2)));
        assert!(!fits(ClassId(1), ClassId(2)));
    }
    assert!(fits(ClassId(1), ClassId(2)));
    assert!(!fits(ClassId(3), ClassId(2)));
    drop(outer);
    assert!(!fits(ClassId(1), ClassId(2)));
}
