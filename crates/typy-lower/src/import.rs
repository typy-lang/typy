//! Lowers imports: each module is initialised once, at its first import, and a module whose
//! code raises is left not started, so the next import runs it again; then each name takes
//! its value (SMT-07, MOD-03, ADR-0015).

use typy_check::{ImportBinding, SymbolId};
use typy_diagnostics::Span;
use typy_tyir::{Comparison, Constant, GlobalId, Operation, RuntimeFunction, Terminator, TyirType};

use crate::function::{Cleanup, FunctionLowerer};
use crate::ownership::Lowered;

/// The states of a module, held in its state global as an `int`.
const NOT_STARTED: &str = "0";
const RUNNING: &str = "1";
const DONE: &str = "2";

impl FunctionLowerer<'_> {
    /// Lowers an import statement: initialises each module in order, then binds each name.
    pub(crate) fn import_statement(
        &mut self,
        modules: &[String],
        bindings: &[ImportBinding],
        span: Span,
    ) {
        for module in modules {
            self.initialise_module(module, span);
        }
        for binding in bindings {
            match binding {
                ImportBinding::Module { target, module } => {
                    let object = self.module_object(module, span);
                    self.assign_name(*target, object, span);
                }
                ImportBinding::Value {
                    target,
                    module,
                    source,
                } => {
                    self.check_module_bound(
                        module,
                        *source,
                        RuntimeFunction::ModuleImportError,
                        span,
                    );
                    let value = self.read_global(*source, span);
                    self.assign_name(*target, value, span);
                }
            }
        }
    }

    /// Runs a module's code when it has not started: the module is running while its code
    /// runs, done when it returns, and not started again when it raises.
    fn initialise_module(&mut self, module: &str, span: Span) {
        let index = self.module_index(module);
        let state = self.module_globals.states[index];
        let body = self.program.modules[index].body;
        let current = self
            .builder
            .push_value(Operation::LoadGlobal(state), TyirType::Int, span);
        let not_started = self.constant(Constant::Int(NOT_STARTED.to_owned()), span);
        let starts = self.int_compare(Comparison::Equal, current, not_started, span);
        let start = self.builder.create_block();
        let after = self.builder.create_block();
        self.builder.terminate(Terminator::Branch {
            condition: starts,
            then: start,
            otherwise: after,
        });
        self.builder.switch_to(start);
        self.store_state(state, RUNNING, span);
        self.cleanups.push(Cleanup::Initialising { state });
        self.call_function(body, Vec::new(), TyirType::Unit, span);
        self.cleanups.pop();
        self.store_state(state, DONE, span);
        self.jump_to(after);
        self.builder.switch_to(after);
    }

    /// Stores a state, a small `int`, in a module's state global.
    pub(crate) fn store_state(&mut self, state: GlobalId, value: &str, span: Span) {
        let value = self.constant(Constant::Int(value.to_owned()), span);
        self.builder.push(
            Operation::StoreGlobal {
                global: state,
                value,
            },
            span,
        );
    }

    /// Returns the immortal module object of a module.
    pub(crate) fn module_object(&mut self, module: &str, span: Span) -> Lowered {
        let index = self.module_index(module);
        let value = self.constant(
            Constant::Module(u32::try_from(index).unwrap_or(u32::MAX)),
            span,
        );
        Self::borrowed(value, TyirType::MODULE)
    }

    /// Returns where a module is in the program's list of modules, which holds every module
    /// the checker resolved an import to.
    fn module_index(&self, module: &str) -> usize {
        self.program
            .modules
            .iter()
            .position(|known| known.name == module)
            .unwrap_or_default()
    }

    /// Reads `module.name`, a module variable of another module, raising `AttributeError`
    /// while it is not bound (MOD-10).
    pub(crate) fn module_attribute(
        &mut self,
        module: &str,
        symbol: SymbolId,
        span: Span,
    ) -> Lowered {
        self.check_module_bound(module, symbol, RuntimeFunction::ModuleAttributeError, span);
        self.read_global(symbol, span)
    }

    /// Reads `package.module`, raising `AttributeError` until the submodule is done.
    pub(crate) fn submodule(&mut self, module: &str, span: Span) -> Lowered {
        let (parent, child) = module.rsplit_once('.').unwrap_or((module, module));
        let (parent_index, index) = (self.module_index(parent), self.module_index(module));
        let state = self.module_globals.states[index];
        let current = self
            .builder
            .push_value(Operation::LoadGlobal(state), TyirType::Int, span);
        let done = self.constant(Constant::Int(DONE.to_owned()), span);
        let ready = self.int_compare(Comparison::Equal, current, done, span);
        let parent_state = self.module_globals.states[parent_index];
        self.raise_unless(
            ready,
            parent,
            child,
            parent_state,
            RuntimeFunction::ModuleAttributeError,
            span,
        );
        self.module_object(module, span)
    }

    /// Raises the error of `function` for `module.name` unless the module variable is bound.
    fn check_module_bound(
        &mut self,
        module: &str,
        symbol: SymbolId,
        function: RuntimeFunction,
        span: Span,
    ) {
        // The checker resolves `module.name` only to a module variable, which has globals.
        let global = self.module_globals.of[&symbol];
        let index = self.module_index(module);
        let bound =
            self.builder
                .push_value(Operation::LoadGlobal(global.bound), TyirType::Bool, span);
        let name = self.program.symbol(symbol).name.clone();
        let state = self.module_globals.states[index];
        self.raise_unless(bound, module, &name, state, function, span);
    }

    /// Goes on when `condition` holds, and otherwise calls the runtime function that raises
    /// the error for `module.name`, with the module's state, which says whether it is still
    /// initialising.
    fn raise_unless(
        &mut self,
        condition: typy_tyir::ValueId,
        module: &str,
        name: &str,
        state: GlobalId,
        function: RuntimeFunction,
        span: Span,
    ) {
        let failed = self.builder.create_block();
        let fine = self.builder.create_block();
        self.builder.terminate(Terminator::Branch {
            condition,
            then: fine,
            otherwise: failed,
        });
        self.builder.switch_to(failed);
        let object = self.module_object(module, span);
        let text = self.string(name, span);
        let current = self
            .builder
            .push_value(Operation::LoadGlobal(state), TyirType::Int, span);
        self.call_runtime(function, vec![object.value, text, current], span);
        self.builder.terminate(Terminator::Unreachable);
        self.builder.switch_to(fine);
    }

    /// Returns a module variable's value, borrowed from its global.
    fn read_global(&mut self, symbol: SymbolId, span: Span) -> Lowered {
        let global = self.module_globals.of[&symbol];
        let ty = self.module_globals.globals[global.value.0 as usize].ty;
        let value = self
            .builder
            .push_value(Operation::LoadGlobal(global.value), ty, span);
        Self::borrowed(value, ty)
    }
}
