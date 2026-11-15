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
    let class = &shared.tyir.classes[index as usize];
    let default = if class.exception_base.is_some() {
        "typy_rt_exception_repr"
    } else {
        "typy_rt_instance_repr"
    };
    match class.repr {
        Some(method) => method_writer(shared, index, method, "repr"),
        None => Ok(runtime_symbol(shared, default, repr_type(shared))),
    }
}

/// Returns the function the type object of class `index` names to write an instance for
/// `str`, one that calls the class's `__str__`, or null when it has none and `str` is `repr`.
fn str_function<'ctx>(
    shared: &Shared<'_, 'ctx>,
    index: u32,
) -> Result<PointerValue<'ctx>, CodegenError> {
    let class = &shared.tyir.classes[index as usize];
    match (class.str, class.exception_base) {
        (Some(method), _) => Ok(method_writer(shared, index, method, "str")?
            .as_global_value()
            .as_pointer_value()),
        // An exception's `str` comes from its arguments (EXC-08), as `OSError` writes them
        // for a class derived from it (EXC-14).
        (None, Some(_)) => {
            let writer = if class.os_error {
                "typy_rt_os_error_str"
            } else {
                "typy_rt_exception_str"
            };
            Ok(runtime_symbol(shared, writer, repr_type(shared))
                .as_global_value()
                .as_pointer_value())
        }
        (None, None) => Ok(shared
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

/// Returns the `hash` slot of class `index`: it calls `__hash__` and reduces the `int` it
/// returns as CPython's `hash()` does (ADR-0026).
fn hash_slot<'ctx>(
    shared: &Shared<'_, 'ctx>,
    index: u32,
    method: typy_tyir::FunctionId,
) -> Result<FunctionValue<'ctx>, CodegenError> {
    let context = shared.context;
    let (word, pointer) = (
        context.i64_type(),
        context.ptr_type(AddressSpace::default()),
    );
    let function = shared.module.add_function(
        &format!("typy.class{index}.hash"),
        word.fn_type(&[pointer.into()], false),
        Some(Linkage::Private),
    );
    let builder = context.create_builder();
    builder.position_at_end(context.append_basic_block(function, ""));
    let instance = slot_parameter(function, 0)?;
    let value = builder
        .build_call(shared.functions[method.0 as usize], &[instance.into()], "")?
        .try_as_basic_value()
        .basic()
        .ok_or_else(|| CodegenError("`__hash__` returns an int".to_owned()))?;
    let reduce = runtime_symbol(
        shared,
        "typy_rt_int_hash",
        word.fn_type(&[word.into()], false),
    );
    let hash = builder
        .build_call(reduce, &[value.into()], "")?
        .try_as_basic_value()
        .basic()
        .ok_or_else(|| CodegenError("a hash is an integer".to_owned()))?;
    builder.build_return(Some(&hash))?;
    Ok(function)
}

/// Returns the `truth` slot of class `index`, which calls `__bool__` (CLS-07).
fn truth_slot<'ctx>(
    shared: &Shared<'_, 'ctx>,
    index: u32,
    method: typy_tyir::FunctionId,
) -> Result<FunctionValue<'ctx>, CodegenError> {
    let context = shared.context;
    let (byte, pointer) = (context.i8_type(), context.ptr_type(AddressSpace::default()));
    let function = shared.module.add_function(
        &format!("typy.class{index}.truth"),
        byte.fn_type(&[pointer.into()], false),
        Some(Linkage::Private),
    );
    let builder = context.create_builder();
    builder.position_at_end(context.append_basic_block(function, ""));
    let instance = slot_parameter(function, 0)?;
    let truth = builder
        .build_call(shared.functions[method.0 as usize], &[instance.into()], "")?
        .try_as_basic_value()
        .basic()
        .ok_or_else(|| CodegenError("`__bool__` returns a bool".to_owned()))?
        .into_int_value();
    let truth = builder.build_int_z_extend(truth, byte, "")?;
    builder.build_return(Some(&truth))?;
    Ok(function)
}

/// Returns the `equal` slot of class `index`: it calls `__eq__` when the other object is an
/// instance of the class its parameter takes, and otherwise compares identities, as
/// `NotImplemented` and CPython's fallback give (ADR-0026).
fn equal_slot<'ctx>(
    shared: &mut Shared<'_, 'ctx>,
    index: u32,
    method: typy_tyir::FunctionId,
    takes: u32,
) -> Result<FunctionValue<'ctx>, CodegenError> {
    let context = shared.context;
    let (byte, pointer) = (context.i8_type(), context.ptr_type(AddressSpace::default()));
    let takes = type_object(shared, takes)?;
    let function = shared.module.add_function(
        &format!("typy.class{index}.equal"),
        byte.fn_type(&[pointer.into(), pointer.into()], false),
        Some(Linkage::Private),
    );
    let builder = context.create_builder();
    let entry = context.append_basic_block(function, "");
    let call = context.append_basic_block(function, "");
    let identity = context.append_basic_block(function, "");
    builder.position_at_end(entry);
    let (left, right) = (slot_parameter(function, 0)?, slot_parameter(function, 1)?);
    let is_instance = crate::runtime::runtime_function(
        context,
        shared.module,
        typy_tyir::RuntimeFunction::IsInstance,
    );
    // The runtime takes the object as the word a list item is.
    let right_bits = builder.build_ptr_to_int(right, context.i64_type(), "")?;
    let accepted = builder
        .build_call(is_instance, &[right_bits.into(), takes.into()], "")?
        .try_as_basic_value()
        .basic()
        .ok_or_else(|| CodegenError("`is_instance` gives a bool".to_owned()))?
        .into_int_value();
    let accepted = builder.build_int_compare(IntPredicate::NE, accepted, byte.const_zero(), "")?;
    builder.build_conditional_branch(accepted, call, identity)?;
    builder.position_at_end(call);
    let equal = call_comparison(shared, &builder, method, left, right)?;
    builder.build_return(Some(&equal))?;
    builder.position_at_end(identity);
    let same = builder.build_int_compare(
        IntPredicate::EQ,
        builder.build_ptr_to_int(left, context.i64_type(), "")?,
        builder.build_ptr_to_int(right, context.i64_type(), "")?,
        "",
    )?;
    let same = builder.build_int_z_extend(same, byte, "")?;
    builder.build_return(Some(&same))?;
    Ok(function)
}

/// Returns the `order` slot of class `index`: for the comparison the runtime asks for, by its
/// tagged code, it calls `__lt__`, `__le__`, `__gt__` or `__ge__`, or the reflected method
/// with the operands exchanged (ADR-0026).
fn order_slot<'ctx>(
    shared: &Shared<'_, 'ctx>,
    index: u32,
    methods: [Option<typy_tyir::FunctionId>; 4],
) -> Result<FunctionValue<'ctx>, CodegenError> {
    let context = shared.context;
    let (byte, word) = (context.i8_type(), context.i64_type());
    let pointer = context.ptr_type(AddressSpace::default());
    let function = shared.module.add_function(
        &format!("typy.class{index}.order"),
        byte.fn_type(&[pointer.into(), pointer.into(), word.into()], false),
        Some(Linkage::Private),
    );
    let builder = context.create_builder();
    let entry = context.append_basic_block(function, "");
    let otherwise = context.append_basic_block(function, "");
    let (left, right) = (slot_parameter(function, 0)?, slot_parameter(function, 1)?);
    let code = function
        .get_nth_param(2)
        .ok_or_else(|| CodegenError("the slot takes a comparison".to_owned()))?
        .into_int_value();
    // `<`, `<=`, `>` and `>=` are the comparisons 2 to 5, tagged as `int` words; `a > b` is
    // `b < a` when only the reflected method is defined.
    let mut cases = Vec::new();
    for (position, method) in (2_u64..).zip(methods) {
        let reflected = methods[[2, 3, 0, 1][usize::try_from(position - 2).unwrap_or(0)]];
        let (callee, first, second) = match (method, reflected) {
            (Some(method), _) => (method, left, right),
            (None, Some(method)) => (method, right, left),
            (None, None) => continue,
        };
        let block = context.append_basic_block(function, "");
        builder.position_at_end(block);
        let result = call_comparison(shared, &builder, callee, first, second)?;
        builder.build_return(Some(&result))?;
        cases.push((word.const_int((position << 1) | 1, false), block));
    }
    builder.position_at_end(otherwise);
    builder.build_return(Some(&byte.const_zero()))?;
    builder.position_at_end(entry);
    builder.build_switch(code, otherwise, &cases)?;
    Ok(function)
}

/// Calls a comparison method, which cannot raise, and returns its `bool` as a byte.
fn call_comparison<'ctx>(
    shared: &Shared<'_, 'ctx>,
    builder: &inkwell::builder::Builder<'ctx>,
    method: typy_tyir::FunctionId,
    left: PointerValue<'ctx>,
    right: PointerValue<'ctx>,
) -> Result<BasicValueEnum<'ctx>, CodegenError> {
    let result = builder
        .build_call(
            shared.functions[method.0 as usize],
            &[left.into(), right.into()],
            "",
        )?
        .try_as_basic_value()
        .basic()
        .ok_or_else(|| CodegenError("a comparison method returns a bool".to_owned()))?
        .into_int_value();
    Ok(builder
        .build_int_z_extend(result, shared.context.i8_type(), "")?
        .into())
}

/// Returns a pointer parameter of a slot function.
fn slot_parameter(
    function: FunctionValue<'_>,
    position: u32,
) -> Result<PointerValue<'_>, CodegenError> {
    Ok(function
        .get_nth_param(position)
        .ok_or_else(|| CodegenError("the slot takes the objects".to_owned()))?
        .into_pointer_value())
}

/// Returns the base a class's type object names and its public type: for a copy of a generic
/// class, the generic class both times, so `isinstance` holds and `type()` gives it
/// (ADR-0028); otherwise the class's base, or the runtime's `object`, and null.
fn bases<'ctx>(
    shared: &mut Shared<'_, 'ctx>,
    class: &typy_tyir::TyirClass,
) -> Result<(PointerValue<'ctx>, PointerValue<'ctx>), CodegenError> {
    let pointer = shared.context.ptr_type(AddressSpace::default());
    let public = match class.public {
        Some(generic) => type_object(shared, generic)?,
        None => pointer.const_null(),
    };
    let base = match (class.public.or(class.base), class.exception_base) {
        (Some(base), _) | (_, Some(typy_tyir::TypeObjectRef::Class(base))) => {
            type_object(shared, base)?
        }
        (None, Some(reference)) => {
            let symbol = reference
                .runtime_symbol()
                .ok_or_else(|| CodegenError("a runtime class has a symbol".to_owned()))?;
            runtime_global(shared, &symbol)
        }
        (None, None) => runtime_global(shared, "typy_rt_object"),
    };
    Ok((base, public))
}

/// Returns the layout of a runtime `TypeObject`, which the runtime reads.
fn type_object_type(context: &inkwell::context::Context) -> StructType<'_> {
    let (half, word) = (context.i32_type(), context.i64_type());
    let pointer = context.ptr_type(AddressSpace::default());
    let header = context.struct_type(&[half.into(), half.into()], false);
    context.struct_type(
        &[
            header.into(),
            pointer.into(),
            word.into(),
            pointer.into(),
            pointer.into(),
            pointer.into(),
            pointer.into(),
            pointer.into(),
            pointer.into(),
            pointer.into(),
            word.into(),
            pointer.into(),
            pointer.into(),
            pointer.into(),
        ],
        false,
    )
}

/// Returns the layout of a class's type object followed by its method table, which only the
/// program reads (ADR-0021).
fn class_object_type(context: &inkwell::context::Context, methods: u32) -> StructType<'_> {
    let pointer = context.ptr_type(AddressSpace::default());
    context.struct_type(
        &[
            type_object_type(context).into(),
            pointer.array_type(methods).into(),
        ],
        false,
    )
}

/// Returns the address of the type object of class `index`, emitting it on first use: an
/// immortal object of type `type`, whose base is the class's base or the runtime's `object`,
/// followed by the class's method table.
pub(crate) fn type_object<'ctx>(
    shared: &mut Shared<'_, 'ctx>,
    index: u32,
) -> Result<PointerValue<'ctx>, CodegenError> {
    if let Some(pointer) = shared.statics.type_object(index) {
        return Ok(pointer);
    }
    let class = &shared.tyir.classes[index as usize];
    // The global exists before its contents, so a slot function of the class, such as one
    // whose `__eq__` takes the class itself, can name the class's type object.
    let count = u32::try_from(class.methods.len()).unwrap_or(u32::MAX);
    let global = shared.module.add_global(
        class_object_type(shared.context, count),
        None,
        &format!("typy.class{index}"),
    );
    global.set_constant(true);
    global.set_linkage(Linkage::Private);
    global.set_unnamed_addr(true);
    shared
        .statics
        .add_type_object(index, global.as_pointer_value());
    let fields = tuple_descriptor(shared, class.fields)?;
    let context = shared.context;
    let (half, word) = (context.i32_type(), context.i64_type());
    let pointer = context.ptr_type(AddressSpace::default());
    let (name, name_length) = text_global(shared, &class.name, &format!("typy.class{index}.name"));
    let (module, module_length) =
        text_global(shared, &class.module, &format!("typy.class{index}.module"));
    let (base, public) = bases(shared, class)?;
    let hash = match class.hash {
        Some(method) => hash_slot(shared, index, method)?,
        None => runtime_symbol(
            shared,
            "typy_rt_identity_hash",
            word.fn_type(&[pointer.into()], false),
        ),
    };
    let truth = match class.truth {
        Some(method) => truth_slot(shared, index, method)?
            .as_global_value()
            .as_pointer_value(),
        None => pointer.const_null(),
    };
    let equal = match class.equal {
        Some((method, takes)) => equal_slot(shared, index, method, takes)?
            .as_global_value()
            .as_pointer_value(),
        None => pointer.const_null(),
    };
    let order = if class.order.iter().any(Option::is_some) {
        order_slot(shared, index, class.order)?
            .as_global_value()
            .as_pointer_value()
    } else {
        pointer.const_null()
    };
    let repr = repr_function(shared, index)?;
    let str = str_function(shared, index)?;
    let header = context.const_struct(
        &[
            half.const_int(u64::from(IMMORTAL), false).into(),
            half.const_int(u64::from(TYPE_TYPE_ID), false).into(),
        ],
        false,
    );
    let object = type_object_type(context).const_named_struct(&[
        header.into(),
        name.into(),
        word.const_int(name_length, false).into(),
        base.into(),
        truth.into(),
        hash.as_global_value().as_pointer_value().into(),
        equal.into(),
        order.into(),
        repr.as_global_value().as_pointer_value().into(),
        module.into(),
        word.const_int(module_length, false).into(),
        fields.into(),
        str.into(),
        public.into(),
    ]);
    let methods: Vec<PointerValue<'ctx>> = class
        .methods
        .iter()
        .map(|method| {
            shared.functions[method.0 as usize]
                .as_global_value()
                .as_pointer_value()
        })
        .collect();
    let class_object = class_object_type(context, count)
        .const_named_struct(&[object.into(), pointer.const_array(&methods).into()]);
    global.set_initializer(&class_object.as_basic_value_enum());
    Ok(global.as_pointer_value())
}

impl<'ctx> FunctionEmitter<'_, '_, 'ctx> {
    /// Returns the function at `slot` of the method table of the class of `instance`, an
    /// instance of class `class` or of one derived from it, with the type of `class`'s entry
    /// (ADR-0021). An instance and an exception both hold their class after their header.
    pub(crate) fn method(
        &mut self,
        class: u32,
        slot: u32,
        instance: ValueId,
    ) -> Result<(PointerValue<'ctx>, inkwell::types::FunctionType<'ctx>), CodegenError> {
        let entry = self
            .shared
            .tyir
            .classes
            .get(class as usize)
            .and_then(|owner| owner.methods.get(slot as usize))
            .ok_or_else(|| CodegenError(format!("class#{class} has no method#{slot}")))?;
        let function_type = self.shared.functions[entry.0 as usize].get_type();
        let context = self.shared.context;
        let pointer = context.ptr_type(AddressSpace::default());
        let object = self.value(instance)?.into_pointer_value();
        let header = context.struct_type(&[pointer.into(), pointer.into()], false);
        let class_field = self.builder.build_struct_gep(header, object, 1, "")?;
        let class_object = self
            .builder
            .build_load(pointer, class_field, "")?
            .into_pointer_value();
        // The entries before `slot`, then the entry, laid out as the table is.
        let reach = context.struct_type(
            &[
                type_object_type(context).into(),
                pointer.array_type(slot).into(),
                pointer.into(),
            ],
            false,
        );
        let address = self.builder.build_struct_gep(reach, class_object, 2, "")?;
        let function = self
            .builder
            .build_load(pointer, address, "")?
            .into_pointer_value();
        Ok((function, function_type))
    }

    /// Returns the layout of an instance of the class of `object`: its header, its class, then
    /// its attributes as a tuple, with the attributes' types and the position of the tuple. An
    /// exception of class `class` holds its fixed part, then its attributes (ADR-0013).
    fn instance_layout(
        &self,
        object: ValueId,
        class: Option<u32>,
    ) -> Result<(StructType<'ctx>, Vec<TyirType>, u32), CodegenError> {
        let ty = self.function.value(object).ty;
        let class = match (ty, class) {
            (_, Some(index)) => self.shared.tyir.classes.get(index as usize),
            (TyirType::Object(id), None) => self.shared.tyir.class(id),
            _ => None,
        }
        .ok_or_else(|| CodegenError(format!("v{} is not an instance", object.0)))?;
        let tuples = &self.shared.tyir.tuple_types;
        let items = tuples.items(class.fields).unwrap_or_default().to_vec();
        let fields = value_type(self.shared.context, tuples, TyirType::Tuple(class.fields))
            .ok_or_else(|| CodegenError("attributes have a type".to_owned()))?;
        let context = self.shared.context;
        if class.exception_base.is_some() {
            let fixed = context
                .i8_type()
                .array_type(crate::layout::EXCEPTION_PREFIX);
            let layout = context.struct_type(&[fixed.into(), fields], false);
            return Ok((layout, items, 1));
        }
        let layout = context.struct_type(
            &[
                context.i64_type().into(),
                context.ptr_type(AddressSpace::default()).into(),
                fields,
            ],
            false,
        );
        Ok((layout, items, 2))
    }

    /// Returns the address of the attribute at `index` of `object`, with its type, or `None`
    /// for an attribute of type `None`, which takes no room.
    fn field_address(
        &self,
        object: ValueId,
        class: Option<u32>,
        index: u32,
    ) -> Result<Option<(PointerValue<'ctx>, BasicTypeEnum<'ctx>)>, CodegenError> {
        let (layout, items, position) = self.instance_layout(object, class)?;
        let Some(field) = tuple_field(&items, index as usize) else {
            return Ok(None);
        };
        let fields = layout
            .get_field_type_at_index(position)
            .ok_or_else(|| CodegenError("an instance has attributes".to_owned()))?
            .into_struct_type();
        let field_type = fields
            .get_field_type_at_index(field)
            .ok_or_else(|| CodegenError(format!("attribute {index} has a type")))?;
        let pointer = self.value(object)?.into_pointer_value();
        let attributes = self
            .builder
            .build_struct_gep(layout, pointer, position, "")?;
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
        class: Option<u32>,
        index: u32,
    ) -> Result<Option<BasicValueEnum<'ctx>>, CodegenError> {
        let Some((address, ty)) = self.field_address(object, class, index)? else {
            return Ok(None);
        };
        Ok(Some(self.builder.build_load(ty, address, "")?))
    }

    /// Stores `value` in the attribute at `index` of `object`.
    pub(crate) fn set_field(
        &mut self,
        object: ValueId,
        class: Option<u32>,
        index: u32,
        value: ValueId,
    ) -> Result<(), CodegenError> {
        if let Some((address, _)) = self.field_address(object, class, index)? {
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
