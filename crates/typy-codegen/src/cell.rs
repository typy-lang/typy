//! Emits cells (FUN-04, ADR-0061): an object of the header, the descriptor of its layout and
//! the layout, a value and whether it is assigned, which the runtime makes with every byte
//! zero and frees by releasing the value through the descriptor.

use inkwell::AddressSpace;
use inkwell::types::BasicTypeEnum;
use inkwell::values::{BasicValueEnum, PointerValue};
use typy_tyir::{TupleTypeId, TyirType, ValueId};

use crate::abi::{tuple_field, value_type};
use crate::error::CodegenError;
use crate::function::FunctionEmitter;

/// The field of a cell that holds its layout, after the header and the descriptor.
const CELL_PAYLOAD: u32 = 2;
/// The position of the value in a cell's layout.
const VALUE: usize = 0;
/// The position of the flag that says the value is assigned.
const ASSIGNED: usize = 1;

impl<'ctx> FunctionEmitter<'_, '_, 'ctx> {
    /// Returns a new cell of `layout` with no value, from the runtime.
    pub(crate) fn new_cell(
        &mut self,
        layout: TupleTypeId,
    ) -> Result<BasicValueEnum<'ctx>, CodegenError> {
        let descriptor = crate::descriptor::tuple_descriptor(self.shared, layout)?;
        let pointer = self.shared.context.ptr_type(AddressSpace::default());
        let allocate = crate::class::runtime_symbol(
            self.shared,
            "typy_rt_cell_new",
            pointer.fn_type(&[pointer.into()], false),
        );
        self.builder
            .build_call(allocate, &[descriptor.into()], "")?
            .try_as_basic_value()
            .basic()
            .ok_or_else(|| CodegenError("the runtime returns the cell".to_owned()))
    }

    /// Returns the address of the part `position` of the cell `cell` of `layout`, with its
    /// type, or `None` for a value of type `None`, which takes no room.
    fn cell_part(
        &mut self,
        cell: ValueId,
        layout: TupleTypeId,
        position: usize,
    ) -> Result<Option<(PointerValue<'ctx>, BasicTypeEnum<'ctx>)>, CodegenError> {
        let context = self.shared.context;
        let tuples = &self.shared.tyir.tuple_types;
        let items = tuples.items(layout).unwrap_or_default().to_vec();
        let Some(field) = tuple_field(tuples, &items, position) else {
            return Ok(None);
        };
        let payload = value_type(context, tuples, TyirType::Tuple(layout))
            .ok_or_else(|| CodegenError("a cell's layout has a type".to_owned()))?
            .into_struct_type();
        let pointer = context.ptr_type(AddressSpace::default());
        let object = context.struct_type(
            &[context.i64_type().into(), pointer.into(), payload.into()],
            false,
        );
        let part_type = payload
            .get_field_type_at_index(field)
            .ok_or_else(|| CodegenError(format!("a cell's part {position} has a type")))?;
        let cell = self.value(cell)?.into_pointer_value();
        let all = self
            .builder
            .build_struct_gep(object, cell, CELL_PAYLOAD, "")?;
        let address = self.builder.build_struct_gep(payload, all, field, "")?;
        Ok(Some((address, part_type)))
    }

    /// Returns the value a cell holds, borrowed from it; zero when it holds none.
    pub(crate) fn cell_get(
        &mut self,
        cell: ValueId,
        layout: TupleTypeId,
    ) -> Result<Option<BasicValueEnum<'ctx>>, CodegenError> {
        match self.cell_part(cell, layout, VALUE)? {
            Some((address, ty)) => Ok(Some(self.builder.build_load(ty, address, "")?)),
            None => Ok(None),
        }
    }

    /// Returns whether a cell holds a value.
    pub(crate) fn cell_bound(
        &mut self,
        cell: ValueId,
        layout: TupleTypeId,
    ) -> Result<BasicValueEnum<'ctx>, CodegenError> {
        let (address, ty) = self
            .cell_part(cell, layout, ASSIGNED)?
            .ok_or_else(|| CodegenError("a cell has its flag".to_owned()))?;
        Ok(self.builder.build_load(ty, address, "")?)
    }

    /// Stores `value` in a cell, or with `None` leaves it with no value and its value zero.
    pub(crate) fn cell_set(
        &mut self,
        cell: ValueId,
        layout: TupleTypeId,
        value: Option<ValueId>,
    ) -> Result<(), CodegenError> {
        if let Some((address, ty)) = self.cell_part(cell, layout, VALUE)? {
            let stored = match value {
                Some(value) => self.value(value)?,
                None => ty.const_zero(),
            };
            self.builder.build_store(address, stored)?;
        }
        let (address, _) = self
            .cell_part(cell, layout, ASSIGNED)?
            .ok_or_else(|| CodegenError("a cell has its flag".to_owned()))?;
        let assigned = self
            .shared
            .context
            .bool_type()
            .const_int(u64::from(value.is_some()), false);
        self.builder.build_store(address, assigned)?;
        Ok(())
    }
}
