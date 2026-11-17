//! A `ctypes.Structure` subclass a module declares with a literal `_fields_` list: its
//! fields and their offsets, laid out as a C compiler on 64-bit Linux lays out a struct
//! (FFI-03, ADR-0049).

use super::CData;

/// A structure class.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct StructureClass {
    /// Its number in the program, which tells apart two classes of one name.
    pub id: u32,
    /// The class's name.
    pub name: String,
    /// The module that declares it, which its `repr` names.
    pub module: String,
    /// The fields, in the order `_fields_` lists them.
    pub fields: Vec<StructureField>,
    /// The bytes of an object, a multiple of its alignment.
    pub size: u64,
    /// The alignment of an object: the largest of its fields', and 1 without fields.
    pub align: u64,
}

/// A field of a structure class.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct StructureField {
    /// The field's name.
    pub name: String,
    /// Its class.
    pub class: CData,
    /// Its offset from the start of the structure.
    pub offset: u64,
}

impl StructureClass {
    /// Returns the class `name` of `module` with `fields`, each placed at the next offset its
    /// alignment allows, as C places a struct's members.
    #[must_use]
    pub fn new(id: u32, name: &str, module: &str, fields: Vec<(String, CData)>) -> Self {
        let mut offset: u64 = 0;
        let mut align: u64 = 1;
        let fields = fields
            .into_iter()
            .map(|(name, class)| {
                let field_align = class.align();
                align = align.max(field_align);
                offset = offset.next_multiple_of(field_align);
                let field = StructureField {
                    name,
                    class,
                    offset,
                };
                offset += field.class.size();
                field
            })
            .collect();
        Self {
            id,
            name: name.to_owned(),
            module: module.to_owned(),
            fields,
            size: offset.next_multiple_of(align),
            align,
        }
    }

    /// Returns the field named `name`, with its position.
    #[must_use]
    pub fn field(&self, name: &str) -> Option<(usize, &StructureField)> {
        self.fields
            .iter()
            .enumerate()
            .find(|(_, field)| field.name == name)
    }
}

#[cfg(test)]
mod tests {
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
}
