//! Tests of the verifier on instances of classes, through the public API.

use typy_diagnostics::{FileId, Span, TextOffset};
use typy_tyir::{
    Constant, FunctionBuilder, FunctionId, Operation, Terminator, TupleTypes, TyirClass,
    TyirModule, TyirType, TypeId, ValueId, print, verify,
};

fn span() -> Span {
    Span::empty(FileId(0), TextOffset(0))
}

fn message(module: &TyirModule) -> String {
    verify(module).expect_err("the module is invalid").message
}

/// Operations to add to a function, each with the type of its result, if it has one.
type Body = Vec<(Operation, Option<TyirType>)>;

/// An operation on an instance and an `int`.
type OnPoint = fn(ValueId, ValueId) -> Operation;

/// A function of one parameter of class `Point`, whose one attribute is an `int`, with the
/// operations `body` gives, on that parameter and an `int` constant.
fn on_point(body: impl FnOnce(ValueId, ValueId) -> Body) -> TyirModule {
    let point = TyirType::Object(TypeId::class(0));
    let mut builder = FunctionBuilder::new(FunctionId(0), "_TY8__main__1f", "f", TyirType::Unit);
    let parameter = builder.add_parameter(point);
    let number = builder.push_value(
        Operation::Constant(Constant::Int("1".to_owned())),
        TyirType::Int,
        span(),
    );
    for (operation, ty) in body(parameter, number) {
        match ty {
            Some(ty) => {
                builder.push_value(operation, ty, span());
            }
            None => builder.push(operation, span()),
        }
    }
    let unit = builder.push_value(Operation::Constant(Constant::Unit), TyirType::Unit, span());
    builder.terminate(Terminator::Return(unit));
    let mut tuple_types = TupleTypes::default();
    let TyirType::Tuple(fields) = tuple_types.intern(vec![TyirType::Int]) else {
        unreachable!("interning gives a tuple type");
    };
    TyirModule {
        functions: vec![builder.finish()],
        strings: Vec::new(),
        tuple_types,
        classes: vec![TyirClass {
            name: "Point".to_owned(),
            module: "__main__".to_owned(),
            fields,
            repr: None,
            str: None,
            exception_base: None,
        }],
        entry: FunctionId(0),
    }
}

#[test]
#[allow(
    clippy::too_many_lines,
    reason = "one table row per operation and per way it can be wrong"
)]
fn instance_operations_check_their_types() {
    let point = TyirType::Object(TypeId::class(0));
    let valid = on_point(|object, number| {
        vec![
            (Operation::NewInstance { class: 0 }, Some(point)),
            (
                Operation::Field {
                    object,
                    class: None,
                    index: 0,
                },
                Some(TyirType::Int),
            ),
            (
                Operation::SetField {
                    class: None,
                    object,
                    index: 0,
                    value: number,
                },
                None,
            ),
            (
                Operation::Identical {
                    left: object,
                    right: object,
                },
                Some(TyirType::Bool),
            ),
            (
                Operation::Constant(Constant::NoInstance(TypeId::class(0))),
                Some(point),
            ),
        ]
    });
    assert_eq!(verify(&valid), Ok(()));
    let text = print(&valid);
    for line in [
        "new class#0",
        "field v0, 0",
        "set_field v0, 0, v1",
        "identical v0, v0",
        "null class#0",
    ] {
        assert!(text.contains(line), "{line} in\n{text}");
    }
    let cases: [(OnPoint, &str); 5] = [
        (
            |_, _| Operation::NewInstance { class: 1 },
            "class#1 is not a class",
        ),
        (
            |_, number| Operation::Field {
                class: None,
                object: number,
                index: 0,
            },
            "is not an instance",
        ),
        (
            |object, _| Operation::Field {
                object,
                class: None,
                index: 1,
            },
            "Point has no attribute 1",
        ),
        (
            |object, _| Operation::SetField {
                class: None,
                object,
                index: 0,
                value: object,
            },
            "the attribute's value",
        ),
        (
            |object, number| Operation::Identical {
                left: object,
                right: number,
            },
            "is not an object",
        ),
    ];
    for (operation, expected) in cases {
        let module = on_point(|object, number| {
            let operation = operation(object, number);
            let ty = match operation {
                Operation::SetField { .. } => None,
                Operation::Identical { .. } => Some(TyirType::Bool),
                Operation::NewInstance { .. } => Some(TyirType::Object(TypeId::class(1))),
                _ => Some(TyirType::Int),
            };
            vec![(operation, ty)]
        });
        assert!(
            message(&module).contains(expected),
            "{expected}: {}",
            message(&module)
        );
    }
    assert_eq!(point.to_string(), "class#0");
    let unknown = TyirType::Object(TypeId::class(5));
    let field_of_unknown = on_point(|_, _| Vec::new());
    assert!(field_of_unknown.class(TypeId::STR).is_none());
    assert!(field_of_unknown.class(TypeId::class(5)).is_none());
    assert_ne!(unknown, point);
}

/// A module whose class 0 is an exception class with one `int` attribute, and whose function
/// takes `parameter` and applies `operation` to it.
fn on_exception(parameter: TyirType, operation: Operation) -> TyirModule {
    let mut builder = FunctionBuilder::new(FunctionId(0), "_TY8__main__1f", "f", TyirType::Unit);
    let object = builder.add_parameter(parameter);
    let (operation, ty) = match operation {
        Operation::Field { class, index, .. } => (
            Operation::Field {
                object,
                class,
                index,
            },
            Some(TyirType::Int),
        ),
        other => (other, None),
    };
    match ty {
        Some(ty) => {
            builder.push_value(operation, ty, span());
        }
        None => builder.push(operation, span()),
    }
    let unit = builder.push_value(Operation::Constant(Constant::Unit), TyirType::Unit, span());
    builder.terminate(Terminator::Return(unit));
    let mut tuple_types = TupleTypes::default();
    let TyirType::Tuple(fields) = tuple_types.intern(vec![TyirType::Int]) else {
        unreachable!("interning gives a tuple type");
    };
    TyirModule {
        functions: vec![builder.finish()],
        strings: Vec::new(),
        tuple_types,
        classes: vec![TyirClass {
            name: "AppError".to_owned(),
            module: "__main__".to_owned(),
            fields,
            repr: None,
            str: None,
            exception_base: Some(typy_tyir::TypeObjectRef::Exception("ValueError")),
        }],
        entry: FunctionId(0),
    }
}

#[test]
fn an_exception_s_attribute_names_its_class() {
    let field = Operation::Field {
        object: ValueId(0),
        class: Some(0),
        index: 0,
    };
    let valid = on_exception(TyirType::EXCEPTION, field.clone());
    assert_eq!(verify(&valid), Ok(()));
    assert!(
        print(&valid).contains("field v0 of class#0, 0"),
        "{}",
        print(&valid)
    );
    let error = verify(&on_exception(TyirType::Int, field)).unwrap_err();
    assert!(error.to_string().contains("is not an instance"), "{error}");
}
