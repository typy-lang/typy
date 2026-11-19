//! The values and slots of one function that may hold the same object, in sets, and which
//! sets escape: a value of a set that escapes may outlive the function.

use crate::block::BasicBlock;
use crate::function::{FunctionId, TyirFunction};
use crate::operation::Operation;
use crate::runtime::RuntimeFunction;
use crate::terminator::{Callee, Terminator};
use crate::value::ValueId;

use super::Program;

/// The sets of one function, as a forest: each node is a value, then a slot.
pub(super) struct Aliases {
    parents: Vec<usize>,
    sizes: Vec<usize>,
    /// Whether the set a root names escapes; meaningful at roots only.
    escaped: Vec<bool>,
}

impl Aliases {
    /// Returns the sets of `function`, with those that escape marked.
    pub(super) fn of(program: &Program<'_>, function: &TyirFunction) -> Self {
        let nodes = function.values.len() + function.slots.len();
        let mut aliases = Self {
            parents: (0..nodes).collect(),
            sizes: vec![1; nodes],
            escaped: vec![false; nodes],
        };
        let mut escaping = Vec::new();
        for block in &function.blocks {
            for instruction in &block.instructions {
                aliases.operation(
                    program,
                    function,
                    instruction.result,
                    &instruction.operation,
                    &mut escaping,
                );
            }
            aliases.terminator(program, function, block, &mut escaping);
        }
        for value in escaping {
            let root = aliases.root(value.0 as usize);
            aliases.escaped[root] = true;
        }
        aliases
    }

    /// Returns whether `value` may outlive the function.
    pub(super) fn escapes(&self, value: ValueId) -> bool {
        self.escaped[self.root(value.0 as usize)]
    }

    fn root(&self, mut node: usize) -> usize {
        while self.parents[node] != node {
            node = self.parents[node];
        }
        node
    }

    /// Puts two nodes in one set, the smaller set under the larger, so a path stays short.
    fn join(&mut self, left: usize, right: usize) {
        let (left, right) = (self.root(left), self.root(right));
        if left == right {
            return;
        }
        let (small, large) = if self.sizes[left] < self.sizes[right] {
            (left, right)
        } else {
            (right, left)
        };
        self.parents[small] = large;
        self.sizes[large] += self.sizes[small];
    }

    fn join_values(&mut self, left: ValueId, right: ValueId) {
        self.join(left.0 as usize, right.0 as usize);
    }

    /// Notes what one operation does with the objects it is given.
    fn operation(
        &mut self,
        program: &Program<'_>,
        function: &TyirFunction,
        result: Option<ValueId>,
        operation: &Operation,
        escaping: &mut Vec<ValueId>,
    ) {
        match operation {
            Operation::Store { slot, value } => {
                self.join(value.0 as usize, function.values.len() + slot.0 as usize);
            }
            Operation::Load(slot) => {
                if let Some(result) = result {
                    self.join(result.0 as usize, function.values.len() + slot.0 as usize);
                }
            }
            Operation::Downcast { value, .. }
            | Operation::Upcast { value, .. }
            | Operation::Wrap { value, .. }
            | Operation::FromBits { bits: value, .. }
            | Operation::Unwrap(value)
            | Operation::ToBits(value) => {
                if let Some(result) = result {
                    self.join_values(result, *value);
                }
            }
            // A capture is an escape: the function object may outlive the function that
            // made it (MEM-03, ADR-0061).
            Operation::StoreGlobal { value, .. }
            | Operation::SetField { value, .. }
            | Operation::SetCapture { value, .. }
            | Operation::CellSet { value, .. } => {
                escaping.push(*value);
            }
            // C may keep what it is given; no instance converts to a C type, so this is a
            // rule for the analysis, not a case it meets.
            Operation::MakeTuple { items, .. }
            | Operation::CallForeign {
                arguments: items, ..
            } => escaping.extend(items),
            Operation::CallRuntime {
                function: runtime,
                arguments,
            } => runtime_call(*runtime, arguments, escaping),
            Operation::Call {
                function: callee,
                arguments,
            } => call(program, &[*callee], arguments, escaping),
            Operation::CallMethod {
                class,
                slot,
                arguments,
            } => call(
                program,
                &program.table_functions(*class, *slot),
                arguments,
                escaping,
            ),
            // These read their operands, count them or make a value of their own: none keeps
            // an object beyond the instruction.
            Operation::Constant(_)
            | Operation::LoadGlobal(_)
            | Operation::IntBinary { .. }
            | Operation::IntUnary { .. }
            | Operation::FloatBinary { .. }
            | Operation::FloatNegate(_)
            | Operation::BoolBinary { .. }
            | Operation::BoolNot(_)
            | Operation::Compare { .. }
            | Operation::IsTrue { .. }
            | Operation::SameMethod { .. }
            | Operation::TupleItem { .. }
            | Operation::IsNone(_)
            | Operation::NewInstance { .. }
            | Operation::Field { .. }
            | Operation::NewClosure { .. }
            | Operation::Capture { .. }
            | Operation::NewCell { .. }
            | Operation::CellGet { .. }
            | Operation::CellBound { .. }
            | Operation::CellClear { .. }
            | Operation::Identical { .. }
            | Operation::Increment(_)
            | Operation::Decrement(_) => {}
        }
    }

    /// Notes what a block's terminator does with the objects it is given.
    fn terminator(
        &mut self,
        program: &Program<'_>,
        function: &TyirFunction,
        block: &BasicBlock,
        escaping: &mut Vec<ValueId>,
    ) {
        match &block.terminator {
            Terminator::Jump { target, arguments } => {
                let parameters = &function.block(*target).parameters;
                for (argument, parameter) in arguments.iter().zip(parameters) {
                    self.join_values(*argument, *parameter);
                }
            }
            Terminator::Invoke {
                callee, arguments, ..
            } => match callee {
                Callee::Function(callee) => call(program, &[*callee], arguments, escaping),
                Callee::Runtime(runtime) => runtime_call(*runtime, arguments, escaping),
                // The code a function value runs is known only when the program runs, so each
                // argument may be kept by it (MEM-03).
                Callee::Value => escaping.extend(arguments),
                Callee::Method { class, slot } => call(
                    program,
                    &program.table_functions(*class, *slot),
                    arguments,
                    escaping,
                ),
            },
            Terminator::Return(value) | Terminator::ReturnError(value) => escaping.push(*value),
            Terminator::Branch { .. } | Terminator::Unreachable => {}
        }
    }
}

/// Notes the arguments of a call of any of `callees` that a parameter lets escape.
fn call(
    program: &Program<'_>,
    callees: &[FunctionId],
    arguments: &[ValueId],
    escaping: &mut Vec<ValueId>,
) {
    for (position, argument) in arguments.iter().enumerate() {
        if callees
            .iter()
            .any(|callee| program.parameter_escapes(*callee, position))
        {
            escaping.push(*argument);
        }
    }
}

/// Notes the arguments of a runtime function: each escapes but for a function that reads only
/// the type of its object.
fn runtime_call(runtime: RuntimeFunction, arguments: &[ValueId], escaping: &mut Vec<ValueId>) {
    if !matches!(
        runtime,
        RuntimeFunction::IsInstance
            | RuntimeFunction::ObjectHasTypeId
            | RuntimeFunction::ObjectType
    ) {
        escaping.extend(arguments);
    }
}
