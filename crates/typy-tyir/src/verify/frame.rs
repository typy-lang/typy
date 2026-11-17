//! Only an instance of a class of the program that is not an exception is held in a frame,
//! since the runtime makes and releases nothing else there (MEM-03, ADR-0042).

use std::collections::BTreeSet;

use super::VerifyError;
use crate::escape::may_be_in_frame;
use crate::function::TyirFunction;
use crate::module::TyirModule;
use crate::operation::Operation;
use crate::value::{ValueId, ValueRepresentation};

pub(super) fn check(module: &TyirModule, function: &TyirFunction) -> Result<(), VerifyError> {
    let instances: BTreeSet<ValueId> = function
        .blocks
        .iter()
        .flat_map(|block| &block.instructions)
        .filter_map(
            |instruction| match (instruction.result, &instruction.operation) {
                (Some(result), Operation::NewInstance { class })
                    if may_be_in_frame(module, *class) =>
                {
                    Some(result)
                }
                _ => None,
            },
        )
        .collect();
    for (index, info) in function.values.iter().enumerate() {
        let value = ValueId(u32::try_from(index).unwrap_or(u32::MAX));
        if info.representation == ValueRepresentation::Stack && !instances.contains(&value) {
            return Err(VerifyError {
                function: function.symbol.clone(),
                block: None,
                message: format!(
                    "v{} is held in the frame but is not a new instance of a class",
                    value.0
                ),
            });
        }
    }
    Ok(())
}
