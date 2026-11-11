//! Lowers lazy imports: a lazy import statement leaves its names unbound and arms one flag
//! for each, and the first use of a name runs the import for it, through its own function,
//! which disarms the flag once the import succeeds (MOD-06).

use typy_check::{FunctionId, SymbolId};
use typy_diagnostics::Span;
use typy_tyir::{Constant, GlobalId, Operation, Terminator, TyirType};

use crate::function::FunctionLowerer;

impl FunctionLowerer<'_> {
    /// Lowers a lazy import statement: each name waits for its first use, and a variable
    /// that held a value lets it go, as rebinding the name does in CPython.
    pub(crate) fn lazy_import_statement(
        &mut self,
        reifies: &[FunctionId],
        targets: &[SymbolId],
        span: Span,
    ) {
        for reify in reifies {
            let armed = self.module_globals.armed[reify];
            self.store_flag(armed, true, span);
        }
        for target in targets {
            self.unbind_name(*target, span);
        }
    }

    /// Runs the lazy import of `reify` when it waits: its statement ran, and the variable it
    /// binds, if any, was not bound since.
    pub(crate) fn run_lazy_import(&mut self, reify: FunctionId, span: Span) {
        let armed = self.module_globals.armed[&reify];
        let waiting = self
            .builder
            .push_value(Operation::LoadGlobal(armed), TyirType::Bool, span);
        let run = self.builder.create_block();
        let after = self.builder.create_block();
        let check_bound = self.program.lazy_imports.get(&reify).copied().flatten();
        if let Some(target) = check_bound {
            let unbound = self.builder.create_block();
            self.builder.terminate(Terminator::Branch {
                condition: waiting,
                then: unbound,
                otherwise: after,
            });
            self.builder.switch_to(unbound);
            let bound_flag = self.module_globals.of[&target].bound;
            let bound =
                self.builder
                    .push_value(Operation::LoadGlobal(bound_flag), TyirType::Bool, span);
            self.builder.terminate(Terminator::Branch {
                condition: bound,
                then: after,
                otherwise: run,
            });
        } else {
            self.builder.terminate(Terminator::Branch {
                condition: waiting,
                then: run,
                otherwise: after,
            });
        }
        self.builder.switch_to(run);
        self.call_function(reify, Vec::new(), TyirType::Unit, span);
        self.jump_to(after);
        self.builder.switch_to(after);
    }

    /// Ends the function of a lazy import: the import succeeded, so its name no longer
    /// waits. A function of any other kind is left as it is.
    pub(crate) fn finish_lazy_import(&mut self, span: Span) {
        if let Some(armed) = self.module_globals.armed.get(&self.function_id).copied()
            && !self.builder.is_terminated()
        {
            self.store_flag(armed, false, span);
        }
    }

    /// Stores `true` or `false` in a flag global.
    fn store_flag(&mut self, flag: GlobalId, value: bool, span: Span) {
        let value = self.constant(Constant::Bool(value), span);
        self.builder.push(
            Operation::StoreGlobal {
                global: flag,
                value,
            },
            span,
        );
    }
}
