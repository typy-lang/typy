//! Writes TyIR as deterministic text, for `--emit=tyir` and `--emit=tyir-mem` (detailed design section 8.2).

use std::fmt::Write as _;

use crate::block::BasicBlock;
use crate::function::TyirFunction;
use crate::module::TyirModule;
use crate::operation::{IntUnaryOperator, Operation};
use crate::terminator::{Callee, Terminator};
use crate::value::ValueId;

/// Returns the module as text, the string pool and then each function in order, with every
/// reference count operation, for `--emit=tyir-mem`.
#[must_use]
pub fn print(module: &TyirModule) -> String {
    print_with(module, true)
}

/// Returns the module as text without its `inc` and `dec` instructions, for `--emit=tyir`.
#[must_use]
pub fn print_without_counting(module: &TyirModule) -> String {
    print_with(module, false)
}

fn print_with(module: &TyirModule, counting: bool) -> String {
    let mut text = String::new();
    for (index, string) in module.strings.iter().enumerate() {
        let _ = writeln!(text, "string s{index} = {string:?}");
    }
    let _ = writeln!(text, "entry fn{}", module.entry.0);
    for function in &module.functions {
        text.push('\n');
        print_function(module, function, counting, &mut text);
    }
    text
}

fn value_list(function: &TyirFunction, values: &[ValueId]) -> String {
    values
        .iter()
        .map(|value| format!("v{}: {}", value.0, function.value(*value).ty))
        .collect::<Vec<_>>()
        .join(", ")
}

fn names(values: &[ValueId]) -> String {
    values
        .iter()
        .map(|value| format!("v{}", value.0))
        .collect::<Vec<_>>()
        .join(", ")
}

fn callee(module: &TyirModule, callee: Callee) -> String {
    match callee {
        Callee::Runtime(function) => format!("rt.{}", function.name()),
        Callee::Function(id) => module
            .functions
            .get(id.0 as usize)
            .map_or_else(|| format!("fn{}", id.0), |function| function.symbol.clone()),
    }
}

fn print_function(module: &TyirModule, function: &TyirFunction, counting: bool, text: &mut String) {
    let raises = if function.may_raise { " raises" } else { "" };
    let _ = writeln!(
        text,
        "fn {}({}) -> {}{raises} {{  # {}",
        function.symbol,
        value_list(function, &function.parameters),
        function.return_type,
        function.name
    );
    for (index, slot) in function.slots.iter().enumerate() {
        let _ = writeln!(text, "  slot s{index} {}: {}", slot.name, slot.ty);
    }
    for block in &function.blocks {
        print_block(module, function, block, counting, text);
    }
    text.push_str("}\n");
}

fn print_block(
    module: &TyirModule,
    function: &TyirFunction,
    block: &BasicBlock,
    counting: bool,
    text: &mut String,
) {
    let _ = writeln!(
        text,
        "b{}({}):",
        block.id.0,
        value_list(function, &block.parameters)
    );
    for instruction in &block.instructions {
        let counts = matches!(
            instruction.operation,
            Operation::Increment(_) | Operation::Decrement(_)
        );
        if counts && !counting {
            continue;
        }
        let operation = operation(module, &instruction.operation);
        match instruction.result {
            Some(value) => {
                let _ = writeln!(text, "  v{} = {operation}", value.0);
            }
            None => {
                let _ = writeln!(text, "  {operation}");
            }
        }
    }
    let terminator = match &block.terminator {
        Terminator::Jump { target, arguments } => {
            format!("jump b{}({})", target.0, names(arguments))
        }
        Terminator::Branch {
            condition,
            then,
            otherwise,
        } => {
            format!("branch v{}, b{}, b{}", condition.0, then.0, otherwise.0)
        }
        Terminator::Invoke {
            callee: target,
            arguments,
            normal,
            error,
            ..
        } => format!(
            "invoke {}({}) to b{} unwind b{}",
            callee(module, *target),
            names(arguments),
            normal.0,
            error.0
        ),
        Terminator::Return(value) => format!("return v{}", value.0),
        Terminator::ReturnError(value) => format!("return_error v{}", value.0),
        Terminator::Unreachable => "unreachable".to_owned(),
    };
    let _ = writeln!(text, "  {terminator}");
}

fn operation(module: &TyirModule, operation: &Operation) -> String {
    match operation {
        Operation::Constant(constant) => format!("const {constant}"),
        Operation::Load(slot) => format!("load s{}", slot.0),
        Operation::Store { slot, value } => format!("store s{}, v{}", slot.0, value.0),
        Operation::IntBinary {
            operator,
            left,
            right,
        } => format!("int.{operator} v{}, v{}", left.0, right.0),
        Operation::IntUnary { operator, operand } => {
            let name = match operator {
                IntUnaryOperator::Negate => "neg",
                IntUnaryOperator::Invert => "invert",
            };
            format!("int.{name} v{}", operand.0)
        }
        Operation::FloatBinary {
            operator,
            left,
            right,
        } => format!("float.{operator} v{}, v{}", left.0, right.0),
        Operation::FloatNegate(operand) => format!("float.neg v{}", operand.0),
        Operation::BoolBinary {
            operator,
            left,
            right,
        } => format!("bool.{operator} v{}, v{}", left.0, right.0),
        Operation::BoolNot(operand) => format!("bool.not v{}", operand.0),
        Operation::Compare {
            kind,
            comparison,
            left,
            right,
        } => format!("cmp.{kind}.{comparison} v{}, v{}", left.0, right.0),
        Operation::IsTrue { kind, value } => format!("is_true.{kind} v{}", value.0),
        Operation::CallRuntime {
            function,
            arguments,
        } => format!("call rt.{}({})", function.name(), names(arguments)),
        Operation::Call {
            function,
            arguments,
        } => format!(
            "call {}({})",
            callee(module, Callee::Function(*function)),
            names(arguments)
        ),
        Operation::MakeTuple { ty, items } => format!("tuple#{} ({})", ty.0, names(items)),
        Operation::TupleItem { tuple, index } => format!("item v{}, {index}", tuple.0),
        Operation::ToBits(value) => format!("to_bits v{}", value.0),
        Operation::Wrap { value, ty } => format!("wrap v{}, {ty}", value.0),
        Operation::Unwrap(value) => format!("unwrap v{}", value.0),
        Operation::IsNone(value) => format!("is_none v{}", value.0),
        Operation::FromBits { bits, ty } => format!("from_bits v{}, {ty}", bits.0),
        Operation::NewInstance { class } => format!("new class#{class}"),
        Operation::Field { object, index } => format!("field v{}, {index}", object.0),
        Operation::SetField {
            object,
            index,
            value,
        } => format!("set_field v{}, {index}, v{}", object.0, value.0),
        Operation::Identical { left, right } => format!("identical v{}, v{}", left.0, right.0),
        Operation::Increment(value) => format!("inc v{}", value.0),
        Operation::Decrement(value) => format!("dec v{}", value.0),
    }
}
