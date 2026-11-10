//! Every value is defined exactly once, and its definition comes before each use on every path.

use std::collections::BTreeMap;

use super::VerifyError;
use super::dominance::dominators;
use crate::block::BlockId;
use crate::function::TyirFunction;
use crate::operation::Operation;
use crate::value::ValueId;

/// Where a value is defined: its block, and its position (parameters come before instruction 0).
#[derive(Clone, Copy)]
struct Definition {
    block: BlockId,
    position: usize,
}

pub(super) fn check(function: &TyirFunction) -> Result<(), VerifyError> {
    let error = |block: Option<BlockId>, message: String| VerifyError {
        function: function.symbol.clone(),
        block,
        message,
    };
    let mut defined: BTreeMap<ValueId, Definition> = BTreeMap::new();
    for block in &function.blocks {
        let definitions = block.parameters.iter().map(|value| (*value, 0)).chain(
            block
                .instructions
                .iter()
                .enumerate()
                .filter_map(|(position, instruction)| {
                    instruction.result.map(|value| (value, position + 1))
                }),
        );
        for (value, position) in definitions {
            if value.0 as usize >= function.values.len() {
                return Err(error(Some(block.id), format!("v{} has no type", value.0)));
            }
            if defined
                .insert(
                    value,
                    Definition {
                        block: block.id,
                        position,
                    },
                )
                .is_some()
            {
                return Err(error(
                    Some(block.id),
                    format!("v{} is defined more than once", value.0),
                ));
            }
        }
    }
    let dominators = dominators(function);
    for block in &function.blocks {
        let uses = block
            .instructions
            .iter()
            .enumerate()
            .flat_map(|(position, instruction)| {
                operands(&instruction.operation)
                    .into_iter()
                    .map(move |value| (value, position + 1))
            })
            .chain(
                block
                    .terminator
                    .operands()
                    .into_iter()
                    .map(|value| (value, usize::MAX)),
            );
        for (value, position) in uses {
            let Some(definition) = defined.get(&value) else {
                return Err(error(
                    Some(block.id),
                    format!("v{} is used but never defined", value.0),
                ));
            };
            let dominates = if definition.block == block.id {
                definition.position < position
            } else {
                dominators[block.id.0 as usize].contains(&definition.block)
            };
            if !dominates {
                return Err(error(
                    Some(block.id),
                    format!("v{} is used where its definition may not have run", value.0),
                ));
            }
        }
    }
    Ok(())
}

/// Returns the values an operation reads.
pub(super) fn operands(operation: &Operation) -> Vec<ValueId> {
    match operation {
        Operation::Constant(_) | Operation::Load(_) | Operation::NewInstance { .. } => Vec::new(),
        Operation::Store { value, .. }
        | Operation::IntUnary { operand: value, .. }
        | Operation::FloatNegate(value)
        | Operation::BoolNot(value)
        | Operation::IsTrue { value, .. }
        | Operation::Increment(value)
        | Operation::Decrement(value)
        | Operation::TupleItem { tuple: value, .. }
        | Operation::ToBits(value)
        | Operation::Wrap { value, .. }
        | Operation::Unwrap(value)
        | Operation::IsNone(value)
        | Operation::FromBits { bits: value, .. }
        | Operation::Field { object: value, .. } => vec![*value],
        Operation::SetField { object, value, .. } => vec![*object, *value],
        Operation::Identical { left, right }
        | Operation::IntBinary { left, right, .. }
        | Operation::FloatBinary { left, right, .. }
        | Operation::BoolBinary { left, right, .. }
        | Operation::Compare { left, right, .. } => vec![*left, *right],
        Operation::CallRuntime { arguments, .. }
        | Operation::Call { arguments, .. }
        | Operation::MakeTuple {
            items: arguments, ..
        } => arguments.clone(),
    }
}
