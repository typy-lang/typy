//! Tests of `iterator_fits`.

use super::{Iteration, iteration, note, reset, take};
use crate::typed::ClassId;
use crate::types::Type;

#[test]
fn the_table_holds_what_is_noted_until_it_is_reset() {
    note(ClassId(1), Iteration::Iterator(Type::Int));
    assert_eq!(iteration(ClassId(1)), Some(Iteration::Iterator(Type::Int)));
    assert_eq!(
        iteration(ClassId(1)).map(|each| each.item().clone()),
        Some(Type::Int)
    );
    reset(&std::collections::BTreeMap::new());
    assert_eq!(iteration(ClassId(1)), None);
    note(ClassId(2), Iteration::Iterable(Type::Str));
    assert_eq!(take().len(), 1);
    assert_eq!(iteration(ClassId(2)), None);
}
