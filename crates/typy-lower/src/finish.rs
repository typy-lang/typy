//! The function the runtime's `main` calls when the program ends (FUN-11, ADR-0071). In a
//! program with generator functions it releases the module variables, those of `__main__`
//! first, each in the order its module's code binds them, then those of each other module,
//! the last to run first, so a generator still suspended in one closes, as CPython closes
//! it at exit. A program without one has nothing to close, so the function only returns.

use typy_check::{FunctionSignature, Type, TypedFunction, TypedProgram};
use typy_tyir::TyirFunction;

use crate::function::{FunctionLowerer, Tables};
use crate::mangle::FINISH_NAME;
use crate::methods::Calls;
use crate::module_variable::ModuleGlobals;

/// Returns the function `main` calls at the end, under the ID `id`, after every other.
pub(crate) fn finish_function(
    program: &TypedProgram,
    calls: &Calls<'_>,
    tables: Tables<'_>,
    module_globals: &ModuleGlobals,
    id: typy_check::FunctionId,
) -> TyirFunction {
    let span = program.function(program.entry).span;
    let function = TypedFunction {
        id,
        name: FINISH_NAME.to_owned(),
        qualified_name: FINISH_NAME.to_owned(),
        module: "__main__".to_owned(),
        signature: FunctionSignature {
            parameters: Vec::new(),
            returns: Type::NoneType,
        },
        parameters: Vec::new(),
        locals: Vec::new(),
        body: Vec::new(),
        span,
        generic: false,
        named_targets: Vec::new(),
        generator: None,
    };
    let closes = calls.keeps_frames;
    FunctionLowerer::lower_with(
        program,
        (&function, None),
        calls,
        tables,
        (module_globals, false),
        |lowerer| {
            if closes {
                lowerer.release_module_variables(span);
            }
        },
    )
}
