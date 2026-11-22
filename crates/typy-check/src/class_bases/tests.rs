//! Tests of `class_bases`.

use super::{derives, note, reset, take};
use crate::typed::ClassId;

#[test]
fn a_class_derives_from_itself_and_the_bases_noted_for_it() {
    let before = take();
    note(ClassId(2), vec![ClassId(1), ClassId(0)]);
    assert!(derives(ClassId(2), ClassId(0)));
    assert!(derives(ClassId(1), ClassId(1)));
    assert!(!derives(ClassId(1), ClassId(2)));
    reset(&before);
}
