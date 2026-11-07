//! Tests of the verifier: one valid function, and one broken rule per test.

use super::verify;
use crate::block::BlockId;
use crate::builder::FunctionBuilder;
use crate::constant::Constant;
use crate::function::{FunctionId, TyirFunction};
use crate::module::TyirModule;
use crate::operation::{Comparison, IntOperator, Operation, ScalarKind};
use crate::print::print;
use crate::runtime::RuntimeFunction;
use crate::terminator::{Callee, Terminator};
use crate::types::TyirType;
use crate::value::ValueId;
use typy_diagnostics::{FileId, Span, TextOffset};

fn span() -> Span {
    Span::empty(FileId(0), TextOffset(0))
}

fn module(function: TyirFunction) -> TyirModule {
    TyirModule {
        functions: vec![function],
        strings: vec!["hi\n".to_owned()],
        entry: FunctionId(0),
    }
}

/// `def div(a: int, b: int) -> int: return a // b if b != 0 else 0`, with the division raising through an invoke.
fn division() -> TyirFunction {
    let mut builder = FunctionBuilder::new(FunctionId(0), "_TY8__main__3div", "div", TyirType::Int);
    builder.set_may_raise(true);
    let a = builder.add_parameter(TyirType::Int);
    let b = builder.add_parameter(TyirType::Int);
    let slot = builder.add_slot("q", TyirType::Int);
    let zero = builder.push_value(
        Operation::Constant(Constant::Int("0".to_owned())),
        TyirType::Int,
        span(),
    );
    let nonzero = builder.push_value(
        Operation::Compare {
            kind: ScalarKind::Int,
            comparison: Comparison::NotEqual,
            left: b,
            right: zero,
        },
        TyirType::Bool,
        span(),
    );
    let divide = builder.create_block();
    let done = builder.create_block();
    builder.terminate(Terminator::Branch {
        condition: nonzero,
        then: divide,
        otherwise: done,
    });
    builder.switch_to(divide);
    let normal = builder.create_block();
    let quotient = builder.add_block_parameter(normal, TyirType::Int);
    let failed = builder.create_block();
    let exception = builder.add_block_parameter(failed, TyirType::EXCEPTION);
    builder.terminate(Terminator::Invoke {
        callee: Callee::Runtime(RuntimeFunction::IntFloorDivide),
        arguments: vec![a, b],
        normal,
        error: failed,
    });
    builder.switch_to(failed);
    builder.terminate(Terminator::ReturnError(exception));
    builder.switch_to(normal);
    builder.push(
        Operation::Store {
            slot,
            value: quotient,
        },
        span(),
    );
    builder.terminate(Terminator::Return(quotient));
    builder.switch_to(done);
    let sum = builder.push_value(
        Operation::IntBinary {
            operator: IntOperator::Add,
            left: a,
            right: zero,
        },
        TyirType::Int,
        span(),
    );
    builder.terminate(Terminator::Return(sum));
    builder.finish()
}

fn message(module: &TyirModule) -> String {
    verify(module).expect_err("the module is invalid").message
}

#[test]
fn a_valid_function_passes() {
    assert_eq!(verify(&module(division())), Ok(()));
}

#[test]
fn the_text_form_lists_strings_slots_blocks_and_terminators() {
    let text = print(&module(division()));
    for line in [
        "string s0 = \"hi\\n\"",
        "entry fn0",
        "fn _TY8__main__3div(v0: int, v1: int) -> int raises {  # div",
        "  slot s0 q: int",
        "  v2 = const int 0",
        "  v3 = cmp.int.ne v1, v2",
        "  branch v3, b1, b2",
        "  invoke rt.int_floor_divide(v0, v1) to b3 unwind b4",
        "b4(v5: exception):",
        "  return_error v5",
        "  store s0, v4",
        "  v6 = int.add v0, v2",
    ] {
        assert!(text.contains(line), "{line:?} missing from\n{text}");
    }
}

#[test]
fn a_value_used_before_its_definition_fails() {
    let mut function = division();
    function.blocks[0].instructions.swap(0, 1);
    assert!(message(&module(function)).contains("may not have run"));
}

#[test]
fn a_value_used_on_a_path_its_definition_is_not_on_fails() {
    let mut function = division();
    function.blocks[2].terminator = Terminator::Return(ValueId(4));
    assert!(message(&module(function)).contains("may not have run"));
}

#[test]
fn a_value_defined_twice_fails() {
    let mut function = division();
    function.blocks[0].instructions[1].result = Some(ValueId(2));
    assert!(message(&module(function)).contains("defined more than once"));
}

#[test]
fn a_value_that_is_never_defined_fails() {
    let mut function = division();
    function
        .values
        .push(crate::value::ValueInfo::of(TyirType::Int));
    function.blocks[2].terminator = Terminator::Return(ValueId(7));
    assert!(message(&module(function)).contains("never defined"));
}

#[test]
fn operands_of_the_wrong_type_fail() {
    let mut function = division();
    function.blocks[0].instructions[1].operation = Operation::Compare {
        kind: ScalarKind::Float,
        comparison: Comparison::Equal,
        left: ValueId(1),
        right: ValueId(2),
    };
    assert!(message(&module(function)).contains("is int, not float"));
}

#[test]
fn a_raising_call_without_invoke_fails() {
    let mut function = division();
    function.blocks[2].instructions[0].operation = Operation::CallRuntime {
        function: RuntimeFunction::IntFloorDivide,
        arguments: vec![ValueId(0), ValueId(1)],
    };
    assert!(message(&module(function)).contains("is called with invoke"));
}

#[test]
fn an_invoke_of_a_function_that_cannot_raise_fails() {
    let mut function = division();
    function.blocks[1].terminator = Terminator::Invoke {
        callee: Callee::Runtime(RuntimeFunction::StrLength),
        arguments: vec![ValueId(0)],
        normal: BlockId(3),
        error: BlockId(4),
    };
    assert!(message(&module(function)).contains("cannot raise"));
}

#[test]
fn an_exception_returned_by_a_function_that_cannot_raise_fails() {
    let mut function = division();
    function.may_raise = false;
    assert!(message(&module(function)).contains("cannot raise returns an exception"));
}

#[test]
fn branch_targets_with_parameters_and_wrong_arities_fail() {
    let mut function = division();
    function.blocks[0].terminator = Terminator::Branch {
        condition: ValueId(3),
        then: BlockId(3),
        otherwise: BlockId(2),
    };
    assert!(message(&module(function)).contains("branch target with parameters"));
    let mut function = division();
    function.blocks[2].terminator = Terminator::Jump {
        target: BlockId(3),
        arguments: vec![],
    };
    assert!(message(&module(function)).contains("takes 1 arguments, not 0"));
}

#[test]
fn counting_a_float_fails() {
    let mut function = division();
    let float = ValueId(u32::try_from(function.values.len()).unwrap());
    function
        .values
        .push(crate::value::ValueInfo::of(TyirType::Float));
    function.blocks[2].instructions.insert(
        0,
        crate::instruction::Instruction {
            result: Some(float),
            operation: Operation::Constant(Constant::Float(1.0)),
            span: span(),
        },
    );
    function.blocks[2].instructions.insert(
        1,
        crate::instruction::Instruction {
            result: None,
            operation: Operation::Increment(float),
            span: span(),
        },
    );
    assert!(message(&module(function)).contains("is not counted"));
}

#[test]
fn a_missing_entry_function_fails() {
    let mut broken = module(division());
    broken.entry = FunctionId(5);
    assert!(message(&broken).contains("entry function does not exist"));
}

#[test]
fn errors_name_the_function_and_block() {
    let mut function = division();
    function.blocks[0].instructions.swap(0, 1);
    let error = verify(&module(function)).unwrap_err();
    assert_eq!(
        error.to_string(),
        format!("_TY8__main__3div b0: {}", error.message)
    );
}

/// A function that uses every operation kind once, with the types each one takes.
#[allow(clippy::too_many_lines, reason = "one instruction per operation kind")]
fn every_operation() -> TyirModule {
    use crate::operation::{BoolOperator, FloatOperator, IntUnaryOperator};
    let mut callee =
        FunctionBuilder::new(FunctionId(1), "_TY8__main__4unit", "unit", TyirType::Unit);
    let unit = callee.push_value(Operation::Constant(Constant::Unit), TyirType::Unit, span());
    callee.terminate(Terminator::Return(unit));
    let mut builder =
        FunctionBuilder::new(FunctionId(0), "_TY8__main__3all", "all", TyirType::Unit);
    let slot = builder.add_slot("s", TyirType::STR);
    let integer = builder.push_value(
        Operation::Constant(Constant::Int("7".to_owned())),
        TyirType::Int,
        span(),
    );
    let float = builder.push_value(
        Operation::Constant(Constant::Float(2.5)),
        TyirType::Float,
        span(),
    );
    let truth = builder.push_value(
        Operation::Constant(Constant::Bool(false)),
        TyirType::Bool,
        span(),
    );
    let text = builder.push_value(
        Operation::Constant(Constant::Str(crate::constant::StringId(0))),
        TyirType::STR,
        span(),
    );
    builder.push(Operation::Store { slot, value: text }, span());
    let loaded = builder.push_value(Operation::Load(slot), TyirType::STR, span());
    for operator in [
        IntOperator::Subtract,
        IntOperator::Multiply,
        IntOperator::BitAnd,
        IntOperator::BitOr,
        IntOperator::BitXor,
    ] {
        builder.push_value(
            Operation::IntBinary {
                operator,
                left: integer,
                right: integer,
            },
            TyirType::Int,
            span(),
        );
    }
    builder.push_value(
        Operation::IntUnary {
            operator: IntUnaryOperator::Negate,
            operand: integer,
        },
        TyirType::Int,
        span(),
    );
    builder.push_value(
        Operation::IntUnary {
            operator: IntUnaryOperator::Invert,
            operand: integer,
        },
        TyirType::Int,
        span(),
    );
    for operator in [
        FloatOperator::Add,
        FloatOperator::Subtract,
        FloatOperator::Multiply,
    ] {
        builder.push_value(
            Operation::FloatBinary {
                operator,
                left: float,
                right: float,
            },
            TyirType::Float,
            span(),
        );
    }
    builder.push_value(Operation::FloatNegate(float), TyirType::Float, span());
    for operator in [BoolOperator::And, BoolOperator::Or, BoolOperator::Xor] {
        builder.push_value(
            Operation::BoolBinary {
                operator,
                left: truth,
                right: truth,
            },
            TyirType::Bool,
            span(),
        );
    }
    builder.push_value(Operation::BoolNot(truth), TyirType::Bool, span());
    for comparison in [
        Comparison::Equal,
        Comparison::Less,
        Comparison::LessEqual,
        Comparison::Greater,
        Comparison::GreaterEqual,
    ] {
        builder.push_value(
            Operation::Compare {
                kind: ScalarKind::Str,
                comparison,
                left: loaded,
                right: text,
            },
            TyirType::Bool,
            span(),
        );
    }
    for (kind, value) in [
        (ScalarKind::Int, integer),
        (ScalarKind::Float, float),
        (ScalarKind::Bool, truth),
        (ScalarKind::Str, text),
    ] {
        builder.push_value(Operation::IsTrue { kind, value }, TyirType::Bool, span());
    }
    builder.push_value(
        Operation::CallRuntime {
            function: RuntimeFunction::StrLength,
            arguments: vec![text],
        },
        TyirType::Int,
        span(),
    );
    builder.push(
        Operation::CallRuntime {
            function: RuntimeFunction::WriteStdout,
            arguments: vec![text],
        },
        span(),
    );
    builder.push(
        Operation::Call {
            function: FunctionId(1),
            arguments: vec![],
        },
        span(),
    );
    builder.push(Operation::Increment(text), span());
    builder.push(Operation::Decrement(integer), span());
    let next = builder.create_block();
    builder.terminate(Terminator::Jump {
        target: next,
        arguments: vec![],
    });
    builder.switch_to(next);
    let unit = builder.push_value(Operation::Constant(Constant::Unit), TyirType::Unit, span());
    builder.terminate(Terminator::Return(unit));
    builder.push_value(Operation::Constant(Constant::Unit), TyirType::Unit, span());
    TyirModule {
        functions: vec![builder.finish(), callee.finish()],
        strings: vec!["a".to_owned()],
        entry: FunctionId(0),
    }
}

#[test]
fn every_operation_kind_verifies_and_prints() {
    let module = every_operation();
    assert_eq!(verify(&module), Ok(()));
    assert_eq!(module.function(FunctionId(1)).name, "unit");
    assert_eq!(module.string(crate::constant::StringId(0)), "a");
    let text = print(&module);
    for line in [
        "const float 2.5",
        "const bool false",
        "const str s0",
        "load s0",
        "int.sub",
        "int.mul",
        "int.and",
        "int.or",
        "int.xor",
        "int.neg",
        "int.invert",
        "float.add",
        "float.sub",
        "float.mul",
        "float.neg",
        "bool.and",
        "bool.or",
        "bool.xor",
        "bool.not",
        "cmp.str.eq",
        "cmp.str.lt",
        "cmp.str.le",
        "cmp.str.gt",
        "cmp.str.ge",
        "is_true.int",
        "is_true.float",
        "is_true.bool",
        "is_true.str",
        "call rt.str_length(v3)",
        "call rt.write_stdout(v3)",
        "call _TY8__main__4unit()",
        "inc v3",
        "dec v0",
        "jump b1()",
        "  return v",
        "  unreachable",
    ] {
        assert!(text.contains(line), "{line:?} missing from\n{text}");
    }
}

fn first_block_instruction(
    module: &mut TyirModule,
    index: usize,
) -> &mut crate::instruction::Instruction {
    &mut module.functions[0].blocks[0].instructions[index]
}

/// A change that breaks one rule of a valid module.
type Breaker = Box<dyn Fn(&mut TyirModule)>;

#[test]
fn type_errors_in_operations_are_reported() {
    let cases: Vec<(Breaker, &str)> = vec![
        (
            Box::new(|module| {
                first_block_instruction(module, 5).operation =
                    Operation::Load(crate::slot::SlotId(9));
            }),
            "slot s9 does not exist",
        ),
        (
            Box::new(|module| {
                first_block_instruction(module, 4).operation = Operation::Store {
                    slot: crate::slot::SlotId(0),
                    value: ValueId(0),
                };
            }),
            "the stored value",
        ),
        (
            Box::new(|module| {
                first_block_instruction(module, 4).operation = Operation::Store {
                    slot: crate::slot::SlotId(7),
                    value: ValueId(3),
                };
            }),
            "slot s7 does not exist",
        ),
        (
            Box::new(|module| {
                first_block_instruction(module, 6).result = None;
            }),
            "result is dropped",
        ),
        (
            Box::new(|module| {
                first_block_instruction(module, 4).result = Some(ValueId(0));
            }),
            "defined more than once",
        ),
        (
            Box::new(|module| {
                first_block_instruction(module, 1).operation = Operation::FloatNegate(ValueId(0));
            }),
            "is int, not float",
        ),
        (
            Box::new(|module| {
                first_block_instruction(module, 2).operation = Operation::BoolNot(ValueId(1));
            }),
            "is float, not bool",
        ),
        (
            Box::new(|module| {
                first_block_instruction(module, 2).operation = Operation::IntUnary {
                    operator: crate::operation::IntUnaryOperator::Negate,
                    operand: ValueId(1),
                };
            }),
            "is float, not int",
        ),
        (
            Box::new(|module| {
                first_block_instruction(module, 1).operation = Operation::CallRuntime {
                    function: RuntimeFunction::StrLength,
                    arguments: vec![],
                };
            }),
            "takes 1 arguments, not 0",
        ),
        (
            Box::new(|module| {
                first_block_instruction(module, 1).operation = Operation::Call {
                    function: FunctionId(9),
                    arguments: vec![],
                };
            }),
            "fn9 does not exist",
        ),
    ];
    for (break_rule, expected) in cases {
        let mut module = every_operation();
        break_rule(&mut module);
        let found = message(&module);
        assert!(
            found.contains(expected),
            "expected {expected:?}, found {found:?}"
        );
    }
}

#[test]
fn terminator_type_errors_are_reported() {
    let mut broken = module(division());
    broken.functions[0].blocks[2].terminator = Terminator::Return(ValueId(3));
    assert!(message(&broken).contains("the returned value"));
    let mut broken = module(division());
    broken.functions[0].blocks[4].terminator = Terminator::ReturnError(ValueId(0));
    assert!(message(&broken).contains("the returned exception"));
    let mut broken = module(division());
    let wrong_normal = Terminator::Invoke {
        callee: Callee::Runtime(RuntimeFunction::IntFloorDivide),
        arguments: vec![ValueId(0), ValueId(1)],
        normal: BlockId(4),
        error: BlockId(4),
    };
    broken.functions[0].blocks[1].terminator = wrong_normal;
    assert!(message(&broken).contains("must take the int result"));
    let mut broken = module(division());
    broken.functions[0].blocks[1].terminator = Terminator::Invoke {
        callee: Callee::Runtime(RuntimeFunction::IntFloorDivide),
        arguments: vec![ValueId(0), ValueId(1)],
        normal: BlockId(3),
        error: BlockId(3),
    };
    assert!(message(&broken).contains("must take the exception"));
    let mut broken = module(division());
    broken.functions[0].blocks[1].terminator = Terminator::Jump {
        target: BlockId(9),
        arguments: vec![],
    };
    assert!(message(&broken).contains("b9 does not exist"));
}

#[test]
fn every_runtime_function_has_a_complete_signature() {
    for function in RuntimeFunction::ALL {
        let signature = function.signature();
        assert!(signature.parameters.len() <= 3, "{function:?}");
        assert_ne!(function.name(), "");
    }
}
