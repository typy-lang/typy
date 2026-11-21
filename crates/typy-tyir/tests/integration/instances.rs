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
        globals: Vec::new(),
        modules: Vec::new(),
        type_aliases: Vec::new(),
        functions: vec![builder.finish()],
        strings: Vec::new(),
        str_points: Vec::new(),
        tuple_types,
        classes: vec![TyirClass {
            name: "Point".to_owned(),
            module: "__main__".to_owned(),
            fields,
            methods: Vec::new(),
            repr: None,
            str: None,
            equal: None,
            order: [None; 4],
            hash: None,
            truth: None,
            next: None,
            iter: None,
            base: None,
            exception_base: None,
            os_error: false,
            tuple_base: false,
            public: None,
        }],
        entry: FunctionId(0),
        finish: FunctionId(0),
        allocator: typy_tyir::TyirAllocator::default(),
        foreign_libraries: Vec::new(),
        foreign_functions: Vec::new(),
        cdata_classes: Vec::new(),
        library: None,
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
        globals: Vec::new(),
        modules: Vec::new(),
        type_aliases: Vec::new(),
        functions: vec![builder.finish()],
        strings: Vec::new(),
        str_points: Vec::new(),
        tuple_types,
        classes: vec![TyirClass {
            name: "AppError".to_owned(),
            module: "__main__".to_owned(),
            fields,
            methods: Vec::new(),
            repr: None,
            str: None,
            equal: None,
            order: [None; 4],
            hash: None,
            truth: None,
            next: None,
            iter: None,
            base: None,
            exception_base: Some(typy_tyir::TypeObjectRef::Exception("ValueError")),
            os_error: false,
            tuple_base: false,
            public: None,
        }],
        entry: FunctionId(0),
        finish: FunctionId(0),
        allocator: typy_tyir::TyirAllocator::default(),
        foreign_libraries: Vec::new(),
        foreign_functions: Vec::new(),
        cdata_classes: Vec::new(),
        library: None,
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

/// A module of three classes, `Shape`, `Circle` derived from it and `Other`, and a function
/// of a `Circle` and an `int`, with one operation `make` gives on them.
fn on_circle(make: impl FnOnce(ValueId, ValueId) -> (Operation, TyirType)) -> TyirModule {
    let circle = TyirType::Object(TypeId::class(1));
    let mut builder = FunctionBuilder::new(FunctionId(0), "_TY8__main__1f", "f", TyirType::Unit);
    let parameter = builder.add_parameter(circle);
    let number = builder.push_value(
        Operation::Constant(Constant::Int("1".to_owned())),
        TyirType::Int,
        span(),
    );
    let (operation, ty) = make(parameter, number);
    builder.push_value(operation, ty, span());
    let unit = builder.push_value(Operation::Constant(Constant::Unit), TyirType::Unit, span());
    builder.terminate(Terminator::Return(unit));
    let mut tuple_types = TupleTypes::default();
    let TyirType::Tuple(fields) = tuple_types.intern(Vec::new()) else {
        unreachable!("interning gives a tuple type");
    };
    let class = |name: &str, base: Option<u32>| TyirClass {
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
        next: None,
        iter: None,
        base,
        exception_base: None,
        os_error: false,
        tuple_base: false,
        public: None,
    };
    TyirModule {
        globals: Vec::new(),
        modules: Vec::new(),
        type_aliases: Vec::new(),
        functions: vec![builder.finish()],
        strings: Vec::new(),
        str_points: Vec::new(),
        tuple_types,
        classes: vec![
            class("Shape", None),
            class("Circle", Some(0)),
            class("Other", None),
        ],
        entry: FunctionId(0),
        finish: FunctionId(0),
        allocator: typy_tyir::TyirAllocator::default(),
        foreign_libraries: Vec::new(),
        foreign_functions: Vec::new(),
        cdata_classes: Vec::new(),
        library: None,
    }
}

#[test]
fn an_upcast_reaches_a_base_and_nothing_else() {
    let shape = TyirType::Object(TypeId::class(0));
    let valid = on_circle(|circle, _| {
        (
            Operation::Upcast {
                value: circle,
                ty: shape,
            },
            shape,
        )
    });
    assert_eq!(verify(&valid), Ok(()));
    assert!(
        print(&valid).contains("upcast v0, class#0"),
        "{}",
        print(&valid)
    );
    let other = TyirType::Object(TypeId::class(2));
    let unrelated = on_circle(|circle, _| {
        (
            Operation::Upcast {
                value: circle,
                ty: other,
            },
            other,
        )
    });
    assert!(message(&unrelated).contains("is not an instance of a class derived from"));
    let number = on_circle(|_, number| {
        (
            Operation::Upcast {
                value: number,
                ty: shape,
            },
            shape,
        )
    });
    assert!(message(&number).contains("v1 of type int is not an instance"));
    let circle = TyirType::Object(TypeId::class(1));
    let back = on_circle(|value, _| (Operation::Downcast { value, ty: circle }, circle));
    assert_eq!(verify(&back), Ok(()));
    assert!(
        print(&back).contains("downcast v0, class#1"),
        "{}",
        print(&back)
    );
    let sideways = on_circle(|value, _| (Operation::Downcast { value, ty: other }, other));
    assert!(message(&sideways).contains("is not the type of a class derived from"));
    let optional = TyirType::Optional(typy_tyir::OptionalBase::Object(TypeId::class(0)));
    let to_optional = on_circle(|circle, _| {
        (
            Operation::Upcast {
                value: circle,
                ty: optional,
            },
            optional,
        )
    });
    assert!(message(&to_optional).contains("is not an instance of a class derived from"));
}

/// A method of a class `class` of the module that returns `result`, the function `id`.
fn method(id: u32, class: u32, result: TyirType) -> typy_tyir::TyirFunction {
    let mut builder = FunctionBuilder::new(FunctionId(id), format!("m{id}"), "m", result);
    builder.add_parameter(TyirType::Object(TypeId::class(class)));
    let constant = if result == TyirType::Float {
        Constant::Float(1.0)
    } else {
        Constant::Int("1".to_owned())
    };
    let value = builder.push_value(Operation::Constant(constant), result, span());
    builder.terminate(Terminator::Return(value));
    builder.finish()
}

/// A module whose entry calls the method at `slot` of `Shape`'s table on a `Circle`, where
/// `Shape`'s table holds `Shape.area` and `Circle`'s the tables `circle` gives.
fn dispatching(slot: u32, circle: Vec<FunctionId>, circle_area: TyirType) -> TyirModule {
    let mut builder = FunctionBuilder::new(FunctionId(0), "entry", "entry", TyirType::Unit);
    let instance = builder.push_value(
        Operation::NewInstance { class: 1 },
        TyirType::Object(TypeId::class(1)),
        span(),
    );
    let shape = TyirType::Object(TypeId::class(0));
    let upcast = builder.push_value(
        Operation::Upcast {
            value: instance,
            ty: shape,
        },
        shape,
        span(),
    );
    builder.push_value(
        Operation::CallMethod {
            class: 0,
            slot,
            arguments: vec![upcast],
        },
        TyirType::Int,
        span(),
    );
    let unit = builder.push_value(Operation::Constant(Constant::Unit), TyirType::Unit, span());
    builder.terminate(Terminator::Return(unit));
    let mut tuple_types = TupleTypes::default();
    let TyirType::Tuple(fields) = tuple_types.intern(Vec::new()) else {
        unreachable!("interning gives a tuple type");
    };
    let class = |name: &str, base: Option<u32>, methods: Vec<FunctionId>| TyirClass {
        name: name.to_owned(),
        module: "__main__".to_owned(),
        fields,
        methods,
        repr: None,
        str: None,
        equal: None,
        order: [None; 4],
        hash: None,
        truth: None,
        next: None,
        iter: None,
        base,
        exception_base: None,
        os_error: false,
        tuple_base: false,
        public: None,
    };
    TyirModule {
        globals: Vec::new(),
        modules: Vec::new(),
        type_aliases: Vec::new(),
        functions: vec![
            builder.finish(),
            method(1, 0, TyirType::Int),
            method(2, 1, circle_area),
        ],
        strings: Vec::new(),
        str_points: Vec::new(),
        tuple_types,
        classes: vec![
            class("Shape", None, vec![FunctionId(1)]),
            class("Circle", Some(0), circle),
        ],
        entry: FunctionId(0),
        finish: FunctionId(0),
        allocator: typy_tyir::TyirAllocator::default(),
        foreign_libraries: Vec::new(),
        foreign_functions: Vec::new(),
        cdata_classes: Vec::new(),
        library: None,
    }
}

#[test]
fn a_method_call_goes_through_a_table_that_extends_its_base_s() {
    let valid = dispatching(0, vec![FunctionId(2)], TyirType::Int);
    assert_eq!(verify(&valid), Ok(()));
    assert!(
        print(&valid).contains("call class#0.method#0(v1)"),
        "{}",
        print(&valid)
    );
    let cases = [
        (
            dispatching(1, vec![FunctionId(2)], TyirType::Int),
            "class#0 has no method#1 in its table",
        ),
        (
            dispatching(0, Vec::new(), TyirType::Int),
            "the table has 0 methods, fewer than the 1 of its base",
        ),
        (
            dispatching(0, vec![FunctionId(2)], TyirType::Float),
            "method#0, m2, is not called as m1, its base's entry, is",
        ),
        (
            dispatching(0, vec![FunctionId(7)], TyirType::Int),
            "method#0 is fn7, which does not exist",
        ),
        (
            dispatching(0, vec![FunctionId(0)], TyirType::Int),
            "method#0 takes no instance",
        ),
        (
            {
                let mut module = dispatching(0, vec![FunctionId(2)], TyirType::Int);
                module.classes[0].base = Some(1);
                module
            },
            "the bases of class#0 come back to it",
        ),
        (
            {
                let mut module = dispatching(0, vec![FunctionId(2)], TyirType::Int);
                module.classes[0].base = Some(9);
                module
            },
            "the base class#9 does not exist",
        ),
        (
            {
                let mut module = dispatching(0, vec![FunctionId(2)], TyirType::Int);
                let extra = module.functions[2].parameters[0];
                module.functions[2].parameters.push(extra);
                module
            },
            "method#0, m2, is not called as m1, its base's entry, is",
        ),
    ];
    for (module, expected) in cases {
        let found = message(&module);
        assert!(found.contains(expected), "{expected} in {found}");
    }
}

#[test]
fn same_method_compares_two_instances_of_a_class_with_a_table() {
    let same = |slot: u32, use_number: bool| {
        let mut module = dispatching(0, vec![FunctionId(2)], TyirType::Int);
        let entry = &mut module.functions[0];
        let mut builder = FunctionBuilder::new(FunctionId(0), "entry", "entry", TyirType::Unit);
        let circle = builder.push_value(
            Operation::NewInstance { class: 1 },
            TyirType::Object(TypeId::class(1)),
            span(),
        );
        let number = builder.push_value(
            Operation::Constant(Constant::Int("1".to_owned())),
            TyirType::Int,
            span(),
        );
        builder.push_value(
            Operation::SameMethod {
                class: 0,
                slot,
                left: circle,
                right: if use_number { number } else { circle },
            },
            TyirType::Bool,
            span(),
        );
        let unit = builder.push_value(Operation::Constant(Constant::Unit), TyirType::Unit, span());
        builder.terminate(Terminator::Return(unit));
        *entry = builder.finish();
        module
    };
    let valid = same(0, false);
    assert_eq!(verify(&valid), Ok(()));
    assert!(print(&valid).contains("same_method class#0.method#0, v0, v0"));
    assert!(message(&same(4, false)).contains("class#0 has no method#4 in its table"));
    assert!(message(&same(0, true)).contains("v1 of type int is not an instance of class#0"));
}

#[test]
fn any_object_becomes_a_union_and_a_union_any_object() {
    let to_union = on_circle(|circle, _| {
        (
            Operation::Upcast {
                value: circle,
                ty: TyirType::UNION,
            },
            TyirType::UNION,
        )
    });
    assert_eq!(verify(&to_union), Ok(()));
    assert!(
        print(&to_union).contains("upcast v0, union"),
        "{}",
        print(&to_union)
    );
    let number = on_circle(|_, number| {
        (
            Operation::Upcast {
                value: number,
                ty: TyirType::UNION,
            },
            TyirType::UNION,
        )
    });
    assert!(message(&number).contains("is not an instance"));
    let optional = |id| TyirType::Optional(typy_tyir::OptionalBase::Object(id));
    let mut builder = FunctionBuilder::new(FunctionId(0), "_TY8__main__1g", "g", TyirType::Unit);
    let held = builder.add_parameter(optional(TypeId::UNION));
    builder.push_value(
        Operation::Downcast {
            value: held,
            ty: optional(TypeId::STR),
        },
        optional(TypeId::STR),
        span(),
    );
    let unit = builder.push_value(Operation::Constant(Constant::Unit), TyirType::Unit, span());
    builder.terminate(Terminator::Return(unit));
    let mut module = on_circle(|circle, _| (Operation::Increment(circle), TyirType::Unit));
    module.functions = vec![builder.finish()];
    assert_eq!(verify(&module), Ok(()));
}
