//! Emits the descriptor of each tuple type a list holds: the size of an item, the number of its
//! items, and each item's kind and byte offset, which the runtime reads to store, count, compare
//! and write the tuples (ADR-0009). The offsets come from the target's own data layout, so the
//! runtime reads each field where the generated code stores it.

use inkwell::AddressSpace;
use inkwell::targets::TargetData;
use inkwell::values::{BasicValue, PointerValue};
use typy_tyir::{TupleTypeId, TyirType};

use crate::abi::{tuple_field, value_type};
use crate::error::CodegenError;
use crate::function::Shared;
use crate::statics::Statics;

/// The kind codes of the runtime's `ItemKind`, one for each item type a tuple field can have.
fn kind_code(item: TyirType) -> Result<u64, CodegenError> {
    Ok(match item {
        TyirType::Int => 0,
        TyirType::Float => 1,
        TyirType::Bool => 2,
        TyirType::Object(_) => 3,
        TyirType::Unit => 4,
        TyirType::Tuple(_) => 5,
        TyirType::Optional(base) => match base {
            typy_tyir::OptionalBase::Int => 6,
            typy_tyir::OptionalBase::Float => 7,
            typy_tyir::OptionalBase::Bool => 8,
            typy_tyir::OptionalBase::Object(_) => 9,
            typy_tyir::OptionalBase::Tuple(_) => 10,
        },
        TyirType::Bits => return Err(CodegenError("bits are not a tuple item".to_owned())),
    })
}

/// Returns the address of the descriptor of tuple type `id`, emitting it, and the descriptors
/// of the tuples nested in it, on first use.
pub(crate) fn tuple_descriptor<'ctx>(
    shared: &mut Shared<'_, 'ctx>,
    id: TupleTypeId,
) -> Result<PointerValue<'ctx>, CodegenError> {
    if let Some(pointer) = shared.statics.descriptor(id.0) {
        return Ok(pointer);
    }
    let context = shared.context;
    let tuples = &shared.tyir.tuple_types;
    let items = tuples
        .items(id)
        .ok_or_else(|| CodegenError(format!("tuple#{} is not a type of the module", id.0)))?
        .to_vec();
    let struct_type = value_type(context, tuples, TyirType::Tuple(id))
        .ok_or_else(|| CodegenError("a tuple has a type".to_owned()))?
        .into_struct_type();
    let layout = shared.module.get_data_layout();
    let data = TargetData::create(&layout.as_str().to_string_lossy());
    let size = data.get_abi_size(&struct_type).max(8).next_multiple_of(8);
    let (word, half) = (context.i64_type(), context.i32_type());
    let pointer = context.ptr_type(AddressSpace::default());
    let field_type = context.struct_type(&[half.into(), half.into(), pointer.into()], false);
    let mut fields = Vec::with_capacity(items.len());
    for (index, item) in items.iter().enumerate() {
        let nested = match item {
            TyirType::Tuple(inner) => tuple_descriptor(shared, *inner)?,
            _ => pointer.const_null(),
        };
        let offset = tuple_field(&items, index)
            .and_then(|field| data.offset_of_element(&struct_type, field))
            .unwrap_or(0);
        fields.push(field_type.const_named_struct(&[
            half.const_int(kind_code(*item)?, false).into(),
            half.const_int(offset, false).into(),
            nested.into(),
        ]));
    }
    let table = Statics::global(
        shared.module,
        field_type.const_array(&fields).as_basic_value_enum(),
        &format!("typy.tuple{}.fields", id.0),
    );
    let length = u64::try_from(items.len()).unwrap_or(u64::MAX);
    let header = context.const_struct(
        &[
            word.const_int(size, false).into(),
            word.const_int(length, false).into(),
            table.into(),
        ],
        false,
    );
    let descriptor = Statics::global(
        shared.module,
        header.as_basic_value_enum(),
        &format!("typy.tuple{}", id.0),
    );
    shared.statics.add_descriptor(id.0, descriptor);
    Ok(descriptor)
}
