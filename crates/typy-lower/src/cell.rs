//! Lowers cells (FUN-04, ADR-0061): a variable that a nested function captures and that may
//! change after the `def`, or have no value yet, lives in a cell, which its slot holds and
//! each function that captures it shares.

use typy_check::SymbolId;
use typy_diagnostics::Span;
use typy_tyir::{Operation, RuntimeFunction, Terminator, TupleTypeId, TyirType};

use crate::function::FunctionLowerer;
use crate::ownership::Lowered;

impl FunctionLowerer<'_> {
    /// Returns the layout of a cell that holds a value of type `ty`: the value, then whether
    /// it is assigned.
    pub(crate) fn cell_layout(&mut self, ty: TyirType) -> TupleTypeId {
        self.tuple_types.intern_id(vec![ty, TyirType::Bool])
    }

    /// Returns the cell of the variable `symbol`, borrowed from its slot, with its layout.
    pub(crate) fn cell_of(&mut self, symbol: SymbolId, span: Span) -> (Lowered, TupleTypeId) {
        let layout = self.cells[&symbol];
        let slot = self.slots[&symbol];
        (self.load(slot, span), layout)
    }

    /// Returns the value the cell `cell` holds, of type `ty`, borrowed from it.
    pub(crate) fn cell_value(
        &mut self,
        cell: Lowered,
        layout: TupleTypeId,
        ty: TyirType,
        span: Span,
    ) -> Lowered {
        let value = self.builder.push_value(
            Operation::CellGet {
                cell: cell.value,
                layout,
            },
            ty,
            span,
        );
        Self::borrowed(value, ty)
    }

    /// Stores `value` in the cell of the variable `symbol`, which takes over its reference,
    /// and releases the value the cell held, which is zero when it held none.
    pub(crate) fn assign_cell(&mut self, symbol: SymbolId, value: Lowered, span: Span) {
        let (cell, layout) = self.cell_of(symbol, span);
        let ty = self
            .tuple_types
            .items(layout)
            .map_or(value.ty, |items| items[0]);
        let value = self.held_as(value, ty, span);
        let old = self.cell_value(cell, layout, ty, span);
        let stored = self.take_reference(value, span);
        self.builder.push(
            Operation::CellSet {
                cell: cell.value,
                layout,
                value: stored,
            },
            span,
        );
        if self.counted(ty) {
            self.builder.push(Operation::Decrement(old.value), span);
        }
    }

    /// Leaves the cell of the variable `symbol` with no value, as the end of an `except`
    /// clause leaves its variable, and releases the value it held.
    pub(crate) fn clear_cell(&mut self, symbol: SymbolId, span: Span) {
        let (cell, layout) = self.cell_of(symbol, span);
        let ty = self
            .tuple_types
            .items(layout)
            .map_or(TyirType::Unit, |items| items[0]);
        let old = self.cell_value(cell, layout, ty, span);
        self.builder.push(
            Operation::CellClear {
                cell: cell.value,
                layout,
            },
            span,
        );
        if self.counted(ty) {
            self.builder.push(Operation::Decrement(old.value), span);
        }
    }

    /// Returns the name of the variable the running nested function captures at `index`.
    pub(crate) fn captured_name(&self, index: u32) -> String {
        self.program
            .closures
            .get(&self.function_id)
            .and_then(|captures| captures.get(index as usize))
            .map(|captured| captured.name.clone())
            .unwrap_or_default()
    }

    /// Returns the value of the captured cell at `index` of the running nested function, of
    /// type `ty`, borrowed; a cell with no value yet raises `NameError` with the variable's
    /// `name`, as CPython does.
    pub(crate) fn captured_cell(
        &mut self,
        index: u32,
        name: &str,
        ty: TyirType,
        span: Span,
    ) -> Lowered {
        let cell = self.capture(index, TyirType::CELL, span);
        let layout = self.cell_layout(ty);
        let bound = self.builder.push_value(
            Operation::CellBound {
                cell: cell.value,
                layout,
            },
            TyirType::Bool,
            span,
        );
        let read = self.builder.create_block();
        let unbound = self.builder.create_block();
        self.builder.terminate(Terminator::Branch {
            condition: bound,
            then: read,
            otherwise: unbound,
        });
        self.builder.switch_to(unbound);
        let text = self.string(name, span);
        self.call_runtime(RuntimeFunction::FreeVariableUnbound, vec![text], span);
        self.builder.terminate(Terminator::Unreachable);
        self.builder.switch_to(read);
        self.cell_value(cell, layout, ty, span)
    }
}
