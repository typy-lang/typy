//! A function in TyIR.

use crate::block::{BasicBlock, BlockId};
use crate::slot::{Slot, SlotId};
use crate::types::TyirType;
use crate::value::{ValueId, ValueInfo};

/// Index of a function in its module.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct FunctionId(pub u32);

/// A function: its blocks, values and slots.
#[derive(Clone, Debug, PartialEq)]
pub struct TyirFunction {
    /// The function's ID.
    pub id: FunctionId,
    /// The symbol it is emitted under (detailed design section 10.3).
    pub symbol: String,
    /// The name tracebacks show, such as `fib` or `<module>`.
    pub name: String,
    /// The parameters: the entry block's parameters.
    pub parameters: Vec<ValueId>,
    /// The return type.
    pub return_type: TyirType,
    /// True when the function may raise, so its callers use `invoke`.
    pub may_raise: bool,
    /// The blocks; the first is the entry.
    pub blocks: Vec<BasicBlock>,
    /// Every value, indexed by `ValueId`.
    pub values: Vec<ValueInfo>,
    /// Every slot, indexed by `SlotId`.
    pub slots: Vec<Slot>,
}

impl TyirFunction {
    /// Returns a block by its ID.
    #[must_use]
    pub fn block(&self, id: BlockId) -> &BasicBlock {
        &self.blocks[id.0 as usize]
    }

    /// Returns what is known about a value.
    #[must_use]
    pub fn value(&self, id: ValueId) -> ValueInfo {
        self.values[id.0 as usize]
    }

    /// Returns a slot by its ID.
    #[must_use]
    pub fn slot(&self, id: SlotId) -> &Slot {
        &self.slots[id.0 as usize]
    }
}
