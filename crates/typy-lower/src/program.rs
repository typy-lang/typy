//! Lowers the whole program, finds which functions may raise, and verifies the result.

use typy_check::TypedProgram;
use typy_diagnostics::{Diagnostic, DiagnosticCode, Diagnostics, RuleId};
use typy_tyir::{FunctionId, TupleTypes, TyirClass, TyirModule, verify};

use crate::types::tyir_type;

use crate::call_graph::recursive_functions;
use crate::function::FunctionLowerer;
use crate::methods::{Calls, MethodTables};
use crate::module_variable::ModuleGlobals;
use crate::strings::StringPool;

/// Returns the program in TyIR, verified. A function may raise when its lowered code contains an `invoke`, so lowering repeats until that set stops growing (detailed design section 9).
pub fn lower(program: &TypedProgram) -> Result<TyirModule, Diagnostics> {
    let recursive = recursive_functions(program);
    let methods = MethodTables::new(program);
    let families = methods.families();
    let mut may_raise = vec![false; program.functions.len()];
    loop {
        let mut strings = StringPool::default();
        let mut tuple_types = TupleTypes::default();
        let module_globals = ModuleGlobals::new(program, &mut tuple_types);
        let functions: Vec<_> = program
            .functions
            .iter()
            .map(|function| {
                FunctionLowerer::lower(
                    program,
                    function,
                    &Calls {
                        may_raise: &may_raise,
                        methods: &methods,
                    },
                    &mut strings,
                    &mut tuple_types,
                    &module_globals,
                    recursive.contains(&function.id),
                )
            })
            .collect();
        let mut found: Vec<bool> = functions
            .iter()
            .map(|function| function.may_raise)
            .collect();
        // A call through a table may run any method of the entry, so if one may raise, each
        // is called as one that may (ADR-0021).
        for family in &families {
            if family.iter().any(|id| found[id.0 as usize]) {
                for id in family {
                    found[id.0 as usize] = true;
                }
            }
        }
        if found != may_raise {
            may_raise = found;
            continue;
        }
        let classes: Vec<TyirClass> = program
            .classes
            .iter()
            .map(|class| tyir_class(program, class, &methods, &mut tuple_types))
            .collect();
        let raising = raising_slot_methods(program, &classes, &may_raise);
        if !raising.is_empty() {
            return Err(raising);
        }
        let (strings, str_points) = strings.into_tables();
        let module = TyirModule {
            functions,
            globals: module_globals.globals,
            modules: module_objects(program),
            strings,
            str_points,
            tuple_types,
            classes,
            entry: FunctionId(program.entry.0),
        };
        return match verify(&module) {
            Ok(()) => Ok(module),
            Err(error) => {
                let diagnostic = Diagnostic::new(
                    DiagnosticCode::error(9001),
                    program.function(program.entry).span,
                    format!("internal compiler error in lowering: {error}"),
                )
                .with_rule(RuleId::NFR_09)
                .with_help(
                    "this is a defect in TyPy; please report it with the program that caused it",
                );
                Err(Diagnostics::from(diagnostic))
            }
        };
    }
}

/// Returns the module object of each module: the program's, then the built-in modules it
/// imports, which have no file, so their text names none (ADR-0018).
fn module_objects(program: &TypedProgram) -> Vec<typy_tyir::TyirModuleObject> {
    program
        .modules
        .iter()
        .map(|module| typy_tyir::TyirModuleObject {
            name: module.name.clone(),
            path: module.path.clone(),
        })
        .chain(
            program
                .builtin_modules
                .iter()
                .map(|name| typy_tyir::TyirModuleObject {
                    name: name.clone(),
                    path: String::new(),
                }),
        )
        .collect()
}

/// Reports each comparison or hash method that may raise, which a container would call from a
/// type object's slot, which cannot pass an exception on (ADR-0026).
fn raising_slot_methods(
    program: &TypedProgram,
    classes: &[TyirClass],
    may_raise: &[bool],
) -> Diagnostics {
    let mut found = Diagnostics::new();
    let mut reported = Vec::new();
    for class in classes {
        let methods = class
            .equal
            .map(|(method, _)| method)
            .into_iter()
            .chain(class.order.iter().flatten().copied())
            .chain(class.hash)
            .chain(class.truth);
        for method in methods {
            if !may_raise[method.0 as usize] || reported.contains(&method) {
                continue;
            }
            reported.push(method);
            let function = program.function(typy_check::FunctionId(method.0));
            found.push(
                Diagnostic::new(
                    DiagnosticCode::error(201),
                    function.span,
                    format!(
                        "`{}` may raise an exception, and a comparison or hash method that may \
                         raise is planned for TyPy 0.5 (CLS-07)",
                        function.qualified_name
                    ),
                )
                .with_rule(RuleId::CLS_07)
                .with_help("lists, sets and dicts call it, and cannot pass the exception on"),
            );
        }
    }
    found
}

/// Returns a class as TyIR holds it: its attributes' layout, its method table, and the
/// methods its type object's slots call (ADR-0021, ADR-0026).
fn tyir_class(
    program: &TypedProgram,
    class: &typy_check::TypedClass,
    methods: &MethodTables,
    tuple_types: &mut TupleTypes,
) -> TyirClass {
    let fields = class
        .attributes
        .iter()
        .map(|attribute| tyir_type(&attribute.ty, tuple_types))
        .collect();
    let fields = tuple_types.intern_id(fields);
    let method = |wanted: &str| program.method(class.id, wanted).map(|id| FunctionId(id.0));
    let exception_base = class
        .exception
        .as_ref()
        .and_then(typy_check::ExceptionClass::base)
        .map(|base| crate::exception::exception_class(&base));
    // The `equal` slot calls `__eq__` only for an object of the class its parameter takes.
    let equal = method("__eq__").map(|equal| {
        let parameters = &program
            .function(typy_check::FunctionId(equal.0))
            .signature
            .parameters;
        let takes = match parameters.get(1).map(|parameter| &parameter.ty) {
            Some(typy_check::Type::Instance(takes)) => takes.id.0,
            Some(typy_check::Type::Optional(inner)) => match inner.as_ref() {
                typy_check::Type::Instance(takes) => takes.id.0,
                _ => class.id.0,
            },
            _ => class.id.0,
        };
        (equal, takes)
    });
    TyirClass {
        name: class.name.clone(),
        module: class.module.clone(),
        fields,
        methods: methods.table(class.id),
        repr: method("__repr__"),
        str: method("__str__"),
        equal,
        order: ["__lt__", "__le__", "__gt__", "__ge__"].map(method),
        hash: method("__hash__"),
        truth: method("__bool__"),
        base: class.base.map(|base| base.0),
        exception_base,
        public: class.generic.map(|generic| generic.0),
    }
}
