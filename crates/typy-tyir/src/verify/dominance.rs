//! Which blocks dominate which: every path from the entry to a block passes through its
//! dominators. The immediate dominators come from Cooper, Harvey and Kennedy's iteration over
//! the blocks in reverse postorder, and a numbering of the dominator tree in preorder and
//! postorder answers whether one block dominates another in constant time, so a function of
//! many blocks costs memory in proportion to them and not to their square (NFR-09).

use crate::block::BlockId;
use crate::function::TyirFunction;

/// The dominator tree of a function, numbered for constant-time queries.
pub(super) struct Dominance {
    /// The position of each block in a preorder walk of the dominator tree; `None` for a
    /// block no path from the entry reaches.
    preorder: Vec<Option<usize>>,
    /// The position of each block in a postorder walk of the dominator tree.
    postorder: Vec<usize>,
}

impl Dominance {
    /// Returns whether `dominator` dominates `block`. A block no path reaches is dominated by
    /// every block, and dominates no block that a path reaches.
    pub(super) fn dominates(&self, dominator: BlockId, block: BlockId) -> bool {
        let (dominator, block) = (dominator.0 as usize, block.0 as usize);
        let Some(Some(inner)) = self.preorder.get(block).copied() else {
            return true;
        };
        let Some(Some(outer)) = self.preorder.get(dominator).copied() else {
            return false;
        };
        outer <= inner && self.postorder[block] <= self.postorder[dominator]
    }
}

/// Returns the blocks that a path from the entry reaches, in reverse postorder.
fn reverse_postorder(successors: &[Vec<usize>]) -> Vec<usize> {
    let mut visited = vec![false; successors.len()];
    let mut order = Vec::with_capacity(successors.len());
    if successors.is_empty() {
        return order;
    }
    // Each entry of the stack is a block and how many of its successors are done.
    let mut stack = vec![(0_usize, 0_usize)];
    visited[0] = true;
    while let Some((block, next)) = stack.last_mut() {
        if let Some(&successor) = successors[*block].get(*next) {
            *next += 1;
            if !visited[successor] {
                visited[successor] = true;
                stack.push((successor, 0));
            }
        } else {
            order.push(*block);
            stack.pop();
        }
    }
    order.reverse();
    order
}

/// Returns the dominator tree of `function`.
pub(super) fn dominance(function: &TyirFunction) -> Dominance {
    let count = function.blocks.len();
    let mut successors: Vec<Vec<usize>> = vec![Vec::new(); count];
    let mut predecessors: Vec<Vec<usize>> = vec![Vec::new(); count];
    for block in &function.blocks {
        for successor in block.terminator.successors() {
            let (from, to) = (block.id.0 as usize, successor.0 as usize);
            if to < count {
                successors[from].push(to);
                predecessors[to].push(from);
            }
        }
    }
    let order = reverse_postorder(&successors);
    let mut rank = vec![usize::MAX; count];
    for (position, block) in order.iter().enumerate() {
        rank[*block] = position;
    }
    let mut immediate: Vec<Option<usize>> = vec![None; count];
    if let Some(&entry) = order.first() {
        immediate[entry] = Some(entry);
    }
    let intersect = |immediate: &[Option<usize>], mut left: usize, mut right: usize| {
        while left != right {
            while rank[left] > rank[right] {
                left = immediate[left].unwrap_or(left);
            }
            while rank[right] > rank[left] {
                right = immediate[right].unwrap_or(right);
            }
        }
        left
    };
    let mut changed = true;
    while changed {
        changed = false;
        for &block in order.iter().skip(1) {
            let mut new = None;
            for &predecessor in &predecessors[block] {
                if immediate[predecessor].is_none() {
                    continue;
                }
                new = Some(match new {
                    None => predecessor,
                    Some(current) => intersect(&immediate, predecessor, current),
                });
            }
            if new.is_some() && immediate[block] != new {
                immediate[block] = new;
                changed = true;
            }
        }
    }
    number(&order, &immediate, count)
}

/// Numbers the dominator tree given by `immediate` in preorder and postorder, from the entry.
fn number(order: &[usize], immediate: &[Option<usize>], count: usize) -> Dominance {
    let mut children: Vec<Vec<usize>> = vec![Vec::new(); count];
    for &block in order.iter().skip(1) {
        if let Some(parent) = immediate[block] {
            children[parent].push(block);
        }
    }
    let mut preorder = vec![None; count];
    let mut postorder = vec![0; count];
    let (mut before, mut after) = (0, 0);
    let mut stack: Vec<(usize, usize)> =
        order.first().map(|&entry| (entry, 0)).into_iter().collect();
    if let Some(&entry) = order.first() {
        preorder[entry] = Some(before);
        before += 1;
    }
    while let Some((block, next)) = stack.last_mut() {
        if let Some(&child) = children[*block].get(*next) {
            *next += 1;
            preorder[child] = Some(before);
            before += 1;
            stack.push((child, 0));
        } else {
            postorder[*block] = after;
            after += 1;
            stack.pop();
        }
    }
    Dominance {
        preorder,
        postorder,
    }
}
