//! The instruction that ends a basic block.

use typy_diagnostics::Span;

use crate::block::BlockId;
use crate::function::FunctionId;
use crate::runtime::RuntimeFunction;
use crate::value::ValueId;

/// The callee of an `invoke`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Callee {
    /// A program function.
    Function(FunctionId),
    /// A runtime function.
    Runtime(RuntimeFunction),
    /// The code of the function object that is the first argument, which takes that object,
    /// then the other arguments, and returns its result beside an exception (FUN-05,
    /// ADR-0060). The other arguments and the parameter of the normal block are its
    /// parameters and result.
    Value,
    /// The method at `slot` of the method table of the class of the first argument, an
    /// instance of class `class` or of a class derived from it, whose table starts as
    /// `class`'s does (ADR-0021). Its parameters and result are those of `class`'s entry.
    Method {
        /// The class whose table gives the entry's parameters and result.
        class: u32,
        /// The position in the table.
        slot: u32,
    },
}

/// How a basic block ends.
#[derive(Clone, Debug, PartialEq)]
pub enum Terminator {
    /// Continues in `target`, passing `arguments` as its parameters.
    Jump {
        /// The next block.
        target: BlockId,
        /// Values for the target's parameters.
        arguments: Vec<ValueId>,
    },
    /// Continues in one of two blocks, which take no parameters.
    Branch {
        /// A `bool`.
        condition: ValueId,
        /// The block when true.
        then: BlockId,
        /// The block when false.
        otherwise: BlockId,
    },
    /// Calls a function that may raise (ADR-0004). The result is the one parameter of `normal`; the exception is the one parameter of `error`.
    Invoke {
        /// The function.
        callee: Callee,
        /// The arguments, borrowed.
        arguments: Vec<ValueId>,
        /// Where execution continues when nothing is raised.
        normal: BlockId,
        /// Where execution continues with the exception.
        error: BlockId,
        /// The call in the source; its line is added to the traceback when the call raises (ADR-0004).
        span: Span,
    },
    /// Returns a value, which the caller then owns.
    Return(ValueId),
    /// Returns an exception to the caller.
    ReturnError(ValueId),
    /// Marks a point that cannot be reached.
    Unreachable,
    /// Starts a step of a generator, the entry of its resume function: the first step
    /// continues in `start`, and a later one in the block after the `yield` it stopped at,
    /// each a successor of this block alone, so no value lives across a yield (ADR-0063).
    Resume {
        /// The generator object.
        generator: ValueId,
        /// The block of the first step.
        start: BlockId,
        /// The block after each `yield`, in the order of the yields.
        points: Vec<BlockId>,
    },
    /// Ends a step of a generator at a `yield`: the next step continues in `resume`, which
    /// the function's `Resume` lists (ADR-0063).
    Yield {
        /// The generator object.
        generator: ValueId,
        /// The block the next step continues in.
        resume: BlockId,
    },
}

impl Terminator {
    /// Returns the blocks this terminator can continue in.
    #[must_use]
    pub fn successors(&self) -> Vec<BlockId> {
        match self {
            Self::Jump { target, .. } => vec![*target],
            Self::Branch {
                then, otherwise, ..
            } => vec![*then, *otherwise],
            Self::Invoke { normal, error, .. } => vec![*normal, *error],
            Self::Resume { start, points, .. } => std::iter::once(*start)
                .chain(points.iter().copied())
                .collect(),
            // The next step enters its block through the function's `Resume`.
            Self::Return(_) | Self::ReturnError(_) | Self::Unreachable | Self::Yield { .. } => {
                Vec::new()
            }
        }
    }

    /// Returns the values the terminator reads.
    #[must_use]
    pub fn operands(&self) -> Vec<ValueId> {
        match self {
            Self::Jump { arguments, .. } | Self::Invoke { arguments, .. } => arguments.clone(),
            Self::Branch { condition, .. } => vec![*condition],
            Self::Return(value)
            | Self::ReturnError(value)
            | Self::Resume {
                generator: value, ..
            }
            | Self::Yield {
                generator: value, ..
            } => vec![*value],
            Self::Unreachable => Vec::new(),
        }
    }
}

#[cfg(test)]
mod tests;
