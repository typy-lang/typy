//! One instruction: an operation, its result and where it came from.

use typy_diagnostics::Span;

use crate::operation::Operation;
use crate::value::ValueId;

/// One instruction of a basic block.
#[derive(Clone, Debug, PartialEq)]
pub struct Instruction {
    /// The value it defines, if it has a result.
    pub result: Option<ValueId>,
    /// What it does.
    pub operation: Operation,
    /// The source it came from; its line becomes a traceback entry (ADR-0004).
    pub span: Span,
}
