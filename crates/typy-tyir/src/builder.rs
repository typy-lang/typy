//! Builds a `TyirFunction` one block at a time.

use typy_diagnostics::Span;

use crate::block::{BasicBlock, BlockId};
use crate::function::{FunctionId, TyirFunction};
use crate::instruction::Instruction;
use crate::operation::Operation;
use crate::slot::{Slot, SlotId};
use crate::terminator::Terminator;
use crate::types::TyirType;
use crate::value::{ValueId, ValueInfo};

struct PartialBlock {
    parameters: Vec<ValueId>,
    instructions: Vec<Instruction>,
    terminator: Option<Terminator>,
}

/// Builds one function. Code added after a block is terminated goes to a new block with no predecessors, as dead code after `return` does.
pub struct FunctionBuilder {
    id: FunctionId,
    symbol: String,
    name: String,
    return_type: TyirType,
    may_raise: bool,
    parameters: Vec<ValueId>,
    blocks: Vec<PartialBlock>,
    values: Vec<ValueInfo>,
    slots: Vec<Slot>,
    current: BlockId,
}

impl FunctionBuilder {
    /// Starts a function with an empty entry block.
    #[must_use]
    pub fn new(
        id: FunctionId,
        symbol: impl Into<String>,
        name: impl Into<String>,
        return_type: TyirType,
    ) -> Self {
        Self {
            id,
            symbol: symbol.into(),
            name: name.into(),
            return_type,
            may_raise: false,
            parameters: Vec::new(),
            blocks: vec![PartialBlock {
                parameters: Vec::new(),
                instructions: Vec::new(),
                terminator: None,
            }],
            values: Vec::new(),
            slots: Vec::new(),
            current: BlockId(0),
        }
    }

    fn new_value(&mut self, ty: TyirType) -> ValueId {
        let value = ValueId(index(self.values.len()));
        self.values.push(ValueInfo::of(ty));
        value
    }

    /// Adds a parameter to the function, as a parameter of the entry block.
    pub fn add_parameter(&mut self, ty: TyirType) -> ValueId {
        let value = self.new_value(ty);
        self.blocks[0].parameters.push(value);
        self.parameters.push(value);
        value
    }

    /// Adds a slot for a local variable.
    pub fn add_slot(&mut self, name: impl Into<String>, ty: TyirType) -> SlotId {
        let slot = SlotId(index(self.slots.len()));
        self.slots.push(Slot {
            name: name.into(),
            ty,
        });
        slot
    }

    /// Returns the type the function returns.
    #[must_use]
    pub fn return_type(&self) -> TyirType {
        self.return_type
    }

    /// Returns the type of a slot.
    #[must_use]
    pub fn slot_type(&self, slot: SlotId) -> TyirType {
        self.slots[slot.0 as usize].ty
    }

    /// Returns the number of slots added so far.
    #[must_use]
    pub fn slot_count(&self) -> usize {
        self.slots.len()
    }

    /// Creates an empty block; code goes into it after `switch_to`.
    pub fn create_block(&mut self) -> BlockId {
        let block = BlockId(index(self.blocks.len()));
        self.blocks.push(PartialBlock {
            parameters: Vec::new(),
            instructions: Vec::new(),
            terminator: None,
        });
        block
    }

    /// Adds a parameter to a block.
    pub fn add_block_parameter(&mut self, block: BlockId, ty: TyirType) -> ValueId {
        let value = self.new_value(ty);
        self.blocks[block.0 as usize].parameters.push(value);
        value
    }

    /// Makes `block` the block that code is added to.
    pub fn switch_to(&mut self, block: BlockId) {
        self.current = block;
    }

    /// Returns the block code is added to.
    #[must_use]
    pub fn current_block(&self) -> BlockId {
        self.current
    }

    /// Returns true when the current block already has its terminator.
    #[must_use]
    pub fn is_terminated(&self) -> bool {
        self.blocks[self.current.0 as usize].terminator.is_some()
    }

    fn open_block(&mut self) {
        if self.is_terminated() {
            let unreachable = self.create_block();
            self.switch_to(unreachable);
        }
    }

    /// Adds an instruction with a result of type `ty`, and returns the result.
    pub fn push_value(&mut self, operation: Operation, ty: TyirType, span: Span) -> ValueId {
        self.open_block();
        let result = self.new_value(ty);
        let instruction = Instruction {
            result: Some(result),
            operation,
            span,
        };
        self.blocks[self.current.0 as usize]
            .instructions
            .push(instruction);
        result
    }

    /// Adds an instruction without a result.
    pub fn push(&mut self, operation: Operation, span: Span) {
        self.open_block();
        let instruction = Instruction {
            result: None,
            operation,
            span,
        };
        self.blocks[self.current.0 as usize]
            .instructions
            .push(instruction);
    }

    /// Ends the current block.
    pub fn terminate(&mut self, terminator: Terminator) {
        self.open_block();
        self.blocks[self.current.0 as usize].terminator = Some(terminator);
    }

    /// Marks the function as one that may raise (ADR-0004).
    pub fn set_may_raise(&mut self, may_raise: bool) {
        self.may_raise = may_raise;
    }

    /// Returns the finished function; a block left without a terminator ends in `unreachable`.
    #[must_use]
    pub fn finish(self) -> TyirFunction {
        let blocks = self
            .blocks
            .into_iter()
            .enumerate()
            .map(|(position, block)| BasicBlock {
                id: BlockId(index(position)),
                parameters: block.parameters,
                instructions: block.instructions,
                terminator: block.terminator.unwrap_or(Terminator::Unreachable),
            })
            .collect();
        TyirFunction {
            id: self.id,
            symbol: self.symbol,
            name: self.name,
            parameters: self.parameters,
            return_type: self.return_type,
            may_raise: self.may_raise,
            blocks,
            values: self.values,
            slots: self.slots,
        }
    }
}

fn index(position: usize) -> u32 {
    u32::try_from(position).unwrap_or(u32::MAX)
}

#[cfg(test)]
mod tests {
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
}
