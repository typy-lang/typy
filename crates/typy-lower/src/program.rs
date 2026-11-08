//! Lowers the whole program, finds which functions may raise, and verifies the result.

use typy_check::TypedProgram;
use typy_diagnostics::{Diagnostic, DiagnosticCode, Diagnostics, RuleId};
use typy_tyir::{FunctionId, TupleTypes, TyirModule, verify};

use crate::call_graph::recursive_functions;
use crate::function::FunctionLowerer;
use crate::strings::StringPool;

/// Returns the program in TyIR, verified. A function may raise when its lowered code contains an `invoke`, so lowering repeats until that set stops growing (detailed design section 9).
pub fn lower(program: &TypedProgram) -> Result<TyirModule, Diagnostics> {
    let recursive = recursive_functions(program);
    let mut may_raise = vec![false; program.functions.len()];
    loop {
        let mut strings = StringPool::default();
        let mut tuple_types = TupleTypes::default();
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
        let module = TyirModule {
            functions,
            strings: strings.into_strings(),
            tuple_types,
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
