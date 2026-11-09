//! Lowers one function, or the module body, and writes its prologue.

use std::collections::BTreeMap;

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
}

/// Lowers one function. Its slots own their values; owned temporaries are tracked so an error path can release them.
pub(crate) struct FunctionLowerer<'context> {
    pub(crate) builder: FunctionBuilder,
    pub(crate) program: &'context TypedProgram,
    pub(crate) may_raise: &'context [bool],
    pub(crate) strings: &'context mut StringPool,
    pub(crate) tuple_types: &'context mut TupleTypes,
    pub(crate) slots: BTreeMap<SymbolId, SlotId>,
    pub(crate) temporaries: Vec<ValueId>,
    pub(crate) loops: Vec<LoopTargets>,
    pub(crate) raises: bool,
    parameters: Vec<(ValueId, SlotId)>,
    recursive: bool,
    body_start: BlockId,
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

    /// Lowers `function`. `may_raise` says, for each function of the program, whether calls to it are invokes.
    pub(crate) fn lower(
        program: &'context TypedProgram,
        function: &TypedFunction,
        may_raise: &'context [bool],
        strings: &'context mut StringPool,
        tuple_types: &'context mut TupleTypes,
        recursive: bool,
    ) -> TyirFunction {
        let entry = function.id == program.entry;
        let symbol_name = if entry {
            ENTRY_SYMBOL.to_owned()
        } else {
            symbol(&function.module, &function.name)
        };
        let return_type = tyir_type(&function.signature.returns, tuple_types);
        let mut builder = FunctionBuilder::new(
            FunctionId(function.id.0),
            symbol_name,
            function.name.clone(),
            return_type,
        );
        let mut slots = BTreeMap::new();
        let mut parameters = Vec::new();
        for symbol_id in &function.parameters {
            let symbol = program.symbol(*symbol_id);
            let ty = tyir_type(&symbol.ty, tuple_types);
            let value = builder.add_parameter(ty);
            let slot = builder.add_slot(symbol.name.clone(), ty);
            slots.insert(*symbol_id, slot);
            parameters.push((value, slot));
        }
        for symbol_id in &function.locals {
            let symbol = program.symbol(*symbol_id);
            let slot = builder.add_slot(symbol.name.clone(), tyir_type(&symbol.ty, tuple_types));
            slots.insert(*symbol_id, slot);
        }
        let body_start = builder.create_block();
        builder.switch_to(body_start);
        let mut lowerer = FunctionLowerer {
            builder,
            program,
            may_raise,
            strings,
            tuple_types,
            slots,
            temporaries: Vec::new(),
            loops: Vec::new(),
            raises: false,
            parameters,
            recursive,
            body_start,
            span: function.span,
        };
        lowerer.statements(&function.body);
        if !lowerer.builder.is_terminated() {
            let unit = lowerer.constant(Constant::Unit, function.span);
            lowerer.release_slots(function.span);
            lowerer.builder.terminate(Terminator::Return(unit));
        }
        lowerer.prologue();
        lowerer.builder.set_may_raise(lowerer.raises);
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
        self.builder.switch_to(BlockId(0));
        let parameter_slots: Vec<(ValueId, SlotId)> = self.parameters.clone();
        for (value, slot) in &parameter_slots {
            if self.counted(self.builder.slot_type(*slot)) {
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
            {
                continue;
            }
            let ty = self.builder.slot_type(slot);
            let initial = self.initial_value(ty, span);
            if let Some(value) = initial {
                self.builder.push(Operation::Store { slot, value }, span);
            }
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
