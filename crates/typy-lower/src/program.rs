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
                        raising_slots,
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
        let (strings, str_points) = strings.into_tables();
        let module = TyirModule {
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
        let [equal, less, less_equal, greater, greater_equal] =
            typy_check::COMPARISON_BRIDGES.map(|(name, _, _, bridge)| {
                method(bridge).or_else(|| method(name).filter(|own| !gives_way(*own)))
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
