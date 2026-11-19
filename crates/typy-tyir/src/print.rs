//! Writes TyIR as deterministic text, for `--emit=tyir` and `--emit=tyir-mem` (detailed design section 8.2).

use std::fmt::Write as _;

use crate::block::BasicBlock;
use crate::function::TyirFunction;
use crate::module::TyirModule;
use crate::operation::{IntUnaryOperator, Operation};
use crate::terminator::{Callee, Terminator};
use crate::value::{ValueId, ValueRepresentation};

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
    for (index, points) in module.str_points.iter().enumerate() {
        let written: Vec<String> = points.iter().map(|point| format!("{point:#x}")).collect();
        let _ = writeln!(text, "points p{index} = [{}]", written.join(", "));
    }
    for (index, global) in module.globals.iter().enumerate() {
        let _ = writeln!(text, "global g{index}: {} = {:?}", global.ty, global.name);
    }
    let _ = writeln!(text, "entry fn{}", module.entry.0);
    if let Some(library) = &module.library {
        match &library.version {
            Some(version) => {
                let _ = writeln!(text, "library {} version {version}", library.name);
            }
            None => {
                let _ = writeln!(text, "library {}", library.name);
            }
        }
        for export in &library.exports {
            let parameters: Vec<&str> = export
                .parameters
                .iter()
                .map(|parameter| parameter.c_name())
                .collect();
            let result = export.result.map_or("void", |result| result.c_name());
            let _ = writeln!(
                text,
                "export {}({}) -> {result} = fn{}",
                export.name,
                parameters.join(", "),
                export.function.0
            );
        }
    }
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
        Callee::Method { class, slot } => format!("class#{class}.method#{slot}"),
        Callee::Value => "value".to_owned(),
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
            Some(value) if function.value(value).representation == ValueRepresentation::Stack => {
                let _ = writeln!(text, "  v{} = {operation} in frame", value.0);
            }
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
        Terminator::Resume {
            generator,
            start,
            points,
        } => {
            let points: Vec<String> = points.iter().map(|point| format!("b{}", point.0)).collect();
            format!(
                "resume v{}, b{}, [{}]",
                generator.0,
                start.0,
                points.join(", ")
            )
        }
        Terminator::Yield { generator, resume } => {
            format!("yield v{}, b{}", generator.0, resume.0)
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

#[allow(clippy::too_many_lines, reason = "one arm per operation")]
fn operation(module: &TyirModule, operation: &Operation) -> String {
    match operation {
        Operation::Constant(constant) => format!("const {constant}"),
        Operation::Load(slot) => format!("load s{}", slot.0),
        Operation::Store { slot, value } => format!("store s{}, v{}", slot.0, value.0),
        Operation::LoadGlobal(global) => format!("load_global g{}", global.0),
        Operation::StoreGlobal { global, value } => {
            format!("store_global g{}, v{}", global.0, value.0)
        }
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
        Operation::CallForeign {
            function,
            arguments,
        } => format!(
            "call_foreign {}({})",
            module
                .foreign_functions
                .get(*function as usize)
                .map_or("?", |foreign| foreign.name.as_str()),
            names(arguments)
        ),
        Operation::Call {
            function,
            arguments,
        } => format!(
            "call {}({})",
            callee(module, Callee::Function(*function)),
            names(arguments)
        ),
        Operation::SameMethod {
            class,
            slot,
            left,
            right,
        } => format!(
            "same_method class#{class}.method#{slot}, v{}, v{}",
            left.0, right.0
        ),
        Operation::CallMethod {
            class,
            slot,
            arguments,
        } => format!(
            "call {}({})",
            callee(
                module,
                Callee::Method {
                    class: *class,
                    slot: *slot
                }
            ),
            names(arguments)
        ),
        Operation::MakeTuple { ty, items } => format!("tuple#{} ({})", ty.0, names(items)),
        Operation::TupleItem { tuple, index } => format!("item v{}, {index}", tuple.0),
        Operation::ToBits(value) => format!("to_bits v{}", value.0),
        Operation::Wrap { value, ty } => format!("wrap v{}, {ty}", value.0),
        Operation::Unwrap(value) => format!("unwrap v{}", value.0),
        Operation::Upcast { value, ty } => format!("upcast v{}, {ty}", value.0),
        Operation::Downcast { value, ty } => format!("downcast v{}, {ty}", value.0),
        Operation::IsNone(value) => format!("is_none v{}", value.0),
        Operation::FromBits { bits, ty } => format!("from_bits v{}, {ty}", bits.0),
        Operation::NewInstance { class } => format!("new class#{class}"),
        Operation::NewCell { layout } => format!("new_cell tuple#{}", layout.0),
        Operation::CellGet { cell, layout } => format!("cell_get v{}, tuple#{}", cell.0, layout.0),
        Operation::CellBound { cell, layout } => {
            format!("cell_bound v{}, tuple#{}", cell.0, layout.0)
        }
        Operation::CellSet {
            cell,
            layout,
            value,
        } => format!("cell_set v{}, tuple#{}, v{}", cell.0, layout.0, value.0),
        Operation::CellClear { cell, layout } => {
            format!("cell_clear v{}, tuple#{}", cell.0, layout.0)
        }
        Operation::NewClosure { function, captures } => {
            format!("new_closure fn{}, tuple#{}", function.0, captures.0)
        }
        Operation::NewGenerator { function } => format!("new_generator fn{}", function.0),
        Operation::SetGeneratorArgument {
            generator,
            function,
            index,
            value,
        } => format!(
            "set_generator_argument v{}, fn{}, {index}, v{}",
            generator.0, function.0, value.0
        ),
        Operation::GeneratorArgument {
            generator,
            function,
            index,
        } => format!(
            "generator_argument v{}, fn{}, {index}",
            generator.0, function.0
        ),
        Operation::GeneratorClosure {
            generator,
            function,
        } => format!("generator_closure v{}, fn{}", generator.0, function.0),
        Operation::SetGeneratorItem { generator, item } => {
            format!("set_generator_item v{}, v{}", generator.0, item.0)
        }
        Operation::Capture {
            closure,
            captures,
            index,
        } => format!("capture v{}, tuple#{}, {index}", closure.0, captures.0),
        Operation::SetCapture {
            closure,
            captures,
            index,
            value,
        } => format!(
            "set_capture v{}, tuple#{}, {index}, v{}",
            closure.0, captures.0, value.0
        ),
        Operation::Field {
            object,
            class,
            index,
        } => format!("field v{}{}, {index}", object.0, of_class(*class)),
        Operation::SetField {
            object,
            class,
            index,
            value,
        } => format!(
            "set_field v{}{}, {index}, v{}",
            object.0,
            of_class(*class),
            value.0
        ),
        Operation::Identical { left, right } => format!("identical v{}, v{}", left.0, right.0),
        Operation::Increment(value) => format!("inc v{}", value.0),
        Operation::Decrement(value) => format!("dec v{}", value.0),
    }
}

/// Returns ` of class#n` for the class of an exception's attribute, or nothing.
fn of_class(class: Option<u32>) -> String {
    class.map_or_else(String::new, |class| format!(" of class#{class}"))
}
