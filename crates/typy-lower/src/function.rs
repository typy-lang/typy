//! Lowers one function, or the module body, and writes its prologue.

use std::collections::{BTreeMap, BTreeSet};

use typy_check::{SymbolId, TypedFunction, TypedProgram};
use typy_diagnostics::Span;
use typy_tyir::{
    BlockId, Constant, FunctionBuilder, FunctionId, Operation, RuntimeFunction, SlotId, Terminator,
    TupleTypes, TyirFunction, TyirType, ValueId,
};

use crate::mangle::{ENTRY_SYMBOL, symbol};
use crate::strings::StringPool;
use crate::types::tyir_type;

/// Where `break` and `continue` go inside a loop.
#[derive(Clone, Copy, Debug)]
pub(crate) struct LoopTargets {
    pub(crate) break_target: BlockId,
    pub(crate) continue_target: BlockId,
    /// How many cleanups were open where the loop starts; `break` and `continue` run those
    /// opened since.
    pub(crate) cleanups: usize,
}

/// What leaving a region of code must do, innermost last (ADR-0013).
#[derive(Clone, Debug)]
pub(crate) enum Cleanup {
    /// A module's code, run by its first import: an exception leaves the module not
    /// started, so the next import runs it again (MOD-03).
    Initialising {
        /// The module's state global.
        state: typy_tyir::GlobalId,
    },
    /// The body of a `try`: an exception raised in it goes to the block that matches it
    /// against the `except` clauses, with nothing to do on the way out otherwise.
    Handlers {
        /// The block, whose one parameter is the exception.
        dispatch: BlockId,
    },
    /// An `except` clause: leaving it restores the exception handled before and drops this
    /// one, as CPython deletes the clause's name.
    Handling {
        /// The exception handled before, as bits.
        previous: ValueId,
        /// Where the clause keeps the exception.
        exception: Handled,
    },
    /// A `try` with a `finally` block: every way out of the `try` runs a copy of the block.
    Finally(std::rc::Rc<[typy_check::TypedStmt]>),
    /// The body of a `with`: every way out calls the manager's `__exit__` (CST-05).
    With {
        /// The slot that holds the context manager.
        manager: SlotId,
        /// How the manager is entered and left: its class's `__exit__`, or a file's `close`.
        protocol: typy_check::WithProtocol,
        /// Whether `__exit__` may return a true value, which suppresses an exception.
        suppresses: bool,
        /// Where execution goes on after an exception is suppressed.
        after: BlockId,
    },
}

/// Where an `except` clause keeps the exception it handles.
#[derive(Clone, Copy, Debug)]
pub(crate) enum Handled {
    /// In the variable its `as` names.
    Slot(SlotId),
    /// In the cell of the variable its `as` names, which a nested function captures.
    Cell(SymbolId),
    /// As a value the clause owns, when it names no variable.
    Value(ValueId),
}

/// Lowers one function. Its slots own their values; owned temporaries are tracked so an error path can release them.
/// The tables of the module that every function's lowering adds to.
pub(crate) struct Tables<'context> {
    pub(crate) strings: &'context mut StringPool,
    pub(crate) tuple_types: &'context mut TupleTypes,
    pub(crate) cdata_classes: &'context mut crate::cdata::CDataClasses,
}

pub(crate) struct FunctionLowerer<'context> {
    pub(crate) builder: FunctionBuilder,
    pub(crate) program: &'context TypedProgram,
    pub(crate) may_raise: &'context [bool],
    pub(crate) methods: &'context crate::methods::MethodTables,
    /// Whether runtime calls are followed by the test of a slot's exception.
    pub(crate) slot_tests: crate::invoke::SlotTests,
    pub(crate) strings: &'context mut StringPool,
    pub(crate) tuple_types: &'context mut TupleTypes,
    /// The `ctypes` classes of the module (FFI-03).
    pub(crate) cdata_classes: &'context mut crate::cdata::CDataClasses,
    pub(crate) slots: BTreeMap<SymbolId, SlotId>,
    pub(crate) temporaries: Vec<ValueId>,
    pub(crate) loops: Vec<LoopTargets>,
    pub(crate) cleanups: Vec<Cleanup>,
    pub(crate) raises: bool,
    /// The slots of parameters the function never assigns, which borrow the caller's
    /// reference and are never counted (MEM-04).
    pub(crate) borrowed: BTreeSet<SlotId>,
    /// The reads of a variable that move its value out of its slot, by the variable and the
    /// read's span (MEM-09).
    pub(crate) moves: BTreeSet<(SymbolId, Span)>,
    /// The owned values a store has taken the reference of, which are not released after it.
    pub(crate) handed_over: BTreeSet<ValueId>,
    parameters: Vec<(ValueId, SlotId)>,
    /// The globals of the program's module variables (ADR-0015).
    pub(crate) module_globals: &'context crate::module_variable::ModuleGlobals,
    /// The qualified name of the module the function belongs to, which `__name__` gives.
    pub(crate) module: String,
    /// Whether this is the program's entry, the code of `__main__`, which first gives every
    /// module variable its zero value.
    entry: bool,
    /// The function being lowered.
    pub(crate) function_id: typy_check::FunctionId,
    /// For a nested function, its function object, its first parameter, and the tuple type
    /// of its captures (FUN-04).
    pub(crate) closure: Option<(ValueId, typy_tyir::TupleTypeId)>,
    /// The variables that live in a cell, which their slots hold, with the cell's layout.
    pub(crate) cells: BTreeMap<SymbolId, typy_tyir::TupleTypeId>,
    recursive: bool,
    body_start: BlockId,
    /// The block the prologue fills: the entry, or a resume function's first step.
    prologue_block: BlockId,
    /// For a generator's resume function, its generator and the blocks after its yields
    /// (ADR-0063).
    pub(crate) generator: Option<crate::generator::Resumable>,
    span: Span,
}

impl<'context> FunctionLowerer<'context> {
    /// Returns true when a value of type `ty` holds a reference that is counted.
    pub(crate) fn counted(&self, ty: TyirType) -> bool {
        self.tuple_types.is_counted(ty)
    }

    /// Returns the TyIR type of a checked type, adding a tuple type to the module when it is new.
    pub(crate) fn lowered_type(&mut self, ty: &typy_check::Type) -> TyirType {
        tyir_type(ty, self.tuple_types)
    }

    /// Returns the number of the tuple type of a tuple of `items`, adding it when it is new.
    pub(crate) fn tuple_type_id(&mut self, items: &[typy_check::Type]) -> typy_tyir::TupleTypeId {
        let items = items
            .iter()
            .map(|item| tyir_type(item, self.tuple_types))
            .collect();
        self.tuple_types.intern_id(items)
    }

    /// Lowers `function`, or for a generator function with `resume`, its resume function
    /// under that ID. `may_raise` says, for each function of the program, whether calls to it
    /// are invokes.
    pub(crate) fn lower(
        program: &'context TypedProgram,
        (function, resume): (&TypedFunction, Option<FunctionId>),
        calls: &crate::methods::Calls<'context>,
        tables: Tables<'context>,
        module_globals: &'context crate::module_variable::ModuleGlobals,
        recursive: bool,
    ) -> TyirFunction {
        let Tables {
            strings,
            tuple_types,
            cdata_classes,
        } = tables;
        if function.generic {
            return generic_stub(function);
        }
        let entry = function.id == program.entry;
        let (mut builder, generator, closure) =
            function_builder(program, (function, resume), tuple_types);
        let prologue_block = generator
            .as_ref()
            .map_or(BlockId(0), |resumable| resumable.start);
        let cell_variables = cell_layouts(program, function, tuple_types);
        let arguments = generator
            .as_ref()
            .map(|resumable: &crate::generator::Resumable| resumable.arguments());
        let Slots {
            of: slots,
            parameters,
            borrowed,
        } = variable_slots(
            (program, function),
            &mut builder,
            (tuple_types, module_globals),
            (&cell_variables, arguments),
        );
        let mut candidates = move_candidates(program, function, (&slots, &borrowed), tuple_types);
        // A cell is read through, never moved out of its slot.
        candidates.retain(|symbol| !cell_variables.contains_key(symbol));
        let moves = crate::last_use::last_uses(&function.body, &candidates);
        let body_start = builder.create_block();
        builder.switch_to(body_start);
        let mut lowerer = FunctionLowerer {
            builder,
            program,
            may_raise: calls.may_raise,
            methods: calls.methods,
            slot_tests: crate::invoke::SlotTests::of(calls.raising_slots),
            strings,
            tuple_types,
            cdata_classes,
            slots,
            temporaries: Vec::new(),
            loops: Vec::new(),
            cleanups: Vec::new(),
            raises: false,
            borrowed,
            moves,
            handed_over: BTreeSet::new(),
            parameters,
            module_globals,
            module: function.module.clone(),
            entry,
            function_id: function.id,
            closure,
            cells: cell_variables,
            recursive,
            body_start,
            prologue_block,
            generator,
            span: function.span,
        };
        lowerer.statements(&function.body);
        lowerer.finish_lazy_import(function.span);
        if !lowerer.builder.is_terminated() {
            // The end of a generator's body ends its last step (FUN-07).
            if function.signature.returns == typy_check::Type::NoneType
                || lowerer.generator.is_some()
            {
                let unit = lowerer.constant(Constant::Unit, function.span);
                lowerer.release_slots(function.span);
                lowerer.builder.terminate(Terminator::Return(unit));
            } else {
                // The checker rejects a function that returns a value and can reach its end
                // (TYP-01), so this point follows only paths that returned or raised.
                lowerer.builder.terminate(Terminator::Unreachable);
            }
        }
        lowerer.prologue();
        if lowerer.generator.is_some() {
            return lowerer.finish_resume();
        }
        // A method that shares a table entry with one that raises is called as one that may
        // raise, so it returns as one (ADR-0021).
        let shared = calls
            .may_raise
            .get(function.id.0 as usize)
            .copied()
            .unwrap_or(false);
        lowerer.builder.set_may_raise(lowerer.raises || shared);
        lowerer.builder.finish()
    }

    /// Adds a constant and returns its value.
    pub(crate) fn constant(&mut self, constant: Constant, span: Span) -> ValueId {
        let ty = constant.ty();
        self.builder
            .push_value(Operation::Constant(constant), ty, span)
    }

    /// Returns the value of a string constant, which is immortal and so borrowed.
    pub(crate) fn string(&mut self, text: &str, span: Span) -> ValueId {
        let id = self.strings.intern(text);
        self.constant(Constant::Str(id), span)
    }

    /// Returns a `bytes` constant, each byte held as the code point of its value.
    pub(crate) fn bytes_constant(&mut self, data: &[u8], span: Span) -> ValueId {
        let text: String = data.iter().copied().map(char::from).collect();
        let id = self.strings.intern(&text);
        self.constant(Constant::Bytes(id), span)
    }

    /// Fills the entry block: every slot gets a value before the body runs, so every error path can release every slot.
    fn prologue(&mut self) {
        let span = self.span;
        self.builder.switch_to(self.prologue_block);
        // A generator's arguments come out of its frame with their references (ADR-0063).
        let owned = self.generator.is_some();
        let parameter_slots: Vec<(ValueId, SlotId)> = self.parameters.clone();
        // Each cell is made first; a parameter's holds the parameter's value, with a
        // reference of its own. A variable declared `nonlocal` holds the cell its function
        // object captures, with a reference of its own (SMT-09).
        let cells: Vec<(SymbolId, SlotId, typy_tyir::TupleTypeId)> = self
            .cells
            .iter()
            .map(|(symbol, layout)| (*symbol, self.slots[symbol], *layout))
            .collect();
        for (symbol, slot, layout) in &cells {
            let cell = if let Some(position) = self.program.nonlocals.get(symbol).copied() {
                let captured = self.capture(position, TyirType::CELL, span);
                self.builder
                    .push(Operation::Increment(captured.value), span);
                captured.value
            } else {
                self.builder.push_value(
                    Operation::NewCell { layout: *layout },
                    TyirType::CELL,
                    span,
                )
            };
            self.builder.push(
                Operation::Store {
                    slot: *slot,
                    value: cell,
                },
                span,
            );
            if let Some((value, _)) = parameter_slots.iter().find(|(_, each)| each == slot) {
                let ty = self
                    .tuple_types
                    .items(*layout)
                    .map_or(TyirType::Unit, |items| items[0]);
                if self.counted(ty) && !owned {
                    self.builder.push(Operation::Increment(*value), span);
                }
                self.builder.push(
                    Operation::CellSet {
                        cell,
                        layout: *layout,
                        value: *value,
                    },
                    span,
                );
            }
        }
        for (value, slot) in &parameter_slots {
            if cells.iter().any(|(_, each, _)| each == slot) {
                continue;
            }
            if self.counted(self.builder.slot_type(*slot))
                && !self.borrowed.contains(slot)
                && !owned
            {
                self.builder.push(Operation::Increment(*value), span);
            }
            self.builder.push(
                Operation::Store {
                    slot: *slot,
                    value: *value,
                },
                span,
            );
        }
        let slot_count = self.builder.slot_count();
        for index in 0..slot_count {
            let slot = SlotId(u32::try_from(index).unwrap_or(u32::MAX));
            if parameter_slots
                .iter()
                .any(|(_, parameter)| *parameter == slot)
                || cells.iter().any(|(_, each, _)| *each == slot)
            {
                continue;
            }
            let ty = self.builder.slot_type(slot);
            let initial = self.initial_value(ty, span);
            if let Some(value) = initial {
                self.builder.push(Operation::Store { slot, value }, span);
            }
        }
        if self.entry {
            self.initialize_module_variables(span);
        }
        if self.recursive {
            self.call_runtime(RuntimeFunction::CheckStack, Vec::new(), span);
        }
        self.builder.terminate(Terminator::Jump {
            target: self.body_start,
            arguments: Vec::new(),
        });
    }
}

/// Returns the variables whose last read may move the value out of their slot: those of a
/// counted type held in a slot of their own, other than a parameter the function borrows and
/// a name an exception handler binds (MEM-09).
fn move_candidates(
    program: &TypedProgram,
    function: &TypedFunction,
    (slots, borrowed): (&BTreeMap<SymbolId, SlotId>, &BTreeSet<SlotId>),
    tuple_types: &mut TupleTypes,
) -> BTreeSet<SymbolId> {
    let excluded = crate::last_use::handler_names(&function.body);
    let mut candidates = BTreeSet::new();
    for (symbol, slot) in slots {
        let ty = &program.symbol(*symbol).ty;
        let lowered = tyir_type(ty, tuple_types);
        if !borrowed.contains(slot)
            && !excluded.contains(symbol)
            && crate::last_use::movable_type(ty)
            && tuple_types.is_counted(lowered)
        {
            candidates.insert(*symbol);
        }
    }
    candidates
}

/// Adds a nested function's first parameter, its function object, which holds its captures,
/// and returns it with the tuple type of the captures (FUN-04).
fn closure_parameter(
    program: &TypedProgram,
    function: &TypedFunction,
    builder: &mut FunctionBuilder,
    tuple_types: &mut typy_tyir::TupleTypes,
) -> Option<(ValueId, typy_tyir::TupleTypeId)> {
    let captures = program.closures.get(&function.id)?;
    let value = builder.add_parameter(TyirType::FUNCTION);
    Some((value, captures_layout(captures, tuple_types)))
}

/// Returns the tuple type of a nested function's captures: each value it copies, and each
/// cell it shares as an object.
pub(crate) fn captures_layout(
    captures: &[typy_check::CapturedVariable],
    tuple_types: &mut typy_tyir::TupleTypes,
) -> typy_tyir::TupleTypeId {
    let items = captures
        .iter()
        .map(|captured| {
            if captured.cell {
                TyirType::CELL
            } else {
                tyir_type(&captured.ty, tuple_types)
            }
        })
        .collect();
    tuple_types.intern_id(items)
}

/// The slot of each variable of a function, its parameters' values with their slots, and the
/// slots of the parameters that keep their caller's reference.
struct Slots {
    of: BTreeMap<SymbolId, SlotId>,
    parameters: Vec<(ValueId, SlotId)>,
    borrowed: BTreeSet<SlotId>,
}

/// Adds a parameter and a slot for each parameter of `function`, and a slot for each of its
/// local variables, but a module variable, which lives in a global. A variable that lives in
/// a cell has a slot of the cell, which the prologue fills (FUN-04); a parameter the function
/// never assigns otherwise keeps its caller's reference (MEM-04).
fn variable_slots(
    (program, function): (&TypedProgram, &TypedFunction),
    builder: &mut FunctionBuilder,
    (tuple_types, module_globals): (
        &mut typy_tyir::TupleTypes,
        &crate::module_variable::ModuleGlobals,
    ),
    (cells, arguments): (
        &BTreeMap<SymbolId, typy_tyir::TupleTypeId>,
        Option<crate::generator::Arguments>,
    ),
) -> Slots {
    let mut assigned = crate::borrow::assigned_symbols(&function.body);
    assigned.extend(function.named_targets.iter().copied());
    let mut made = Slots {
        of: BTreeMap::new(),
        parameters: Vec::new(),
        borrowed: BTreeSet::new(),
    };
    for (position, symbol_id) in function.parameters.iter().enumerate() {
        let symbol = program.symbol(*symbol_id);
        let ty = tyir_type(&symbol.ty, tuple_types);
        let value = match arguments {
            Some(arguments) => arguments.take(builder, position, ty, function.span),
            None => builder.add_parameter(ty),
        };
        let in_cell = cells.contains_key(symbol_id);
        let slot = builder.add_slot(
            symbol.name.clone(),
            if in_cell { TyirType::CELL } else { ty },
        );
        made.of.insert(*symbol_id, slot);
        made.parameters.push((value, slot));
        // A generator owns its arguments, which outlive the call that made it.
        if !assigned.contains(symbol_id) && !in_cell && arguments.is_none() {
            made.borrowed.insert(slot);
        }
    }
    for symbol_id in &function.locals {
        if module_globals.of.contains_key(symbol_id) {
            continue;
        }
        let symbol = program.symbol(*symbol_id);
        let ty = if cells.contains_key(symbol_id) {
            TyirType::CELL
        } else {
            tyir_type(&symbol.ty, tuple_types)
        };
        made.of
            .insert(*symbol_id, builder.add_slot(symbol.name.clone(), ty));
    }
    made
}

/// Returns the variables of `function` that live in a cell, each with its cell's layout: its
/// value, then whether it is assigned (FUN-04).
fn cell_layouts(
    program: &TypedProgram,
    function: &TypedFunction,
    tuple_types: &mut typy_tyir::TupleTypes,
) -> BTreeMap<SymbolId, typy_tyir::TupleTypeId> {
    program
        .cells
        .get(&function.id)
        .into_iter()
        .flatten()
        .map(|symbol| {
            let ty = tyir_type(&program.symbol(*symbol).ty, tuple_types);
            (*symbol, tuple_types.intern_id(vec![ty, TyirType::Bool]))
        })
        .collect()
}

/// Returns the builder of `function`, or of its resume function `resume`, with the lowering's
/// state of a resume function and the closure of a nested function with the layout of its
/// captures.
fn function_builder(
    program: &TypedProgram,
    (function, resume): (&TypedFunction, Option<FunctionId>),
    tuple_types: &mut TupleTypes,
) -> (
    FunctionBuilder,
    Option<crate::generator::Resumable>,
    Option<(ValueId, typy_tyir::TupleTypeId)>,
) {
    let (mut builder, generator, closure) = if let Some(resume) = resume {
        crate::generator::resume_builder(program, function, resume, tuple_types)
    } else {
        let return_type = tyir_type(&function.signature.returns, tuple_types);
        let mut builder = FunctionBuilder::new(
            FunctionId(function.id.0),
            function_symbol(program, function),
            function.name.clone(),
            return_type,
        );
        let closure = closure_parameter(program, function, &mut builder, tuple_types);
        (builder, None, closure)
    };
    builder.set_qualified_name(function.qualified_name.clone());
    (builder, generator, closure)
}

/// Returns the symbol `function` is emitted under.
pub(crate) fn function_symbol(program: &TypedProgram, function: &TypedFunction) -> String {
    if function.id == program.entry {
        ENTRY_SYMBOL.to_owned()
    } else if function.name == "<lambda>" {
        // The lambdas of one function share a `__qualname__`, so each symbol adds the
        // lambda's number.
        symbol(
            &function.module,
            &format!("{}.{}", function.qualified_name, function.id.0),
        )
    } else {
        symbol(&function.module, &function.qualified_name)
    }
}

/// Returns the stand-in for a generic function, whose copies are called in its place: it
/// takes nothing and returns nothing, and no call reaches it (ADR-0023).
fn generic_stub(function: &TypedFunction) -> TyirFunction {
    let mut builder = FunctionBuilder::new(
        FunctionId(function.id.0),
        symbol(&function.module, &function.qualified_name),
        function.name.clone(),
        TyirType::Unit,
    );
    let unit = builder.push_value(
        typy_tyir::Operation::Constant(typy_tyir::Constant::Unit),
        TyirType::Unit,
        function.span,
    );
    builder.terminate(typy_tyir::Terminator::Return(unit));
    builder.finish()
}
