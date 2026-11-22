//! Tests of `ctypes::structure`.

use super::StructureClass;
use crate::ctypes::{CData, CType};

fn structure(fields: &[(&str, CData)]) -> StructureClass {
    let fields = fields
        .iter()
        .map(|(name, class)| ((*name).to_owned(), class.clone()))
        .collect();
    StructureClass::new(0, "S", "__main__", fields)
}

#[test]
fn each_field_sits_at_the_offset_c_gives_it_and_the_size_rounds_to_the_alignment() {
    let int = CData::Scalar(CType::Int32);
    let double = CData::Scalar(CType::Double);
    let point = structure(&[("x", int.clone()), ("y", double.clone())]);
    assert_eq!((point.size, point.align), (16, 8));
    assert_eq!(point.fields[1].offset, 8);
    let mixed = structure(&[
        ("a", CData::Scalar(CType::CharPointer)),
        ("b", CData::Scalar(CType::Int8)),
        ("c", CData::Scalar(CType::Int64)),
        ("d", CData::Scalar(CType::Bool)),
    ]);
    let offsets: Vec<u64> = mixed.fields.iter().map(|field| field.offset).collect();
    assert_eq!((offsets, mixed.size), (vec![0, 8, 16, 24], 32));
    let boxed = structure(&[
        ("corner", CData::Structure(std::sync::Arc::new(point))),
        ("size", CData::Array(Box::new(int.clone()), 2)),
        ("tag", CData::Scalar(CType::CharPointer)),
        ("next", CData::Pointer(Box::new(double))),
    ]);
    let offsets: Vec<u64> = boxed.fields.iter().map(|field| field.offset).collect();
    assert_eq!((offsets, boxed.size), (vec![0, 16, 24, 32], 40));
    let small = structure(&[
        ("c", CData::Scalar(CType::Int8)),
        ("s", CData::Scalar(CType::Int16)),
    ]);
    assert_eq!((small.fields[1].offset, small.size, small.align), (2, 4, 2));
    let empty = structure(&[]);
    assert_eq!((empty.size, empty.align), (0, 1));
    assert_eq!(boxed.field("tag").map(|(index, _)| index), Some(2));
    assert_eq!(boxed.field("missing"), None);
    assert_eq!(structure(&[("x", int)]).field("x").unwrap().1.offset, 0);
}
