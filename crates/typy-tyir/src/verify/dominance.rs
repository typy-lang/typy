//! Which blocks dominate which: every path from the entry to a block passes through its dominators.

use std::collections::BTreeSet;

use crate::block::BlockId;
use crate::function::TyirFunction;

/// Returns, for each block, the set of blocks that dominate it. A block no path reaches is dominated by every block.
pub(super) fn dominators(function: &TyirFunction) -> Vec<BTreeSet<BlockId>> {
    let all: BTreeSet<BlockId> = function.blocks.iter().map(|block| block.id).collect();
    let mut predecessors: Vec<Vec<BlockId>> = vec![Vec::new(); function.blocks.len()];
    for block in &function.blocks {
        for successor in block.terminator.successors() {
            if let Some(list) = predecessors.get_mut(successor.0 as usize) {
                list.push(block.id);
            }
        }
    }
    let mut result: Vec<BTreeSet<BlockId>> = vec![all.clone(); function.blocks.len()];
    if let Some(entry) = result.first_mut() {
        *entry = BTreeSet::from([BlockId(0)]);
    }
    let mut changed = true;
    while changed {
        changed = false;
        for block in function.blocks.iter().skip(1) {
            let mut new: Option<BTreeSet<BlockId>> = None;
            for predecessor in &predecessors[block.id.0 as usize] {
                let set = &result[predecessor.0 as usize];
                new = Some(match new {
                    None => set.clone(),
                    Some(current) => current.intersection(set).copied().collect(),
                });
            }
            let mut new = new.unwrap_or_else(|| all.clone());
            new.insert(block.id);
            if new != result[block.id.0 as usize] {
                result[block.id.0 as usize] = new;
                changed = true;
            }
        }
    }
    result
}
