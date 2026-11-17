//! Tests of escape analysis: which instances a function may hold in its frame.

use typy_diagnostics::{FileId, Span, TextOffset};

use super::place_instances;
use crate::builder::FunctionBuilder;
use crate::constant::Constant;
use crate::function::{FunctionId, TyirFunction};
use crate::module::{GlobalId, TyirClass, TyirGlobal, TyirModule};
use crate::operation::Operation;
use crate::runtime::RuntimeFunction;
use crate::slot::SlotId;
use crate::terminator::{Callee, Terminator};
use crate::tuple_types::TupleTypes;
use crate::types::{TyirType, TypeId, TypeObjectRef};
use crate::value::{ValueId, ValueRepresentation};

fn span() -> Span {
    Span::empty(FileId(0), TextOffset(0))
}

const POINT: TyirType = TyirType::Object(TypeId::class(0));

fn class(name: &str, fields: crate::types::TupleTypeId) -> TyirClass {
    TyirClass {
        name: name.to_owned(),
        module: "__main__".to_owned(),
        fields,
        methods: Vec::new(),
        repr: None,
        str: None,
        equal: None,
        order: [None; 4],
        hash: None,
        truth: None,
        base: None,
        exception_base: None,
        os_error: false,
        public: None,
    }
}

/// A module of `functions`, with class 0 `Point`, class 1 `Error`, an exception, class 2
/// `Shape` and class 3 `Square`, derived from `Shape`, and one global of class `Point`.
/// Function 1 is `Shape`'s method and function 2 `Square`'s; one that does nothing stands
/// for each the caller leaves out.
fn module(mut functions: Vec<TyirFunction>) -> TyirModule {
    while functions.len() < 3 {
        let id = u32::try_from(functions.len()).unwrap_or(u32::MAX);
        functions.push(keeps_nothing(id));
    }
    let mut tuple_types = TupleTypes::default();
    let TyirType::Tuple(fields) = tuple_types.intern(vec![TyirType::Int]) else {
        unreachable!("interning gives a tuple type");
    };
    let mut error = class("Error", fields);
    error.exception_base = Some(TypeObjectRef::Exception("Exception"));
    let mut shape = class("Shape", fields);
    shape.methods = vec![FunctionId(1)];
    let mut square = class("Square", fields);
    square.base = Some(2);
    square.methods = vec![FunctionId(2)];
    TyirModule {
        globals: vec![TyirGlobal {
            name: "kept".to_owned(),
            ty: POINT,
        }],
        modules: Vec::new(),
        type_aliases: Vec::new(),
        functions,
        strings: Vec::new(),
        str_points: Vec::new(),
        tuple_types,
        classes: vec![class("Point", fields), error, shape, square],
        entry: FunctionId(0),
    }
}

/// A function of one parameter of `ty` that runs `body` on it and returns unit.
fn taking(id: u32, ty: TyirType, body: impl FnOnce(&mut FunctionBuilder, ValueId)) -> TyirFunction {
    let mut builder = FunctionBuilder::new(
        FunctionId(id),
        format!("f{id}"),
        format!("f{id}"),
        TyirType::Unit,
    );
    let parameter = builder.add_parameter(ty);
    body(&mut builder, parameter);
    let unit = builder.push_value(Operation::Constant(Constant::Unit), TyirType::Unit, span());
    builder.terminate(Terminator::Return(unit));
    builder.finish()
}

/// A function that does nothing with its parameter.
fn keeps_nothing(id: u32) -> TyirFunction {
    taking(id, POINT, |_, _| {})
}

/// A function that stores its parameter in the global.
fn keeps_its_parameter(id: u32) -> TyirFunction {
    taking(id, POINT, |builder, parameter| {
        builder.push(
            Operation::StoreGlobal {
                global: GlobalId(0),
                value: parameter,
            },
            span(),
        );
    })
}

fn new_instance(builder: &mut FunctionBuilder, class: u32) -> ValueId {
    builder.push_value(
        Operation::NewInstance { class },
        TyirType::Object(TypeId::class(class)),
        span(),
    )
}

/// Returns whether each new instance of the first function is held in its frame, in order.
/// The functions are not verified: the analysis reads only what each operation does with its
/// operands, not their types.
fn placed(functions: Vec<TyirFunction>) -> Vec<bool> {
    let mut module = module(functions);
    place_instances(&mut module);
    let function = &module.functions[0];
    function
        .blocks
        .iter()
        .flat_map(|block| &block.instructions)
        .filter(|instruction| matches!(instruction.operation, Operation::NewInstance { .. }))
        .filter_map(|instruction| instruction.result)
        .map(|value| function.value(value).representation == ValueRepresentation::Stack)
        .collect()
}

/// Returns whether the one new `Point` of a function that does `uses` with it is held in its
/// frame, beside the functions 1 to 3 of `callees`.
fn point_used(
    uses: impl FnOnce(&mut FunctionBuilder, ValueId),
    callees: Vec<TyirFunction>,
) -> bool {
    let main = taking(0, TyirType::Int, |builder, _| {
        let point = new_instance(builder, 0);
        uses(builder, point);
    });
    placed([vec![main], callees].concat())[0]
}

fn store_global(builder: &mut FunctionBuilder, value: ValueId) {
    builder.push(
        Operation::StoreGlobal {
            global: GlobalId(0),
            value,
        },
        span(),
    );
}

#[test]
fn an_instance_only_read_and_counted_is_held_in_the_frame() {
    assert!(point_used(
        |builder, point| {
            let number = builder.push_value(
                Operation::Field {
                    object: point,
                    class: None,
                    index: 0,
                },
                TyirType::Int,
                span(),
            );
            builder.push(
                Operation::SetField {
                    object: point,
                    class: None,
                    index: 0,
                    value: number,
                },
                span(),
            );
            builder.push(Operation::Increment(point), span());
            builder.push(Operation::Decrement(point), span());
            builder.push_value(
                Operation::Identical {
                    left: point,
                    right: point,
                },
                TyirType::Bool,
                span(),
            );
            builder.push_value(
                Operation::CallRuntime {
                    function: RuntimeFunction::ObjectType,
                    arguments: vec![point],
                },
                TyirType::Object(TypeId::TYPE),
                span(),
            );
        },
        Vec::new(),
    ));
}

#[test]
fn an_instance_kept_beyond_its_function_comes_from_the_heap() {
    let kept: [fn(&mut FunctionBuilder, ValueId); 4] = [
        store_global,
        |builder, point| {
            let other = new_instance(builder, 0);
            builder.push(
                Operation::SetField {
                    object: other,
                    class: None,
                    index: 0,
                    value: point,
                },
                span(),
            );
        },
        |builder, point| {
            let mut types = TupleTypes::default();
            let TyirType::Tuple(ty) = types.intern(vec![POINT]) else {
                unreachable!("interning gives a tuple type");
            };
            builder.push_value(
                Operation::MakeTuple {
                    ty,
                    items: vec![point],
                },
                TyirType::Tuple(ty),
                span(),
            );
        },
        |builder, point| {
            builder.push_value(
                Operation::CallRuntime {
                    function: RuntimeFunction::ObjectRepr,
                    arguments: vec![point],
                },
                TyirType::STR,
                span(),
            );
        },
    ];
    for uses in kept {
        assert!(!point_used(uses, Vec::new()));
    }
}

#[test]
fn an_instance_a_slot_or_a_cast_holds_escapes_through_them() {
    let through_slot = |builder: &mut FunctionBuilder, point: ValueId| {
        let slot = builder.add_slot("p", POINT);
        builder.push(Operation::Store { slot, value: point }, span());
        let loaded = builder.push_value(Operation::Load(slot), POINT, span());
        let bits = builder.push_value(Operation::ToBits(loaded), TyirType::Bits, span());
        let back = builder.push_value(Operation::FromBits { bits, ty: POINT }, POINT, span());
        store_global(builder, back);
    };
    assert!(!point_used(through_slot, Vec::new()));
    let through_block = |builder: &mut FunctionBuilder, point: ValueId| {
        let next = builder.create_block();
        let held = builder.add_block_parameter(next, POINT);
        builder.terminate(Terminator::Jump {
            target: next,
            arguments: vec![point],
        });
        builder.switch_to(next);
        store_global(builder, held);
    };
    assert!(!point_used(through_block, Vec::new()));
    let unrelated_slot = |builder: &mut FunctionBuilder, point: ValueId| {
        let slot: SlotId = builder.add_slot("p", POINT);
        builder.push(Operation::Store { slot, value: point }, span());
        let other = new_instance(builder, 0);
        store_global(builder, other);
    };
    assert!(point_used(unrelated_slot, Vec::new()));
}

#[test]
fn a_returned_instance_comes_from_the_heap() {
    let mut builder = FunctionBuilder::new(FunctionId(0), "f0", "f0", POINT);
    let point = new_instance(&mut builder, 0);
    builder.terminate(Terminator::Return(point));
    assert_eq!(placed(vec![builder.finish()]), [false]);
    let mut builder = FunctionBuilder::new(FunctionId(0), "f0", "f0", TyirType::Unit);
    builder.set_may_raise(true);
    let point = new_instance(&mut builder, 0);
    builder.terminate(Terminator::ReturnError(point));
    assert_eq!(placed(vec![builder.finish()]), [false]);
}

#[test]
fn an_instance_passed_on_escapes_when_the_parameter_does() {
    let call = |function: u32| {
        move |builder: &mut FunctionBuilder, point: ValueId| {
            builder.push_value(
                Operation::Call {
                    function: FunctionId(function),
                    arguments: vec![point],
                },
                TyirType::Unit,
                span(),
            );
        }
    };
    assert!(point_used(call(1), vec![keeps_nothing(1)]));
    assert!(!point_used(call(1), vec![keeps_its_parameter(1)]));
    // A parameter passed on to one that escapes escapes too, whichever function comes first.
    let passes_on = taking(1, POINT, |builder, parameter| {
        builder.push_value(
            Operation::Call {
                function: FunctionId(2),
                arguments: vec![parameter],
            },
            TyirType::Unit,
            span(),
        );
    });
    assert!(!point_used(
        call(1),
        vec![passes_on, keeps_its_parameter(2)]
    ));
    let invoked = |builder: &mut FunctionBuilder, point: ValueId| {
        let normal = builder.create_block();
        builder.add_block_parameter(normal, TyirType::Unit);
        let error = builder.create_block();
        let exception = builder.add_block_parameter(error, TyirType::EXCEPTION);
        builder.terminate(Terminator::Invoke {
            callee: Callee::Function(FunctionId(1)),
            arguments: vec![point],
            normal,
            error,
            span: span(),
        });
        builder.switch_to(error);
        builder.terminate(Terminator::ReturnError(exception));
        builder.switch_to(normal);
    };
    assert!(point_used(invoked, vec![keeps_nothing(1)]));
    assert!(!point_used(invoked, vec![keeps_its_parameter(1)]));
}

#[test]
fn a_call_through_a_table_counts_each_method_a_derived_class_puts_there() {
    let call_shape = |builder: &mut FunctionBuilder, point: ValueId| {
        builder.push_value(
            Operation::CallMethod {
                class: 2,
                slot: 0,
                arguments: vec![point],
            },
            TyirType::Unit,
            span(),
        );
    };
    assert!(point_used(
        call_shape,
        vec![keeps_nothing(1), keeps_nothing(2)]
    ));
    assert!(!point_used(
        call_shape,
        vec![keeps_nothing(1), keeps_its_parameter(2)]
    ));
    // A call through `Square`'s table never runs `Shape`'s method.
    let call_square = |builder: &mut FunctionBuilder, point: ValueId| {
        let normal = builder.create_block();
        builder.add_block_parameter(normal, TyirType::Unit);
        let error = builder.create_block();
        let exception = builder.add_block_parameter(error, TyirType::EXCEPTION);
        builder.terminate(Terminator::Invoke {
            callee: Callee::Method { class: 3, slot: 0 },
            arguments: vec![point],
            normal,
            error,
            span: span(),
        });
        builder.switch_to(error);
        builder.terminate(Terminator::ReturnError(exception));
        builder.switch_to(normal);
    };
    assert!(point_used(
        call_square,
        vec![keeps_its_parameter(1), keeps_nothing(2)]
    ));
}

#[test]
fn an_exception_instance_comes_from_the_heap() {
    let main = taking(0, TyirType::Int, |builder, _| {
        new_instance(builder, 1);
        new_instance(builder, 0);
    });
    assert_eq!(placed(vec![main]), [false, true]);
}

#[test]
fn a_runtime_call_on_an_instance_lets_it_escape_unless_it_reads_only_its_type() {
    for (runtime, escapes) in [
        (RuntimeFunction::IsInstance, false),
        (RuntimeFunction::ObjectHasTypeId, false),
        (RuntimeFunction::ObjectStr, true),
    ] {
        let uses = move |builder: &mut FunctionBuilder, point: ValueId| {
            let normal = builder.create_block();
            builder.add_block_parameter(normal, TyirType::Bool);
            let error = builder.create_block();
            let exception = builder.add_block_parameter(error, TyirType::EXCEPTION);
            builder.terminate(Terminator::Invoke {
                callee: Callee::Runtime(runtime),
                arguments: vec![point],
                normal,
                error,
                span: span(),
            });
            builder.switch_to(error);
            builder.terminate(Terminator::ReturnError(exception));
            builder.switch_to(normal);
        };
        assert_eq!(point_used(uses, Vec::new()), !escapes, "{runtime:?}");
    }
}

#[test]
fn the_verifier_refuses_a_frame_value_that_is_not_a_new_instance() {
    let mut main = taking(0, POINT, |_, _| {});
    main.values[0].representation = ValueRepresentation::Stack;
    let error = crate::verify(&module(vec![main])).unwrap_err();
    assert_eq!(
        error.message,
        "v0 is held in the frame but is not a new instance of a class"
    );
    let mut raising = taking(0, TyirType::Int, |builder, _| {
        new_instance(builder, 1);
    });
    raising.values[1].representation = ValueRepresentation::Stack;
    assert!(crate::verify(&module(vec![raising])).is_err());
}

#[test]
fn the_text_form_shows_an_instance_held_in_the_frame() {
    let main = taking(0, TyirType::Int, |builder, _| {
        new_instance(builder, 0);
    });
    let mut module = module(vec![main]);
    place_instances(&mut module);
    assert_eq!(crate::verify(&module), Ok(()));
    assert!(crate::print(&module).contains("v1 = new class#0 in frame"));
}
