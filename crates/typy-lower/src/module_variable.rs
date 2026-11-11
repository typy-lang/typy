//! Lowers module variables: each is a global in static storage with a global flag that says
//! whether it is bound, so a function reading it before the module binds it raises
//! `NameError`, as CPython does (SMT-08, ADR-0015).

use std::collections::BTreeMap;

use typy_check::{SymbolId, SymbolKind, TypedProgram};
use typy_diagnostics::Span;
use typy_tyir::{
    Constant, GlobalId, Operation, RuntimeFunction, Terminator, TupleTypes, TyirGlobal, TyirType,
};

use crate::function::FunctionLowerer;
use crate::ownership::Lowered;
use crate::types::tyir_type;

/// The two globals of one module variable.
#[derive(Clone, Copy, Debug)]
pub(crate) struct ModuleGlobal {
    /// The global that holds the value.
    pub(crate) value: GlobalId,
    /// The global that is true once the variable is bound.
    pub(crate) bound: GlobalId,
    /// The function of the code of the module the variable belongs to.
    pub(crate) owner: typy_check::FunctionId,
}

/// The globals of the program's module variables.
#[derive(Debug, Default)]
pub(crate) struct ModuleGlobals {
    /// Every global, as the module lists them.
    pub(crate) globals: Vec<TyirGlobal>,
    /// The globals of each module variable.
    pub(crate) of: BTreeMap<SymbolId, ModuleGlobal>,
    /// The state global of each module, as `TypedProgram::modules` lists them: not
    /// started, running or done (MOD-03).
    pub(crate) states: Vec<GlobalId>,
    /// The flag of each function that runs a lazy import for one name, true while the
    /// import waits for the name's first use (MOD-06).
    pub(crate) armed: BTreeMap<typy_check::FunctionId, GlobalId>,
    /// For each variable bound by a lazy import, the function that runs the import.
    pub(crate) lazy: BTreeMap<SymbolId, typy_check::FunctionId>,
}

impl ModuleGlobals {
    /// Makes the two globals of every module variable of the program.
    pub(crate) fn new(program: &TypedProgram, tuple_types: &mut TupleTypes) -> Self {
        let mut made = Self::default();
        // A module's code holds its module variables among its locals.
        let owners: BTreeMap<SymbolId, typy_check::FunctionId> = program
            .modules
            .iter()
            .flat_map(|module| {
                program
                    .function(module.body)
                    .locals
                    .iter()
                    .map(move |symbol| (*symbol, module.body))
            })
            .collect();
        for symbol in program
            .symbols
            .iter()
            .filter(|symbol| symbol.kind == SymbolKind::ModuleVariable)
        {
            let ty = tyir_type(&symbol.ty, tuple_types);
            let value = made.add(symbol.name.clone(), ty);
            let bound = made.add(format!("{} bound", symbol.name), TyirType::Bool);
            let owner = owners
                .get(&symbol.id)
                .copied()
                .unwrap_or(typy_check::FunctionId(u32::MAX));
            made.of.insert(
                symbol.id,
                ModuleGlobal {
                    value,
                    bound,
                    owner,
                },
            );
        }
        for module in &program.modules {
            let state = made.add(format!("{} state", module.name), TyirType::Int);
            made.states.push(state);
        }
        for (reify, target) in &program.lazy_imports {
            let armed = made.add(format!("lazy import {} armed", reify.0), TyirType::Bool);
            made.armed.insert(*reify, armed);
            if let Some(target) = target {
                made.lazy.insert(*target, *reify);
            }
        }
        made
    }

    fn add(&mut self, name: String, ty: TyirType) -> GlobalId {
        let id = GlobalId(u32::try_from(self.globals.len()).unwrap_or(u32::MAX));
        self.globals.push(TyirGlobal { name, ty });
        id
    }
}

impl FunctionLowerer<'_> {
    /// Gives every counted module variable the zero value of its type, before any module code
    /// runs, so each store can release the value it replaces.
    pub(crate) fn initialize_module_variables(&mut self, span: Span) {
        // A state is an `int`, whose zero is not the word of zeros a global starts with.
        for state in self.module_globals.states.clone() {
            self.store_state(state, "0", span);
        }
        let globals: Vec<ModuleGlobal> = self.module_globals.of.values().copied().collect();
        for global in globals {
            let ty = self.module_globals.globals[global.value.0 as usize].ty;
            if let Some(zero) = self.initial_value(ty, span) {
                self.builder.push(
                    Operation::StoreGlobal {
                        global: global.value,
                        value: zero,
                    },
                    span,
                );
            }
        }
    }

    /// Reads a variable: a local from its slot, or a module variable from its global, which a
    /// function first tests for being bound. The value is borrowed.
    pub(crate) fn read_name(&mut self, symbol: SymbolId, span: Span) -> Lowered {
        let Some(global) = self.module_globals.of.get(&symbol).copied() else {
            let slot = self.slots[&symbol];
            return self.load(slot, span);
        };
        // A lazy import leaves its name unbound in the module's own code too.
        if let Some(reify) = self.module_globals.lazy.get(&symbol).copied() {
            self.run_lazy_import(reify, span);
            self.check_bound(symbol, global, span);
        } else if self.function_id != global.owner {
            self.check_bound(symbol, global, span);
        }
        let ty = self.module_globals.globals[global.value.0 as usize].ty;
        let value = self
            .builder
            .push_value(Operation::LoadGlobal(global.value), ty, span);
        Self::borrowed(value, ty)
    }

    /// Raises `NameError` when the module variable is not bound yet.
    fn check_bound(&mut self, symbol: SymbolId, global: ModuleGlobal, span: Span) {
        let bound =
            self.builder
                .push_value(Operation::LoadGlobal(global.bound), TyirType::Bool, span);
        let unbound = self.builder.create_block();
        let read = self.builder.create_block();
        self.builder.terminate(Terminator::Branch {
            condition: bound,
            then: read,
            otherwise: unbound,
        });
        self.builder.switch_to(unbound);
        let name = self.string(&self.program.symbol(symbol).name.clone(), span);
        self.call_runtime(RuntimeFunction::NameError, vec![name], span);
        self.builder.terminate(Terminator::Unreachable);
        self.builder.switch_to(read);
    }

    /// Unbinds a module variable: the value it held, if counted, is released and replaced by
    /// its type's zero value, so a later store can release what it replaces.
    pub(crate) fn unbind_name(&mut self, symbol: SymbolId, span: Span) {
        let global = self.module_globals.of[&symbol];
        let ty = self.module_globals.globals[global.value.0 as usize].ty;
        if let Some(zero) = self.initial_value(ty, span) {
            let old = self
                .builder
                .push_value(Operation::LoadGlobal(global.value), ty, span);
            self.builder.push(
                Operation::StoreGlobal {
                    global: global.value,
                    value: zero,
                },
                span,
            );
            self.builder.push(Operation::Decrement(old), span);
        }
        let unbound = self.constant(Constant::Bool(false), span);
        self.builder.push(
            Operation::StoreGlobal {
                global: global.bound,
                value: unbound,
            },
            span,
        );
    }

    /// Stores into a variable, which takes over the value's reference: a local's slot, or a
    /// module variable's global, which is bound from then on. The value replaced is released.
    pub(crate) fn assign_name(&mut self, symbol: SymbolId, lowered: Lowered, span: Span) {
        let Some(global) = self.module_globals.of.get(&symbol).copied() else {
            let slot = self.slots[&symbol];
            self.store(slot, lowered, span);
            return;
        };
        let value = self.take_reference(lowered, span);
        let old = self.counted(lowered.ty).then(|| {
            self.builder
                .push_value(Operation::LoadGlobal(global.value), lowered.ty, span)
        });
        self.builder.push(
            Operation::StoreGlobal {
                global: global.value,
                value,
            },
            span,
        );
        if let Some(old) = old {
            self.builder.push(Operation::Decrement(old), span);
        }
        let bound = self.constant(Constant::Bool(true), span);
        self.builder.push(
            Operation::StoreGlobal {
                global: global.bound,
                value: bound,
            },
            span,
        );
    }
}
