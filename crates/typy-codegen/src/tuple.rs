//! Emits tuples: building one from its items, reading an item, and counting the items a tuple
//! holds (DAT-08, ADR-0001).

use inkwell::values::{BasicValue, BasicValueEnum};
use typy_tyir::{TupleTypeId, TyirType, ValueId};

use crate::abi::{tuple_field, value_type};
use crate::error::CodegenError;
use crate::function::FunctionEmitter;

impl<'ctx> FunctionEmitter<'_, '_, 'ctx> {
    fn tuple_items(&self, id: TupleTypeId) -> Result<&[TyirType], CodegenError> {
        self.shared
            .tyir
            .tuple_types
            .items(id)
            .ok_or_else(|| CodegenError(format!("tuple#{} is not a type of the module", id.0)))
    }

    /// Returns a tuple holding `items`, field by field; an item of type `None` takes no field.
    pub(crate) fn make_tuple(
        &mut self,
        id: TupleTypeId,
        items: &[ValueId],
    ) -> Result<BasicValueEnum<'ctx>, CodegenError> {
        let types = self.tuple_items(id)?.to_vec();
        let struct_type = value_type(
            self.shared.context,
            &self.shared.tyir.tuple_types,
            TyirType::Tuple(id),
        )
        .ok_or_else(|| CodegenError("a tuple has a type".to_owned()))?
        .into_struct_type();
        let mut tuple = struct_type.get_undef();
        for (index, item) in items.iter().enumerate() {
            let Some(field) = tuple_field(&types, index) else {
                continue;
            };
            let value = self.value(*item)?;
            tuple = self
                .builder
                .build_insert_value(tuple, value, field, "")?
                .into_struct_value();
        }
        Ok(tuple.as_basic_value_enum())
    }

    /// Returns the item at `index` of `tuple`, or `None` for an item of type `None`.
    pub(crate) fn tuple_item(
        &mut self,
        tuple: ValueId,
        index: u32,
    ) -> Result<Option<BasicValueEnum<'ctx>>, CodegenError> {
        let TyirType::Tuple(id) = self.function.value(tuple).ty else {
            return Err(CodegenError(format!("v{} is not a tuple", tuple.0)));
        };
        let types = self.tuple_items(id)?.to_vec();
        let Some(field) = tuple_field(&types, index as usize) else {
            return Ok(None);
        };
        let value = self.value(tuple)?.into_struct_value();
        Ok(Some(self.builder.build_extract_value(value, field, "")?))
    }

    /// Adds or drops a reference to each counted item of a tuple, nested tuples included. A
    /// counted item always has a field, because only an item of type `None` has none.
    pub(crate) fn count_tuple(
        &mut self,
        id: TupleTypeId,
        tuple: BasicValueEnum<'ctx>,
        increment: bool,
    ) -> Result<(), CodegenError> {
        let types = self.tuple_items(id)?.to_vec();
        let counted: Vec<(u32, TyirType)> = types
            .iter()
            .enumerate()
            .filter(|(_, item)| self.shared.tyir.tuple_types.is_counted(**item))
            .filter_map(|(index, item)| tuple_field(&types, index).map(|field| (field, *item)))
            .collect();
        for (field, item) in counted {
            let value = self
                .builder
                .build_extract_value(tuple.into_struct_value(), field, "")?;
            self.count_value(item, value, increment)?;
        }
        Ok(())
    }
}
