//! Passes a structure by value as the System V calling convention of x86-64 Linux passes a
//! C struct: one of no bytes not at all; one of up to 16 bytes in one or two registers, each
//! eight bytes in a vector register when every value in them is a `float` or a `double`, and
//! in a general register otherwise; and a larger one, or one the registers left cannot hold,
//! in memory (FFI-03, ADR-0049).

use inkwell::context::Context;
use inkwell::types::BasicTypeEnum;
use typy_tyir::{CDataShape, CType, TyirCDataClass};

/// The general registers a call passes arguments in.
const GENERAL_REGISTERS: u32 = 6;
/// The vector registers a call passes arguments in.
const VECTOR_REGISTERS: u32 = 8;

/// Up to eight bytes of a structure, passed in one register.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Piece {
    /// Whether it goes in a vector register: every value in it is a `float` or a `double`.
    pub(crate) vector: bool,
    /// Its bytes, from 1 to 8.
    pub(crate) size: u64,
}

impl Piece {
    /// Returns the LLVM type the piece passes as: `float` or `double` in a vector register,
    /// and a whole word in a general register.
    pub(crate) fn llvm_type(self, context: &Context) -> BasicTypeEnum<'_> {
        match (self.vector, self.size) {
            (true, 4) => context.f32_type().into(),
            (true, _) => context.f64_type().into(),
            (false, _) => context.i64_type().into(),
        }
    }
}

/// How a structure passes.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum Passing {
    /// Not at all: it has no bytes.
    Nothing,
    /// In registers, as these pieces in order.
    Registers(Vec<Piece>),
    /// In memory: a copy on the stack for an argument, and a buffer the caller gives for a
    /// result.
    Memory,
}

/// Returns how the structure class at `index` of `classes` passes, before registers run
/// out.
pub(crate) fn passing(classes: &[TyirCDataClass], index: u32) -> Passing {
    let size = classes[index as usize].size;
    if size == 0 {
        return Passing::Nothing;
    }
    if size > 16 {
        return Passing::Memory;
    }
    let mut values = Vec::new();
    values_of(classes, index, 0, &mut values);
    let pieces = (0..size.div_ceil(8))
        .map(|piece| {
            let start = piece * 8;
            Piece {
                vector: values
                    .iter()
                    .filter(|(offset, _)| (start..start + 8).contains(offset))
                    .all(|(_, vector)| *vector),
                size: (size - start).min(8),
            }
        })
        .collect();
    Passing::Registers(pieces)
}

/// Adds the offset of each scalar a class holds, from `base`, with whether it is a `float` or
/// a `double`.
fn values_of(classes: &[TyirCDataClass], index: u32, base: u64, values: &mut Vec<(u64, bool)>) {
    match &classes[index as usize].shape {
        CDataShape::Scalar(ty) => values.push((base, matches!(ty, CType::Float | CType::Double))),
        CDataShape::Pointer(_) => values.push((base, false)),
        CDataShape::Array(item, length) => {
            let size = classes[*item as usize].size;
            for position in 0..*length {
                values_of(classes, *item, base + position * size, values);
            }
        }
        CDataShape::Structure(fields) => {
            for field in fields {
                values_of(classes, field.class, base + field.offset, values);
            }
        }
    }
}

/// The registers a call has left for its arguments, as each argument takes them in order.
pub(crate) struct Registers {
    general: u32,
    vector: u32,
}

impl Registers {
    /// Returns every register free.
    pub(crate) const fn new() -> Self {
        Self {
            general: GENERAL_REGISTERS,
            vector: VECTOR_REGISTERS,
        }
    }

    /// Takes the register an argument of C type `ty` other than a structure passes in, when
    /// one is left: a vector register for a `float` or a `double`, a general one otherwise.
    pub(crate) fn take_scalar(&mut self, ty: CType) {
        if matches!(ty, CType::Float | CType::Double) {
            self.vector = self.vector.saturating_sub(1);
        } else {
            self.general = self.general.saturating_sub(1);
        }
    }

    /// Takes the registers `pieces` pass in, and returns true, when enough are left for all of
    /// them; returns false, taking none, when the structure passes in memory instead.
    pub(crate) fn take(&mut self, pieces: &[Piece]) -> bool {
        let vector =
            u32::try_from(pieces.iter().filter(|piece| piece.vector).count()).unwrap_or(u32::MAX);
        let general = u32::try_from(pieces.len()).unwrap_or(u32::MAX) - vector;
        if general > self.general || vector > self.vector {
            return false;
        }
        self.general -= general;
        self.vector -= vector;
        true
    }
}

#[cfg(test)]
mod tests {
    use typy_tyir::{CDataShape, CType, TyirCDataClass, TyirField};

    use super::{Passing, Piece, Registers, passing};

    fn scalar(ty: CType, size: u64) -> TyirCDataClass {
        TyirCDataClass {
            name: String::new(),
            shape: CDataShape::Scalar(ty),
            size,
            align: size,
        }
    }

    fn structure(fields: &[(u64, u32)], size: u64) -> TyirCDataClass {
        TyirCDataClass {
            name: String::new(),
            shape: CDataShape::Structure(
                fields
                    .iter()
                    .map(|(offset, class)| TyirField {
                        offset: *offset,
                        class: *class,
                    })
                    .collect(),
            ),
            size,
            align: 8,
        }
    }

    const fn piece(vector: bool, size: u64) -> Piece {
        Piece { vector, size }
    }

    #[test]
    fn each_eight_bytes_pass_in_the_register_their_values_need() {
        // 0 int, 1 double, 2 float, 3 c_char_p, 4 three floats, then the structures.
        let mut classes = vec![
            scalar(CType::Int32, 4),
            scalar(CType::Double, 8),
            scalar(CType::Float, 4),
            scalar(CType::CharPointer, 8),
            TyirCDataClass {
                name: String::new(),
                shape: CDataShape::Array(2, 3),
                size: 12,
                align: 4,
            },
        ];
        let cases = [
            (
                structure(&[(0, 0), (8, 1)], 16),
                Passing::Registers(vec![piece(false, 8), piece(true, 8)]),
            ),
            (
                structure(&[(0, 4)], 12),
                Passing::Registers(vec![piece(true, 8), piece(true, 4)]),
            ),
            (
                structure(&[(0, 2), (4, 0)], 8),
                Passing::Registers(vec![piece(false, 8)]),
            ),
            (
                structure(&[(0, 0)], 4),
                Passing::Registers(vec![piece(false, 4)]),
            ),
            (
                structure(&[(0, 1), (8, 1)], 16),
                Passing::Registers(vec![piece(true, 8), piece(true, 8)]),
            ),
            (structure(&[(0, 3), (8, 3), (16, 3)], 24), Passing::Memory),
            (structure(&[], 0), Passing::Nothing),
        ];
        for (class, expected) in cases {
            classes.push(class);
            let index = u32::try_from(classes.len() - 1).unwrap();
            assert_eq!(passing(&classes, index), expected, "{index}");
        }
        let pointer = TyirCDataClass {
            name: String::new(),
            shape: CDataShape::Pointer(0),
            size: 8,
            align: 8,
        };
        classes.push(pointer);
        let pointer = u32::try_from(classes.len() - 1).unwrap();
        classes.push(structure(&[(0, pointer), (8, 1)], 16));
        let index = u32::try_from(classes.len() - 1).unwrap();
        assert_eq!(
            passing(&classes, index),
            Passing::Registers(vec![piece(false, 8), piece(true, 8)])
        );
    }

    #[test]
    fn a_structure_the_registers_left_cannot_hold_passes_in_memory() {
        let mixed = [piece(false, 8), piece(true, 8)];
        let mut registers = Registers::new();
        for _ in 0..5 {
            registers.take_scalar(CType::Int64);
        }
        assert!(registers.take(&mixed));
        assert!(!registers.take(&mixed));
        for _ in 0..8 {
            registers.take_scalar(CType::Double);
        }
        registers.take_scalar(CType::Float);
        registers.take_scalar(CType::VoidPointer);
        assert!(!registers.take(&[piece(true, 4)]));
    }
}
