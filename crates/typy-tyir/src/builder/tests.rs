//! Tests of `builder`.

use super::FunctionBuilder;
use crate::block::BlockId;
use crate::constant::Constant;
use crate::function::FunctionId;
use crate::operation::Operation;
use crate::terminator::Terminator;
use crate::types::TyirType;
use typy_diagnostics::{FileId, Span, TextOffset};

fn span() -> Span {
    Span::empty(FileId(0), TextOffset(0))
}

#[test]
fn parameters_belong_to_the_entry_block() {
    let mut builder = FunctionBuilder::new(FunctionId(0), "f", "f", TyirType::Int);
    let parameter = builder.add_parameter(TyirType::Int);
    builder.terminate(Terminator::Return(parameter));
    let function = builder.finish();
    assert_eq!(function.parameters, [parameter]);
    assert_eq!(function.block(BlockId(0)).parameters, [parameter]);
    assert_eq!(function.value(parameter).ty, TyirType::Int);
}

#[test]
fn code_after_a_terminator_goes_to_a_new_block() {
    let mut builder = FunctionBuilder::new(FunctionId(0), "f", "f", TyirType::Unit);
    let unit = builder.push_value(Operation::Constant(Constant::Unit), TyirType::Unit, span());
    builder.terminate(Terminator::Return(unit));
    assert!(builder.is_terminated());
    builder.push_value(
        Operation::Constant(Constant::Bool(true)),
        TyirType::Bool,
        span(),
    );
    assert_eq!(builder.current_block(), BlockId(1));
    let function = builder.finish();
    assert_eq!(function.blocks.len(), 2);
    assert_eq!(function.blocks[1].terminator, Terminator::Unreachable);
}

#[test]
fn slots_and_block_parameters_are_numbered_in_order() {
    let mut builder = FunctionBuilder::new(FunctionId(3), "sym", "name", TyirType::Unit);
    let first = builder.add_slot("a", TyirType::Int);
    let second = builder.add_slot("b", TyirType::STR);
    assert_eq!(builder.slot_type(second), TyirType::STR);
    assert_eq!(builder.slot_count(), 2);
    let block = builder.create_block();
    let parameter = builder.add_block_parameter(block, TyirType::EXCEPTION);
    builder.set_may_raise(true);
    builder.switch_to(block);
    builder.push(Operation::Increment(parameter), span());
    builder.terminate(Terminator::ReturnError(parameter));
    let function = builder.finish();
    assert_eq!((first.0, second.0), (0, 1));
    assert_eq!(function.slot(second).name, "b");
    assert_eq!(function.slots.len(), 2);
    assert_eq!(function.block(block).parameters, [parameter]);
    assert!(function.may_raise);
    assert_eq!(function.symbol, "sym");
    assert_eq!(function.name, "name");
}
