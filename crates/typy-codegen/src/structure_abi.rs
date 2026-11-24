//! Passes a structure by value as the target's C calling convention passes a C struct
//! (FFI-03, FFI-12, ADR-0052).
//!
//! System V on x86-64 Linux passes one of no bytes not at all; one of up to 16 bytes in one or
//! two registers, each eight bytes in a vector register when every value in them is a `float`
//! or a `double`, and in a general register otherwise; and a larger one, or one the registers
//! left cannot hold, in memory. AAPCS64 on AArch64 Linux passes one of one to four `float`s
//! or `double`s of one kind as an array of them, in floating-point registers; any other of up
//! to 16 bytes as one or two words; and a larger one as the address of a copy the caller
//! makes. The back end then places each in registers or on the stack.

use inkwell::context::Context;
use inkwell::types::BasicTypeEnum;
use typy_tyir::{CDataShape, CType, TyirCDataClass};

use crate::target::TargetTriple;

/// The general registers a System V call passes arguments in.
const GENERAL_REGISTERS: u32 = 6;
/// The vector registers a System V call passes arguments in.
const VECTOR_REGISTERS: u32 = 8;
/// The most members a homogeneous floating-point aggregate of AAPCS64 has.
const FLOAT_MEMBERS: usize = 4;

/// Up to eight bytes of a structure, passed in one register.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Piece {
    /// Whether it goes in a vector register: every value in it is a `float` or a `double`.
    pub(crate) vector: bool,
    /// Its bytes, from 1 to 8.
    pub(crate) size: u64,
    /// Where it starts in the structure.
    pub(crate) offset: u64,
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
    /// In registers, each piece a parameter of its own, and a result of two as a pair
    /// (System V).
    Registers(Vec<Piece>),
    /// As one array of its pieces, a parameter and a result alike (AAPCS64).
    Array(Vec<Piece>),
    /// In memory: a copy on the stack for an argument, and a buffer the caller gives for a
    /// result (System V).
    Memory,
    /// As the address of a copy the caller makes for an argument, and a buffer the caller
    /// gives for a result (AAPCS64).
    Reference,
}

/// Returns how the structure class at `index` of `classes` passes on `target`; on x86-64,
/// before registers run out.
pub(crate) fn passing(classes: &[TyirCDataClass], index: u32, target: TargetTriple) -> Passing {
    let size = classes[index as usize].size;
    if size == 0 {
        return Passing::Nothing;
    }
    let mut values = Vec::new();
    values_of(classes, index, 0, &mut values);
    match target.architecture() {
        "aarch64" => aapcs64(size, &values),
        _ => system_v(size, &values),
    }
}

/// Returns how System V passes a structure of `size` bytes holding `values`.
fn system_v(size: u64, values: &[(u64, Option<CType>)]) -> Passing {
    if size > 16 {
        return Passing::Memory;
    }
    Passing::Registers(words(size, |start| {
        values
            .iter()
            .filter(|(offset, _)| (start..start + 8).contains(offset))
            .all(|(_, ty)| matches!(ty, Some(CType::Float | CType::Double)))
    }))
}

/// Returns how AAPCS64 passes a structure of `size` bytes holding `values`.
fn aapcs64(size: u64, values: &[(u64, Option<CType>)]) -> Passing {
    if let Some((_, Some(first @ (CType::Float | CType::Double)))) = values.first() {
        let homogeneous =
            values.len() <= FLOAT_MEMBERS && values.iter().all(|(_, ty)| *ty == Some(*first));
        if homogeneous {
            let member = if *first == CType::Float { 4 } else { 8 };
            return Passing::Array(
                values
                    .iter()
                    .map(|(offset, _)| Piece {
                        vector: true,
                        size: member,
                        offset: *offset,
                    })
                    .collect(),
            );
        }
    }
    if size > 16 {
        return Passing::Reference;
    }
    let pieces = words(size, |_| false);
    if pieces.len() == 1 {
        return Passing::Registers(pieces);
    }
    Passing::Array(pieces)
}

/// Returns the eight-byte pieces of `size` bytes, each a vector piece when `vector` says so
/// for its start.
fn words(size: u64, vector: impl Fn(u64) -> bool) -> Vec<Piece> {
    (0..size.div_ceil(8))
        .map(|piece| {
            let start = piece * 8;
            Piece {
                vector: vector(start),
                size: (size - start).min(8),
                offset: start,
            }
        })
        .collect()
}

/// Adds the offset of each scalar a class holds, from `base`, with its C type, or `None` for a
/// pointer.
fn values_of(
    classes: &[TyirCDataClass],
    index: u32,
    base: u64,
    values: &mut Vec<(u64, Option<CType>)>,
) {
    match &classes[index as usize].shape {
        CDataShape::Scalar(ty) => values.push((base, Some(*ty))),
        CDataShape::Pointer(_) => values.push((base, None)),
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

/// The registers a call has left for its arguments, as each argument takes them in order. Only
/// System V asks: AAPCS64's back end places a structure that registers cannot hold on the stack
/// itself.
pub(crate) struct Registers {
    general: u32,
    vector: u32,
}

impl Registers {
    /// Returns every register of `target` free; on AArch64, as many as any call needs.
    pub(crate) fn new(target: TargetTriple) -> Self {
        if target.architecture() == "aarch64" {
            return Self {
                general: u32::MAX,
                vector: u32::MAX,
            };
        }
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
mod tests;
