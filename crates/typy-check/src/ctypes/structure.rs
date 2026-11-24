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
mod tests;
