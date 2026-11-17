//! Tests of a module built as a C library, through the public API: the verifier checks each
//! export against its function, and the text form lists them (FFI-05).

use typy_diagnostics::{FileId, Span, TextOffset};
use typy_tyir::{
    CType, Constant, FunctionBuilder, FunctionId, Operation, Terminator, TupleTypes, TyirAllocator,
    TyirExport, TyirLibrary, TyirModule, TyirType, print, verify,
};

fn span() -> Span {
    Span::empty(FileId(0), TextOffset(0))
}

/// A module whose function 1 takes an `int` and a `float` and returns a `bool`, exported as
/// `export`.
fn module(export: TyirExport) -> TyirModule {
    let mut entry = FunctionBuilder::new(FunctionId(0), "__typy_main", "<module>", TyirType::Unit);
    let unit = entry.push_value(Operation::Constant(Constant::Unit), TyirType::Unit, span());
    entry.terminate(Terminator::Return(unit));
    let mut function =
        FunctionBuilder::new(FunctionId(1), "_TY5tools4test", "test", TyirType::Bool);
    function.add_parameter(TyirType::Int);
    function.add_parameter(TyirType::Float);
    let result = function.push_value(
        Operation::Constant(Constant::Bool(true)),
        TyirType::Bool,
        span(),
    );
    function.terminate(Terminator::Return(result));
    TyirModule {
        functions: vec![entry.finish(), function.finish()],
        globals: Vec::new(),
        modules: Vec::new(),
        type_aliases: Vec::new(),
        strings: Vec::new(),
        str_points: Vec::new(),
        tuple_types: TupleTypes::default(),
        classes: Vec::new(),
        entry: FunctionId(0),
        allocator: TyirAllocator::default(),
        library: Some(TyirLibrary {
            name: "tools".to_owned(),
            exports: vec![export],
        }),
    }
}

fn export(parameters: Vec<CType>, result: Option<CType>, function: u32) -> TyirExport {
    TyirExport {
        name: "test".to_owned(),
        function: FunctionId(function),
        parameter_names: parameters.iter().map(|_| "value".to_owned()).collect(),
        parameters,
        result,
    }
}

#[test]
fn an_export_that_matches_its_function_verifies_and_prints() {
    let valid = module(export(
        vec![CType::Int64, CType::Double],
        Some(CType::Bool),
        1,
    ));
    assert_eq!(verify(&valid), Ok(()));
    let text = print(&valid);
    assert!(
        text.contains("library tools\nexport test(int64_t, double) -> bool = fn1\n"),
        "{text}"
    );
}

#[test]
fn an_export_that_does_not_match_its_function_is_refused() {
    for (export, message) in [
        (
            export(vec![CType::Int64], Some(CType::Bool), 1),
            "fn1 takes [Int, Float], not the export's [Int]",
        ),
        (
            export(vec![CType::Int64, CType::Double], None, 1),
            "fn1 returns bool, not the export's unit",
        ),
        (
            export(vec![CType::Int64, CType::Double], Some(CType::Bool), 7),
            "fn7 does not exist",
        ),
    ] {
        let error = verify(&module(export)).unwrap_err();
        assert_eq!(error.function, "export test");
        assert_eq!(error.message, message);
    }
}
