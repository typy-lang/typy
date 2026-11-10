//! Emits classes: the type object of each class, the function that writes an instance for
//! its `__repr__`, and making instances and reading and writing their attributes (ADR-0012,
//! CLS-01, CLS-08).

use inkwell::AddressSpace;
use inkwell::IntPredicate;
use inkwell::module::Linkage;
use inkwell::types::{BasicTypeEnum, StructType};
use inkwell::values::{BasicValue, BasicValueEnum, FunctionValue, PointerValue};
use typy_tyir::{TyirType, ValueId};

use crate::abi::{function_type, tuple_field, value_type};
use crate::descriptor::tuple_descriptor;
use crate::error::CodegenError;
use crate::function::{FunctionEmitter, Shared};
use crate::layout::{IMMORTAL, TYPE_TYPE_ID};
use crate::statics::Statics;

/// Returns a function the runtime defines, declared on first use.
fn runtime_symbol<'ctx>(
    shared: &Shared<'_, 'ctx>,
    name: &str,
    ty: inkwell::types::FunctionType<'ctx>,
) -> FunctionValue<'ctx> {
    shared.module.get_function(name).unwrap_or_else(|| {
        shared
            .module
            .add_function(name, ty, Some(Linkage::External))
    })
}

/// Returns the address of a static the runtime exports, such as a type object, declared on
/// first use.
pub(crate) fn runtime_global<'ctx>(shared: &Shared<'_, 'ctx>, name: &str) -> PointerValue<'ctx> {
    shared.module.get_global(name).map_or_else(
        || {
            let global = shared
                .module
                .add_global(shared.context.i8_type(), None, name);
            global.set_linkage(Linkage::External);
            global.as_pointer_value()
        },
        inkwell::values::GlobalValue::as_pointer_value,
    )
}

/// Returns the type of the functions that write an object for `repr`: the object in, then a
/// string and an exception out.
fn repr_type<'ctx>(shared: &Shared<'_, 'ctx>) -> inkwell::types::FunctionType<'ctx> {
    function_type(
        shared.context,
        &shared.tyir.tuple_types,
        &[TyirType::STR],
        TyirType::STR,
        true,
        true,
    )
}

/// Returns a constant array of the UTF-8 bytes of `text`, and its length.
fn text_global<'ctx>(
    shared: &Shared<'_, 'ctx>,
    text: &str,
    name: &str,
) -> (PointerValue<'ctx>, u64) {
    let bytes = shared.context.const_string(text.as_bytes(), false);
    let pointer = Statics::global(shared.module, bytes.as_basic_value_enum(), name);
    (pointer, text.len() as u64)
}

/// Returns the function the type object of class `index` names to write an instance for
/// `repr`: one that calls the class's `__repr__`, or the runtime's default.
fn repr_function<'ctx>(
    shared: &Shared<'_, 'ctx>,
    index: u32,
) -> Result<FunctionValue<'ctx>, CodegenError> {
    match shared.tyir.classes[index as usize].repr {
        Some(method) => method_writer(shared, index, method, "repr"),
        None => Ok(runtime_symbol(
            shared,
            "typy_rt_instance_repr",
            repr_type(shared),
        )),
    }
}

/// Returns the function the type object of class `index` names to write an instance for
/// `str`, one that calls the class's `__str__`, or null when it has none and `str` is `repr`.
fn str_function<'ctx>(
    shared: &Shared<'_, 'ctx>,
    index: u32,
) -> Result<PointerValue<'ctx>, CodegenError> {
    match shared.tyir.classes[index as usize].str {
        Some(method) => Ok(method_writer(shared, index, method, "str")?
            .as_global_value()
            .as_pointer_value()),
        None => Ok(shared
            .context
            .ptr_type(AddressSpace::default())
            .const_null()),
    }
}

/// Returns a function of the writer type that calls `method`, `__repr__` or `__str__` of
/// class `index`, and adds a null exception to its string when the method cannot raise.
fn method_writer<'ctx>(
    shared: &Shared<'_, 'ctx>,
    index: u32,
    method: typy_tyir::FunctionId,
    slot: &str,
) -> Result<FunctionValue<'ctx>, CodegenError> {
    let ty = repr_type(shared);
    let function = shared.module.add_function(
        &format!("typy.class{index}.{slot}"),
        ty,
        Some(Linkage::Private),
    );
    let builder = shared.context.create_builder();
    builder.position_at_end(shared.context.append_basic_block(function, ""));
    let instance = function
        .get_first_param()
        .ok_or_else(|| CodegenError("the writer takes the instance".to_owned()))?;
    let callee = shared.functions[method.0 as usize];
    let result = builder
        .build_call(callee, &[instance.into()], "")?
        .try_as_basic_value()
        .basic()
        .ok_or_else(|| CodegenError("the method returns a string".to_owned()))?;
    let result = if shared.raising[method.0 as usize] {
        result
    } else {
        // The method cannot raise, so the writer adds a null exception to its string.
        let pair = ty
            .get_return_type()
            .ok_or_else(|| CodegenError("the writer returns a pair".to_owned()))?
            .into_struct_type();
        let pointer = shared.context.ptr_type(AddressSpace::default());
        let first = builder.build_insert_value(pair.get_undef(), result, 0, "")?;
        builder
            .build_insert_value(first, pointer.const_null(), 1, "")?
            .as_basic_value_enum()
    };
    builder.build_return(Some(&result))?;
    Ok(function)
}

/// Returns the address of the type object of class `index`, emitting it on first use: an
/// immortal object of type `type`, whose base is the runtime's `object`.
pub(crate) fn type_object<'ctx>(
    shared: &mut Shared<'_, 'ctx>,
    index: u32,
) -> Result<PointerValue<'ctx>, CodegenError> {
    if let Some(pointer) = shared.statics.type_object(index) {
        return Ok(pointer);
    }
    let class = &shared.tyir.classes[index as usize];
    let fields = tuple_descriptor(shared, class.fields)?;
    let context = shared.context;
    let (half, word) = (context.i32_type(), context.i64_type());
    let pointer = context.ptr_type(AddressSpace::default());
    let (name, name_length) = text_global(shared, &class.name, &format!("typy.class{index}.name"));
    let (module, module_length) =
        text_global(shared, &class.module, &format!("typy.class{index}.module"));
    let base = shared.module.get_global("typy_rt_object").map_or_else(
        || {
            let global = shared
                .module
                .add_global(context.i8_type(), None, "typy_rt_object");
            global.set_linkage(Linkage::External);
            global.as_pointer_value()
        },
        inkwell::values::GlobalValue::as_pointer_value,
    );
    let hash = runtime_symbol(
        shared,
        "typy_rt_identity_hash",
        word.fn_type(&[pointer.into()], false),
    );
    let repr = repr_function(shared, index)?;
    let str = str_function(shared, index)?;
    let header = context.const_struct(
        &[
            half.const_int(u64::from(IMMORTAL), false).into(),
            half.const_int(u64::from(TYPE_TYPE_ID), false).into(),
        ],
        false,
    );
    let null = pointer.const_null();
    let object = context.const_struct(
        &[
            header.into(),
            name.into(),
            word.const_int(name_length, false).into(),
            base.into(),
            null.into(),
            hash.as_global_value().as_pointer_value().into(),
            null.into(),
            null.into(),
            repr.as_global_value().as_pointer_value().into(),
            module.into(),
            word.const_int(module_length, false).into(),
            fields.into(),
            str.into(),
        ],
        false,
    );
    let global = Statics::global(
        shared.module,
        object.as_basic_value_enum(),
        &format!("typy.class{index}"),
    );
    shared.statics.add_type_object(index, global);
    Ok(global)
}

impl<'ctx> FunctionEmitter<'_, '_, 'ctx> {
    /// Returns the layout of an instance of the class of `object`: its header, its class, then
    /// its attributes as a tuple, with the attributes' types.
    fn instance_layout(
        &self,
        object: ValueId,
    ) -> Result<(StructType<'ctx>, Vec<TyirType>), CodegenError> {
        let ty = self.function.value(object).ty;
        let class = match ty {
            TyirType::Object(id) => self.shared.tyir.class(id),
            _ => None,
        }
        .ok_or_else(|| CodegenError(format!("v{} is not an instance", object.0)))?;
        let tuples = &self.shared.tyir.tuple_types;
        let items = tuples.items(class.fields).unwrap_or_default().to_vec();
        let fields = value_type(self.shared.context, tuples, TyirType::Tuple(class.fields))
            .ok_or_else(|| CodegenError("attributes have a type".to_owned()))?;
        let context = self.shared.context;
        let layout = context.struct_type(
            &[
                context.i64_type().into(),
                context.ptr_type(AddressSpace::default()).into(),
                fields,
            ],
            false,
        );
        Ok((layout, items))
    }

    /// Returns the address of the attribute at `index` of `object`, with its type, or `None`
    /// for an attribute of type `None`, which takes no room.
    fn field_address(
        &self,
        object: ValueId,
        index: u32,
    ) -> Result<Option<(PointerValue<'ctx>, BasicTypeEnum<'ctx>)>, CodegenError> {
        let (layout, items) = self.instance_layout(object)?;
        let Some(field) = tuple_field(&items, index as usize) else {
            return Ok(None);
        };
        let fields = layout
            .get_field_type_at_index(2)
            .ok_or_else(|| CodegenError("an instance has attributes".to_owned()))?
            .into_struct_type();
        let field_type = fields
            .get_field_type_at_index(field)
            .ok_or_else(|| CodegenError(format!("attribute {index} has a type")))?;
        let pointer = self.value(object)?.into_pointer_value();
        let attributes = self.builder.build_struct_gep(layout, pointer, 2, "")?;
        let address = self
            .builder
            .build_struct_gep(fields, attributes, field, "")?;
        Ok(Some((address, field_type)))
    }

    /// Returns a new instance of class `class`.
    pub(crate) fn new_instance(
        &mut self,
        class: u32,
    ) -> Result<BasicValueEnum<'ctx>, CodegenError> {
        let type_object = type_object(self.shared, class)?;
        let pointer = self.shared.context.ptr_type(AddressSpace::default());
        let allocate = runtime_symbol(
            self.shared,
            "typy_rt_instance_new",
            pointer.fn_type(&[pointer.into()], false),
        );
        self.builder
            .build_call(allocate, &[type_object.into()], "")?
            .try_as_basic_value()
            .basic()
            .ok_or_else(|| CodegenError("the runtime returns the instance".to_owned()))
    }

    /// Returns the attribute at `index` of `object`, or `None` for an attribute of type `None`.
    pub(crate) fn field(
        &mut self,
        object: ValueId,
        index: u32,
    ) -> Result<Option<BasicValueEnum<'ctx>>, CodegenError> {
        let Some((address, ty)) = self.field_address(object, index)? else {
            return Ok(None);
        };
        Ok(Some(self.builder.build_load(ty, address, "")?))
    }

    /// Stores `value` in the attribute at `index` of `object`.
    pub(crate) fn set_field(
        &mut self,
        object: ValueId,
        index: u32,
        value: ValueId,
    ) -> Result<(), CodegenError> {
        if let Some((address, _)) = self.field_address(object, index)? {
            let value = self.value(value)?;
            self.builder.build_store(address, value)?;
        }
        Ok(())
    }

    /// Returns whether two objects are one, by address.
    pub(crate) fn identical(
        &mut self,
        left: ValueId,
        right: ValueId,
    ) -> Result<BasicValueEnum<'ctx>, CodegenError> {
        let left = self.value(left)?.into_pointer_value();
        let right = self.value(right)?.into_pointer_value();
        Ok(self
            .builder
            .build_int_compare(IntPredicate::EQ, left, right, "")?
            .into())
    }
}
