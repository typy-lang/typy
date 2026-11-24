//! Tests of `structure_abi`.

use typy_tyir::{CDataShape, CType, TyirCDataClass, TyirField};

use super::{Passing, Piece, Registers, passing};
use crate::target::TargetTriple;

const X86_64: TargetTriple = TargetTriple::X86_64LinuxGnu;
const AARCH64: TargetTriple = TargetTriple::Aarch64LinuxGnu;

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

/// Returns pieces of eight bytes in order, each vector or not, with the size given.
fn words(pieces: &[(bool, u64)]) -> Vec<Piece> {
    (0..)
        .zip(pieces)
        .map(|(index, (vector, size))| Piece {
            vector: *vector,
            size: *size,
            offset: index * 8,
        })
        .collect()
}

/// Returns the members of a homogeneous floating-point aggregate of `count` members of
/// `size` bytes.
fn floats(count: u64, size: u64) -> Vec<Piece> {
    (0..count)
        .map(|index| Piece {
            vector: true,
            size,
            offset: index * size,
        })
        .collect()
}

/// The scalar classes the structures below are made of: 0 int, 1 double, 2 float,
/// 3 `c_char_p`, 4 three floats, 5 int8, 6 int64.
fn classes() -> Vec<TyirCDataClass> {
    vec![
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
        scalar(CType::Int8, 1),
        scalar(CType::Int64, 8),
    ]
}

/// Returns how `class` passes on `target`, added to the scalar classes.
fn passed(class: TyirCDataClass, target: TargetTriple) -> Passing {
    let mut classes = classes();
    classes.push(class);
    passing(&classes, u32::try_from(classes.len() - 1).unwrap(), target)
}

#[test]
fn each_eight_bytes_pass_in_the_register_their_values_need_on_x86_64() {
    let cases = [
        (
            structure(&[(0, 0), (8, 1)], 16),
            Passing::Registers(words(&[(false, 8), (true, 8)])),
        ),
        (
            structure(&[(0, 4)], 12),
            Passing::Registers(words(&[(true, 8), (true, 4)])),
        ),
        (
            structure(&[(0, 2), (4, 0)], 8),
            Passing::Registers(words(&[(false, 8)])),
        ),
        (
            structure(&[(0, 0)], 4),
            Passing::Registers(words(&[(false, 4)])),
        ),
        (
            structure(&[(0, 1), (8, 1)], 16),
            Passing::Registers(words(&[(true, 8), (true, 8)])),
        ),
        (
            structure(&[(0, 3), (8, 1)], 16),
            Passing::Registers(words(&[(false, 8), (true, 8)])),
        ),
        (structure(&[(0, 3), (8, 3), (16, 3)], 24), Passing::Memory),
        (
            structure(&[(0, 1), (8, 1), (16, 1), (24, 1)], 32),
            Passing::Memory,
        ),
        (structure(&[], 0), Passing::Nothing),
    ];
    for (class, expected) in cases {
        assert_eq!(passed(class.clone(), X86_64), expected, "{class:?}");
    }
}

/// The cases of ADR-0052, each as clang 19 lowers the same C structure for
/// `aarch64-linux-gnu`.
#[test]
fn each_structure_passes_as_clang_lowers_it_on_aarch64() {
    let cases = [
        // {int, int}: one word, i64.
        (
            structure(&[(0, 0), (4, 0)], 8),
            Passing::Registers(words(&[(false, 8)])),
        ),
        // {double, double}: [2 x double].
        (
            structure(&[(0, 1), (8, 1)], 16),
            Passing::Array(floats(2, 8)),
        ),
        // {float, float, float}, as three fields and as an array of three.
        (
            structure(&[(0, 2), (4, 2), (8, 2)], 12),
            Passing::Array(floats(3, 4)),
        ),
        (structure(&[(0, 4)], 12), Passing::Array(floats(3, 4))),
        // Four doubles: [4 x double], in registers, where x86-64 passes them in memory.
        (
            structure(&[(0, 1), (8, 1), (16, 1), (24, 1)], 32),
            Passing::Array(floats(4, 8)),
        ),
        // Five floats are no aggregate of floats, and 20 bytes go by reference.
        (
            structure(&[(0, 2), (4, 2), (8, 2), (12, 2), (16, 2)], 20),
            Passing::Reference,
        ),
        // {int, double}: [2 x i64].
        (
            structure(&[(0, 0), (8, 1)], 16),
            Passing::Array(words(&[(false, 8), (false, 8)])),
        ),
        // {float, double} mixes kinds: [2 x i64].
        (
            structure(&[(0, 2), (8, 1)], 16),
            Passing::Array(words(&[(false, 8), (false, 8)])),
        ),
        // {char, char, char}: one word.
        (
            structure(&[(0, 5), (1, 5), (2, 5)], 3),
            Passing::Registers(words(&[(false, 3)])),
        ),
        // {long, long, long}: 24 bytes by reference.
        (
            structure(&[(0, 6), (8, 6), (16, 6)], 24),
            Passing::Reference,
        ),
        // A pointer and a double: [2 x i64].
        (
            structure(&[(0, 3), (8, 1)], 16),
            Passing::Array(words(&[(false, 8), (false, 8)])),
        ),
        (structure(&[], 0), Passing::Nothing),
    ];
    for (class, expected) in cases {
        assert_eq!(passed(class.clone(), AARCH64), expected, "{class:?}");
    }
}

#[test]
fn a_structure_the_registers_left_cannot_hold_passes_in_memory_on_x86_64() {
    let mixed = words(&[(false, 8), (true, 8)]);
    let mut registers = Registers::new(X86_64);
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
    assert!(!registers.take(&words(&[(true, 4)])));
}

#[test]
fn aarch64_leaves_running_out_of_registers_to_the_back_end() {
    let mut registers = Registers::new(AARCH64);
    for _ in 0..16 {
        registers.take_scalar(CType::Double);
        registers.take_scalar(CType::Int64);
    }
    assert!(registers.take(&words(&[(true, 8), (false, 8)])));
}
