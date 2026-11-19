//! Lowers the whole program, finds which functions may raise, and verifies the result.

use typy_check::TypedProgram;
use typy_diagnostics::{Diagnostic, DiagnosticCode, Diagnostics, RuleId};
use typy_tyir::{
    CType, FunctionId, TupleTypes, TyirClass, TyirExport, TyirLibrary, TyirModule, place_instances,
    verify,
};

use crate::types::tyir_type;

use crate::call_graph::recursive_functions;
use crate::function::FunctionLowerer;
use crate::methods::{Calls, MethodTables};
use crate::module_variable::ModuleGlobals;
use crate::strings::StringPool;

/// Returns the program in TyIR, verified, with each instance that does not escape its function
/// held in its frame (MEM-03). A function may raise when its lowered code contains an `invoke`,
/// so lowering repeats until that set stops growing (detailed design section 9).
pub fn lower(program: &TypedProgram) -> Result<TyirModule, Diagnostics> {
    let recursive = recursive_functions(program);
    let methods = MethodTables::new(program);
    let families = methods.families();
    // Each generator function's resume function comes after the program's functions
    // (ADR-0063).
    let generators: Vec<&typy_check::TypedFunction> = program
        .functions
        .iter()
        .filter(|function| function.generator.is_some() && !function.generic)
        .collect();
    let mut may_raise = vec![false; program.functions.len() + generators.len()];
    let slot_methods: Vec<FunctionId> = program
        .classes
        .iter()
        .flat_map(|class| Slots::of(program, class.id).methods().collect::<Vec<_>>())
        .collect();
    loop {
        // A slot's method that may raise leaves its exception with the runtime, which each
        // runtime call that may run a slot is then followed by a test of (CLS-22, ADR-0039).
        let raising_slots = slot_methods
            .iter()
            .any(|method| may_raise[method.0 as usize]);
        let mut strings = StringPool::default();
        let mut tuple_types = TupleTypes::default();
        let mut cdata_classes = crate::cdata::CDataClasses::default();
        let module_globals = ModuleGlobals::new(program, &mut tuple_types);
        let calls = Calls {
            may_raise: &may_raise,
            methods: &methods,
            raising_slots,
            keeps_frames: !generators.is_empty(),
        };
        let functions = lower_functions(
            program,
            &generators,
            &calls,
            crate::function::Tables {
                strings: &mut strings,
                tuple_types: &mut tuple_types,
                cdata_classes: &mut cdata_classes,
            },
            (&module_globals, &recursive),
        );
        // The function `main` calls at the end comes last (ADR-0071).
        let finish = FunctionId(u32::try_from(functions.len() - 1).unwrap_or(u32::MAX));
        let found = raising_functions(&functions, &families);
        if found != may_raise {
            may_raise = found;
            continue;
        }
        let classes: Vec<TyirClass> = program
            .classes
            .iter()
            .map(|class| tyir_class(program, class, &methods, &mut tuple_types))
            .collect();
        let (strings, str_points) = strings.into_tables();
        let mut module = TyirModule {
            functions,
            globals: module_globals.globals,
            modules: module_objects(program),
            type_aliases: program
                .type_aliases
                .iter()
                .map(|alias| typy_tyir::TyirTypeAlias {
                    name: alias.name.clone(),
                    module: alias.module.clone(),
                })
                .collect(),
            strings,
            str_points,
            tuple_types,
            classes,
            entry: FunctionId(program.entry.0),
            finish,
            allocator: typy_tyir::TyirAllocator::default(),
            foreign_libraries: foreign_libraries(program),
            foreign_functions: foreign_functions(program, &mut cdata_classes),
            cdata_classes: cdata_classes.into_table(),
            library: program
                .exports
                .as_ref()
                .map(|exports| library(program, exports)),
        };
        place_instances(&mut module);
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

/// Returns the lowered functions of the program, in order, then the resume function of each
/// generator function of `generators`, in order, whose own lowering makes its generator
/// (ADR-0063), then the function `main` calls at the end (ADR-0071).
fn lower_functions(
    program: &TypedProgram,
    generators: &[&typy_check::TypedFunction],
    calls: &Calls<'_>,
    tables: crate::function::Tables<'_>,
    (module_globals, recursive): (
        &ModuleGlobals,
        &std::collections::BTreeSet<typy_check::FunctionId>,
    ),
) -> Vec<typy_tyir::TyirFunction> {
    let crate::function::Tables {
        strings,
        tuple_types,
        cdata_classes,
    } = tables;
    let resume_of = |position: usize| {
        FunctionId(u32::try_from(program.functions.len() + position).unwrap_or(u32::MAX))
    };
    let mut functions: Vec<_> = program
        .functions
        .iter()
        .map(|function| {
            if let Some(position) = generators.iter().position(|each| each.id == function.id) {
                return crate::generator::maker(
                    program,
                    function,
                    resume_of(position),
                    tuple_types,
                );
            }
            FunctionLowerer::lower(
                program,
                (function, None),
                calls,
                crate::function::Tables {
                    strings,
                    tuple_types,
                    cdata_classes,
                },
                module_globals,
                recursive.contains(&function.id),
            )
        })
        .collect();
    for (position, function) in generators.iter().enumerate() {
        functions.push(FunctionLowerer::lower(
            program,
            (function, Some(resume_of(position))),
            calls,
            crate::function::Tables {
                strings,
                tuple_types,
                cdata_classes,
            },
            module_globals,
            true,
        ));
    }
    let finish = typy_check::FunctionId(u32::try_from(functions.len()).unwrap_or(u32::MAX));
    functions.push(crate::finish::finish_function(
        program,
        calls,
        crate::function::Tables {
            strings,
            tuple_types,
            cdata_classes,
        },
        module_globals,
        finish,
    ));
    functions
}

/// Returns whether each function may raise: one whose lowered code does, and each method of
/// a table entry where one may, since a call through the table may run any of them
/// (ADR-0021).
fn raising_functions(
    functions: &[typy_tyir::TyirFunction],
    families: &[Vec<typy_check::FunctionId>],
) -> Vec<bool> {
    let mut found: Vec<bool> = functions
        .iter()
        .map(|function| function.may_raise)
        .collect();
    for family in families {
        if family.iter().any(|id| found[id.0 as usize]) {
            for id in family {
                found[id.0 as usize] = true;
            }
        }
    }
    found
}

/// Returns the name of each C library the program declares, which the build links, or `None`
/// for the C library every program links (FFI-01, FFI-14).
fn foreign_libraries(program: &TypedProgram) -> Vec<Option<String>> {
    program
        .foreign_libraries
        .iter()
        .map(|library| library.name.clone())
        .collect()
}

/// Returns the C functions the program calls, with their C types, adding the class of each
/// pointer they take or give to `classes` (FFI-02, FFI-03).
fn foreign_functions(
    program: &TypedProgram,
    classes: &mut crate::cdata::CDataClasses,
) -> Vec<typy_tyir::TyirForeignFunction> {
    program
        .foreign_functions
        .iter()
        .map(|function| typy_tyir::TyirForeignFunction {
            name: function.name.clone(),
            library: function.library,
            parameters: function
                .parameters
                .iter()
                .map(|ty| crate::foreign_call::foreign_type(ty, classes))
                .collect(),
            result: function
                .result
                .as_ref()
                .map(|ty| crate::foreign_call::foreign_type(ty, classes)),
        })
        .collect()
}

/// Returns what a library exports, each function with the C types of its parameters and
/// result, which the checker has found to be `int`, `float`, `bool` or, for a result,
/// `None` (FFI-05, ADR-0032).
fn library(program: &TypedProgram, exports: &[typy_check::Export]) -> TyirLibrary {
    let c_type = |ty: &typy_check::Type| match ty {
        typy_check::Type::Int => Some(CType::Int64),
        typy_check::Type::Float => Some(CType::Double),
        typy_check::Type::Bool => Some(CType::Bool),
        _ => None,
    };
    let name = program
        .modules
        .iter()
        .find(|module| module.body == program.entry)
        .map(|module| module.name.clone())
        .unwrap_or_default();
    TyirLibrary {
        name,
        version: program.library_version.clone(),
        exports: exports
            .iter()
            .map(|export| {
                let signature = &program.function(export.function).signature;
                TyirExport {
                    name: export.name.clone(),
                    function: FunctionId(export.function.0),
                    parameters: signature
                        .parameters
                        .iter()
                        .filter_map(|parameter| c_type(&parameter.ty))
                        .collect(),
                    parameter_names: signature
                        .parameters
                        .iter()
                        .map(|parameter| parameter.name.clone())
                        .collect(),
                    result: c_type(&signature.returns),
                }
            })
            .collect(),
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

/// The methods a class's type object's slots call (ADR-0026): the bridge of a comparison
/// method that may give way stands in its place (CLS-21, ADR-0038).
struct Slots {
    equal: Option<FunctionId>,
    order: [Option<FunctionId>; 4],
    hash: Option<FunctionId>,
    truth: Option<FunctionId>,
}

impl Slots {
    /// Returns the methods of the slots of `class`.
    fn of(program: &TypedProgram, class: typy_check::ClassId) -> Self {
        let method = |wanted: &str| program.method(class, wanted).map(|id| FunctionId(id.0));
        let gives_way = |method: FunctionId| {
            program
                .function(typy_check::FunctionId(method.0))
                .signature
                .returns
                .may_give_way()
        };
        // Objects of a chain of bases may be of different classes, which CPython compares as
        // `a < b` does, the right one's reflected method first when its class derives from the
        // left one's; the bridge compares so (CLS-22).
        let chain = program.class(class).base.is_some() || program.has_subclass(class);
        let [equal, less, less_equal, greater, greater_equal] =
            typy_check::COMPARISON_BRIDGES.map(|(name, _, _, bridge)| {
                let own = method(name);
                match (own, method(bridge)) {
                    (Some(own), Some(bridge)) if chain || gives_way(own) => Some(bridge),
                    (Some(own), _) if !gives_way(own) => Some(own),
                    _ => None,
                }
            });
        Self {
            equal,
            order: [less, less_equal, greater, greater_equal],
            hash: method("__hash__"),
            truth: method("__bool__").or_else(|| method(typy_check::TRUTH_BRIDGE)),
        }
    }

    /// Returns each method the slots call.
    fn methods(&self) -> impl Iterator<Item = FunctionId> + '_ {
        self.equal
            .into_iter()
            .chain(self.order.iter().flatten().copied())
            .chain(self.hash)
            .chain(self.truth)
    }
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
    let slots = Slots::of(program, class.id);
    // The `equal` slot calls `__eq__` only for an object of the class its parameter takes.
    let equal = slots.equal.map(|equal| {
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
        order: slots.order,
        hash: slots.hash,
        truth: slots.truth,
        base: class.base.map(|base| base.0),
        exception_base,
        os_error: class
            .exception
            .as_ref()
            .is_some_and(|exception| exception.builtin().is_os_error()),
        public: class.generic.map(|generic| generic.0),
    }
}
