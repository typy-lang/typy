//! Tests of `terminator`.

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
