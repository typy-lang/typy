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
            Self::Return(_) | Self::ReturnError(_) | Self::Unreachable => Vec::new(),
        }
    }

    /// Returns the values the terminator reads.
    #[must_use]
    pub fn operands(&self) -> Vec<ValueId> {
        match self {
            Self::Jump { arguments, .. } | Self::Invoke { arguments, .. } => arguments.clone(),
            Self::Branch { condition, .. } => vec![*condition],
            Self::Return(value) | Self::ReturnError(value) => vec![*value],
            Self::Unreachable => Vec::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{Callee, Terminator};
    use crate::block::BlockId;
    use crate::runtime::RuntimeFunction;
    use crate::value::ValueId;

    #[test]
    fn successors_and_operands_follow_the_kind() {
        let invoke = Terminator::Invoke {
            callee: Callee::Runtime(RuntimeFunction::IntFloorDivide),
            arguments: vec![ValueId(1), ValueId(2)],
            normal: BlockId(3),
            error: BlockId(4),
            span: typy_diagnostics::Span::empty(
                typy_diagnostics::FileId(0),
                typy_diagnostics::TextOffset(0),
            ),
        };
        assert_eq!(invoke.successors(), [BlockId(3), BlockId(4)]);
        assert_eq!(invoke.operands(), [ValueId(1), ValueId(2)]);
        let branch = Terminator::Branch {
            condition: ValueId(0),
            then: BlockId(1),
            otherwise: BlockId(2),
        };
        assert_eq!(branch.successors(), [BlockId(1), BlockId(2)]);
        assert_eq!(branch.operands(), [ValueId(0)]);
        let jump = Terminator::Jump {
            target: BlockId(5),
            arguments: vec![],
        };
        assert_eq!(jump.successors(), [BlockId(5)]);
        assert_eq!(Terminator::Return(ValueId(7)).operands(), [ValueId(7)]);
        assert_eq!(Terminator::ReturnError(ValueId(8)).successors(), []);
        assert_eq!(Terminator::Unreachable.operands(), []);
    }
}
