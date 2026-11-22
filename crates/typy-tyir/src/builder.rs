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
    qualified_name: Option<String>,
    return_type: TyirType,
    may_raise: bool,
    parameters: Vec<ValueId>,
    blocks: Vec<PartialBlock>,
    values: Vec<ValueInfo>,
    slots: Vec<Slot>,
    current: BlockId,
    generator: Option<crate::function::GeneratorFrame>,
    stands_for: crate::function::StandsFor,
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
            qualified_name: None,
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
            generator: None,
            stands_for: crate::function::StandsFor::Itself,
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

    /// Returns the type of a value.
    #[must_use]
    pub fn value_type(&self, value: ValueId) -> TyirType {
        self.values[value.0 as usize].ty
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

    /// Sets the function's `__qualname__`, which is its name until set.
    pub fn set_qualified_name(&mut self, qualified_name: impl Into<String>) {
        self.qualified_name = Some(qualified_name.into());
    }

    /// Makes the function a generator's resume function, whose frame lives in its object
    /// (ADR-0063).
    pub fn set_generator(&mut self, frame: crate::function::GeneratorFrame) {
        self.generator = Some(frame);
    }

    /// Sets what the function stands for as a value (FUN-08).
    pub fn set_stands_for(&mut self, stands_for: crate::function::StandsFor) {
        self.stands_for = stands_for;
    }

    /// Returns the types of the slots made so far, in order.
    #[must_use]
    pub fn slot_types(&self) -> Vec<TyirType> {
        self.slots.iter().map(|slot| slot.ty).collect()
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
            qualified_name: self.qualified_name.unwrap_or_else(|| self.name.clone()),
            name: self.name,
            parameters: self.parameters,
            return_type: self.return_type,
            may_raise: self.may_raise,
            blocks,
            values: self.values,
            slots: self.slots,
            generator: self.generator,
            stands_for: self.stands_for,
        }
    }
}

fn index(position: usize) -> u32 {
    u32::try_from(position).unwrap_or(u32::MAX)
}

#[cfg(test)]
mod tests;
