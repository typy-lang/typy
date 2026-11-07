//! Basic blocks: straight runs of instructions ending in one terminator.

use crate::instruction::Instruction;
use crate::terminator::Terminator;
use crate::value::ValueId;

/// Index of a block in its function.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct BlockId(pub u32);

/// A basic block.
#[derive(Clone, Debug, PartialEq)]
pub struct BasicBlock {
    /// The block's ID.
    pub id: BlockId,
    /// Values the block receives from its predecessors.
    pub parameters: Vec<ValueId>,
    /// The instructions, in order.
    pub instructions: Vec<Instruction>,
    /// How the block ends.
    pub terminator: Terminator,
}
