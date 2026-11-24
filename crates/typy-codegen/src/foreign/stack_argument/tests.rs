//! Tests of `foreign::stack_argument`.

use inkwell::context::Context;
use typy_tyir::CType;

use super::{GeneralRegisters, stack_type};
use crate::structure_abi::{Passing, Piece};

fn word(vector: bool) -> Piece {
    Piece {
        vector,
        size: 8,
        offset: 0,
    }
}

#[test]
fn eight_integer_arguments_go_in_registers_and_the_ninth_on_the_stack() {
    let mut registers = GeneralRegisters::new();
    for _ in 0..8 {
        assert!(registers.take_scalar(CType::Int8));
    }
    assert!(registers.take_scalar(CType::Double));
    assert!(!registers.take_scalar(CType::UInt16));
}

#[test]
fn a_structure_the_registers_left_cannot_hold_takes_the_rest_of_them() {
    let mut registers = GeneralRegisters::new();
    for _ in 0..7 {
        registers.take_scalar(CType::Int32);
    }
    registers.take_structure(&Passing::Array(vec![word(false), word(false)]));
    assert!(!registers.take_scalar(CType::Int8));
}

#[test]
fn structures_take_a_register_for_each_word_or_their_address() {
    let mut registers = GeneralRegisters::new();
    registers.take_structure(&Passing::Array(vec![word(false), word(false)]));
    registers.take_structure(&Passing::Registers(vec![word(false)]));
    registers.take_structure(&Passing::Reference);
    registers.take_structure(&Passing::Array(vec![word(true); 4]));
    registers.take_structure(&Passing::Nothing);
    for _ in 0..4 {
        assert!(registers.take_scalar(CType::VoidPointer));
    }
    assert!(!registers.take_scalar(CType::Int64));
}

#[test]
fn a_stack_argument_takes_its_c_width_with_the_extension_clang_gives_it() {
    let context = Context::create();
    for (ty, bits, extension) in [
        (CType::Int8, 8, Some("signext")),
        (CType::UInt8, 8, Some("zeroext")),
        (CType::Int16, 16, Some("signext")),
        (CType::UInt16, 16, Some("zeroext")),
        (CType::Int32, 32, None),
        (CType::UInt32, 32, None),
        (CType::Bool, 1, Some("zeroext")),
    ] {
        let (stack, attribute) = stack_type(&context, ty).unwrap();
        assert_eq!(stack.get_bit_width(), bits, "{ty:?}");
        let kind = extension.map(inkwell::attributes::Attribute::get_named_enum_kind_id);
        assert_eq!(
            attribute.map(inkwell::attributes::Attribute::get_enum_kind_id),
            kind,
            "{ty:?}"
        );
    }
    for ty in [
        CType::Int64,
        CType::UInt64,
        CType::Double,
        CType::VoidPointer,
    ] {
        assert!(stack_type(&context, ty).is_none(), "{ty:?}");
    }
}
