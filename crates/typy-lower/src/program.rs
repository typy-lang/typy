//! Lowers the whole program, finds which functions may raise, and verifies the result.

use typy_check::TypedProgram;
use typy_diagnostics::{Diagnostic, DiagnosticCode, Diagnostics, RuleId};
use typy_tyir::{FunctionId, TupleTypes, TyirClass, TyirModule, TyirType, verify};

use crate::types::tyir_type;

use crate::call_graph::recursive_functions;
use crate::function::FunctionLowerer;
use crate::module_variable::ModuleGlobals;
use crate::strings::StringPool;

/// Returns the program in TyIR, verified. A function may raise when its lowered code contains an `invoke`, so lowering repeats until that set stops growing (detailed design section 9).
pub fn lower(program: &TypedProgram) -> Result<TyirModule, Diagnostics> {
    let recursive = recursive_functions(program);
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
                    &may_raise,
                    &mut strings,
                    &mut tuple_types,
                    &module_globals,
                    recursive.contains(&function.id),
                )
            })
            .collect();
        let found: Vec<bool> = functions
            .iter()
            .map(|function| function.may_raise)
            .collect();
        if found != may_raise {
            may_raise = found;
            continue;
        }
        let classes = program
            .classes
            .iter()
            .map(|class| {
                let fields = class
                    .attributes
                    .iter()
                    .map(|attribute| tyir_type(&attribute.ty, &mut tuple_types))
                    .collect();
                let TyirType::Tuple(fields) = tuple_types.intern(fields) else {
                    unreachable!("interning gives a tuple type");
                };
                // A method of the class, or of the nearest base that has it.
                let method = |wanted: &str| {
                    let mut current = Some(class);
                    while let Some(each) = current {
                        if let Some((_, id)) = each.methods.iter().find(|(name, _)| name == wanted)
                        {
                            return Some(FunctionId(id.0));
                        }
                        current = each.base.map(|base| program.class(base));
                    }
                    None
                };
                let exception_base = class
                    .exception
                    .as_ref()
                    .and_then(typy_check::ExceptionClass::base)
                    .map(|base| crate::exception::exception_class(&base));
                TyirClass {
                    name: class.name.clone(),
                    module: class.module.clone(),
                    fields,
                    repr: method("__repr__"),
                    str: method("__str__"),
                    exception_base,
                }
            })
            .collect();
        let module = TyirModule {
            functions,
            globals: module_globals.globals,
            strings: strings.into_strings(),
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
