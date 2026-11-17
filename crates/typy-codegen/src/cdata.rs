//! Emits the `ctypes` classes a program uses: one constant for each, in the layout of the
//! runtime's `CDataClass`, which every object of the class points to (FFI-03, ADR-0048).

use inkwell::AddressSpace;
use inkwell::values::{BasicValue, PointerValue};
use typy_tyir::{CDataShape, CType};

use crate::error::CodegenError;
use crate::function::Shared;
use crate::statics::Statics;

/// The runtime's `CDataClass::kind` of each shape.
const SCALAR: u64 = 0;
const POINTER: u64 = 1;
const ARRAY: u64 = 2;

/// Returns the runtime's code for the C type of a scalar class.
fn ctype_code(ty: CType) -> Result<u64, CodegenError> {
    Ok(match ty {
        CType::Int8 => 0,
        CType::UInt8 => 1,
        CType::Int16 => 2,
        CType::UInt16 => 3,
        CType::Int32 => 4,
        CType::UInt32 => 5,
        CType::Int64 => 6,
        CType::UInt64 => 7,
        CType::Float => 8,
        CType::Double => 9,
        CType::Bool => 10,
        CType::CharPointer => 11,
        CType::VoidPointer => 12,
        CType::Pointer(_) => {
            return Err(CodegenError(
                "a pointer to a class is a class of its own, not a scalar".to_owned(),
            ));
        }
    })
}

/// Returns the address of the class at `index` of the module's `ctypes` classes, emitting it
/// and the class of its item the first time.
pub(crate) fn cdata_class<'ctx>(
    shared: &Shared<'_, 'ctx>,
    index: u32,
) -> Result<PointerValue<'ctx>, CodegenError> {
    let name = format!("typy.cdata.{index}");
    if let Some(global) = shared.module.get_global(&name) {
        return Ok(global.as_pointer_value());
    }
    let class = shared
        .tyir
        .cdata_classes
        .get(index as usize)
        .ok_or_else(|| CodegenError(format!("cdata class#{index} does not exist")))?
        .clone();
    let context = shared.context;
    let (word, half) = (context.i64_type(), context.i32_type());
    let pointer = context.ptr_type(AddressSpace::default());
    let (kind, ctype, item, length) = match class.shape {
        CDataShape::Scalar(ty) => (SCALAR, ctype_code(ty)?, pointer.const_null(), 0),
        CDataShape::Pointer(item) => (POINTER, 0, cdata_class(shared, item)?, 0),
        CDataShape::Array(item, length) => (ARRAY, 0, cdata_class(shared, item)?, length),
    };
    let text = context.const_string(class.name.as_bytes(), false);
    let text = Statics::global(
        shared.module,
        text.as_basic_value_enum(),
        &format!("{name}.name"),
    );
    let value = context.const_struct(
        &[
            text.into(),
            word.const_int(class.name.len() as u64, false).into(),
            half.const_int(kind, false).into(),
            half.const_int(ctype, false).into(),
            word.const_int(class.size, false).into(),
            item.into(),
            word.const_int(length, false).into(),
        ],
        false,
    );
    Ok(Statics::global(
        shared.module,
        value.as_basic_value_enum(),
        &name,
    ))
}
