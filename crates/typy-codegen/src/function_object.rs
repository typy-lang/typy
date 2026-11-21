//! Emits function objects and the calls through them (FUN-05, ADR-0060): for each function
//! used as a value, an immortal descriptor, an immortal object and a thunk, and for each call
//! through a value an indirect call of the code its descriptor gives.

use inkwell::AddressSpace;
use inkwell::context::Context;
use inkwell::module::Linkage;
use inkwell::types::{BasicTypeEnum, FunctionType, StructType};
use inkwell::values::{
    BasicMetadataValueEnum, BasicValue, BasicValueEnum, PointerValue, ValueKind,
};
use typy_tyir::{FunctionId, TupleTypeId, TyirType, ValueId};

use crate::abi::{function_type, value_type};
use crate::error::CodegenError;
use crate::function::{FunctionEmitter, Shared};
use crate::layout::{
    BUILTIN_FUNCTION_TYPE_ID, BUILTIN_TYPE_FUNCTION_TYPE_ID, FUNCTION_TYPE_ID, IMMORTAL,
    METHOD_DESCRIPTOR_TYPE_ID,
};
use typy_tyir::StandsFor;

/// The field of the object that holds the descriptor: after the header, where an instance
/// keeps its class (ADR-0012).
const OBJECT_DESCRIPTOR: u32 = 2;
/// The field of the descriptor that holds the address of the code.
const DESCRIPTOR_CODE: u32 = 2;
/// The field of a bound method that holds its instance, after the descriptor.
const BOUND_INSTANCE: u32 = 3;
/// The field of a nested function's object that holds its captures, after the descriptor.
const CLOSURE_CAPTURES: u32 = 2;

/// What a thunk gives the function it calls first.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Receives {
    /// Nothing: a function of the module or a method used as a function.
    Nothing,
    /// The instance a bound method holds.
    Instance,
    /// The function object itself, which a nested function reads its captures from.
    Object,
}

/// Returns the layout of a function object without captures: the header, then the address of
/// its descriptor.
fn object_type(context: &Context) -> StructType<'_> {
    let pointer = context.ptr_type(AddressSpace::default());
    context.struct_type(
        &[
            context.i32_type().into(),
            context.i32_type().into(),
            pointer.into(),
        ],
        false,
    )
}

/// Returns the layout of a bound method: a function object whose one capture is the
/// instance, under the type ID of a method.
fn bound_type(context: &Context) -> StructType<'_> {
    let pointer = context.ptr_type(AddressSpace::default());
    context.struct_type(
        &[
            context.i32_type().into(),
            context.i32_type().into(),
            pointer.into(),
            pointer.into(),
        ],
        false,
    )
}

/// Returns the layout of a descriptor: the `str` objects of the function's name and qualified
/// name, the address of its code, the descriptor of its captures, null for none, and the
/// descriptor of the function it stands for (detailed design section 10.2, ADR-0076).
fn descriptor_type(context: &Context) -> StructType<'_> {
    let pointer = context.ptr_type(AddressSpace::default()).into();
    context.struct_type(&[pointer, pointer, pointer, pointer, pointer], false)
}

/// Returns the immortal function object of `id`, emitting it with its descriptor and its thunk
/// the first time it is used.
pub(crate) fn function_object<'ctx>(
    shared: &mut Shared<'_, 'ctx>,
    id: FunctionId,
) -> Result<PointerValue<'ctx>, CodegenError> {
    if let Some(object) = shared.statics.function_object(id.0) {
        return Ok(object);
    }
    let context = shared.context;
    let code = thunk(shared, id, Receives::Nothing)?;
    let descriptor = descriptor(shared, id, code, ("function", None))?;
    // A built-in function used as a value is written as CPython writes one (FUN-08).
    let stands_for = shared
        .tyir
        .functions
        .get(id.0 as usize)
        .map_or(StandsFor::Itself, |function| function.stands_for);
    let type_id = match stands_for {
        StandsFor::Itself => FUNCTION_TYPE_ID,
        StandsFor::BuiltinFunction => BUILTIN_FUNCTION_TYPE_ID,
        StandsFor::BuiltinMethod => METHOD_DESCRIPTOR_TYPE_ID,
        StandsFor::BuiltinType => BUILTIN_TYPE_FUNCTION_TYPE_ID,
    };
    let object = object_type(context).const_named_struct(&[
        context
            .i32_type()
            .const_int(u64::from(IMMORTAL), false)
            .into(),
        context
            .i32_type()
            .const_int(u64::from(type_id), false)
            .into(),
        descriptor.into(),
    ]);
    // Each function has one object, so `is` compares functions by it: its address is kept.
    let global =
        shared
            .module
            .add_global(object.get_type(), None, &format!("typy.function.{}", id.0));
    global.set_initializer(&object);
    global.set_constant(true);
    global.set_linkage(Linkage::Private);
    let object = global.as_pointer_value();
    shared.statics.add_function_object(id.0, object);
    Ok(object)
}

/// Returns the descriptor of the method `id` as a bound method, emitting it with its thunk the
/// first time it is used.
pub(crate) fn method_descriptor<'ctx>(
    shared: &mut Shared<'_, 'ctx>,
    id: FunctionId,
) -> Result<PointerValue<'ctx>, CodegenError> {
    if let Some(descriptor) = shared.statics.method_descriptor(id.0) {
        return Ok(descriptor);
    }
    let code = thunk(shared, id, Receives::Instance)?;
    let descriptor = descriptor(shared, id, code, ("method", None))?;
    shared.statics.add_method_descriptor(id.0, descriptor);
    Ok(descriptor)
}

/// Returns the descriptor of the nested function `id`, whose captures are laid out as the
/// tuple type `captures`, emitting it with its thunk the first time it is used (FUN-04).
pub(crate) fn closure_descriptor<'ctx>(
    shared: &mut Shared<'_, 'ctx>,
    id: FunctionId,
    captures: TupleTypeId,
) -> Result<PointerValue<'ctx>, CodegenError> {
    if let Some(descriptor) = shared.statics.closure_descriptor(id.0) {
        return Ok(descriptor);
    }
    let code = thunk(shared, id, Receives::Object)?;
    let layout = crate::descriptor::tuple_descriptor(shared, captures)?;
    let descriptor = descriptor(shared, id, code, ("closure", Some(layout)))?;
    shared.statics.add_closure_descriptor(id.0, descriptor);
    Ok(descriptor)
}

/// Emits the descriptor of function `id` whose code is `code`, as the private constant
/// `typy.KIND.descriptor.N`, with the descriptor of its captures, null for none.
fn descriptor<'ctx>(
    shared: &mut Shared<'_, 'ctx>,
    id: FunctionId,
    code: inkwell::values::FunctionValue<'ctx>,
    (kind, captures): (&str, Option<PointerValue<'ctx>>),
) -> Result<PointerValue<'ctx>, CodegenError> {
    let context = shared.context;
    let function = shared
        .tyir
        .functions
        .get(id.0 as usize)
        .ok_or_else(|| CodegenError(format!("fn{} does not exist", id.0)))?;
    let (stands_for, own_name) = (function.stands_for, function.qualified_name.clone());
    let name = shared.statics.string(shared.module, &function.name);
    let qualified_name = shared
        .statics
        .string(shared.module, &function.qualified_name);
    let captures =
        captures.unwrap_or_else(|| context.ptr_type(AddressSpace::default()).const_null());
    let global = shared.module.add_global(
        descriptor_type(context),
        None,
        &format!("typy.{kind}.descriptor.{}", id.0),
    );
    let own = global.as_pointer_value();
    // The hidden functions of one built-in stand for it, so they share one identity; any
    // other function stands for itself (ADR-0076).
    let identity = if stands_for == StandsFor::Itself {
        own
    } else {
        shared.statics.builtin_identity(&own_name, own)
    };
    let descriptor = descriptor_type(context).const_named_struct(&[
        name.into(),
        qualified_name.into(),
        code.as_global_value().as_pointer_value().into(),
        captures.into(),
        identity.into(),
    ]);
    global.set_initializer(&descriptor);
    global.set_constant(true);
    global.set_linkage(Linkage::Private);
    global.set_unnamed_addr(true);
    Ok(own)
}

/// Emits the code a call through a function object of `id` runs: it takes the object, then
/// the function's arguments, calls the function directly, and returns its result beside an
/// exception, null when the function cannot raise. For a bound method the object is the
/// bound method, and the instance it holds is the function's first argument; a nested
/// function takes the object itself first.
fn thunk<'ctx>(
    shared: &Shared<'_, 'ctx>,
    id: FunctionId,
    receives: Receives,
) -> Result<inkwell::values::FunctionValue<'ctx>, CodegenError> {
    let context = shared.context;
    let function = &shared.tyir.functions[id.0 as usize];
    let parameters: Vec<TyirType> = std::iter::once(TyirType::FUNCTION)
        .chain(
            function
                .parameters
                .iter()
                .skip(usize::from(receives != Receives::Nothing))
                .map(|value| function.value(*value).ty),
        )
        .collect();
    let tuples = &shared.tyir.tuple_types;
    let ty = function_type(
        context,
        tuples,
        &parameters,
        function.return_type,
        true,
        false,
    );
    let kind = match receives {
        Receives::Nothing => "thunk",
        Receives::Instance => "method_thunk",
        Receives::Object => "closure_thunk",
    };
    let thunk = shared.module.add_function(
        &format!("typy.{kind}.{}", function.symbol),
        ty,
        Some(Linkage::Internal),
    );
    let builder = context.create_builder();
    builder.position_at_end(context.append_basic_block(thunk, "entry"));
    let skipped = usize::from(receives != Receives::Object);
    let mut arguments: Vec<BasicMetadataValueEnum<'ctx>> = thunk
        .get_param_iter()
        .skip(skipped)
        .map(Into::into)
        .collect();
    if receives == Receives::Instance {
        let object = thunk
            .get_first_param()
            .ok_or_else(|| CodegenError("a thunk takes its object".to_owned()))?
            .into_pointer_value();
        let field = builder.build_struct_gep(bound_type(context), object, BOUND_INSTANCE, "")?;
        let instance = builder.build_load(context.ptr_type(AddressSpace::default()), field, "")?;
        arguments.insert(0, instance.into());
    }
    let call = builder.build_call(shared.functions[id.0 as usize], &arguments, "")?;
    let returned = match call.try_as_basic_value() {
        ValueKind::Basic(value) => Some(value),
        ValueKind::Instruction(_) => None,
    };
    if shared.raising[id.0 as usize] {
        call.set_tail_call(true);
        let returned = returned
            .ok_or_else(|| CodegenError("a raising function returns its exception".to_owned()))?;
        builder.build_return(Some(&returned))?;
        return Ok(thunk);
    }
    let null = context.ptr_type(AddressSpace::default()).const_null();
    match (value_type(context, tuples, function.return_type), returned) {
        (Some(result_type), Some(result)) => {
            let pair_type = context.struct_type(&[result_type, null.get_type().into()], false);
            let pair = builder.build_insert_value(pair_type.get_undef(), result, 0, "")?;
            let pair = builder.build_insert_value(pair, null, 1, "")?;
            builder.build_return(Some(&pair.as_basic_value_enum()))?;
        }
        _ => {
            builder.build_return(Some(&null))?;
        }
    }
    Ok(thunk)
}

impl<'ctx> FunctionEmitter<'_, '_, 'ctx> {
    /// Returns a new function object of the nested function `function`, with every capture
    /// zero, from the runtime (FUN-04).
    pub(crate) fn new_closure(
        &mut self,
        function: FunctionId,
        captures: TupleTypeId,
    ) -> Result<BasicValueEnum<'ctx>, CodegenError> {
        let descriptor = closure_descriptor(self.shared, function, captures)?;
        let pointer = self.shared.context.ptr_type(AddressSpace::default());
        let allocate = crate::class::runtime_symbol(
            self.shared,
            "typy_rt_closure_new",
            pointer.fn_type(&[pointer.into()], false),
        );
        self.builder
            .build_call(allocate, &[descriptor.into()], "")?
            .try_as_basic_value()
            .basic()
            .ok_or_else(|| CodegenError("the runtime returns the function object".to_owned()))
    }

    /// Returns the address of capture `index` of the function object `closure`, whose
    /// captures are laid out as `captures`, with its type, or `None` for a capture of type
    /// `None`, which takes no room.
    pub(crate) fn capture_address(
        &mut self,
        closure: ValueId,
        captures: TupleTypeId,
        index: u32,
    ) -> Result<Option<(PointerValue<'ctx>, BasicTypeEnum<'ctx>)>, CodegenError> {
        let context = self.shared.context;
        let tuples = &self.shared.tyir.tuple_types;
        let items = tuples.items(captures).unwrap_or_default().to_vec();
        let Some(field) = crate::abi::tuple_field(tuples, &items, index as usize) else {
            return Ok(None);
        };
        let fields = value_type(context, tuples, TyirType::Tuple(captures))
            .ok_or_else(|| CodegenError("captures have a type".to_owned()))?
            .into_struct_type();
        let pointer = context.ptr_type(AddressSpace::default());
        let layout = context.struct_type(
            &[context.i64_type().into(), pointer.into(), fields.into()],
            false,
        );
        let field_type = fields
            .get_field_type_at_index(field)
            .ok_or_else(|| CodegenError(format!("capture {index} has a type")))?;
        let object = self.value(closure)?.into_pointer_value();
        let all = self
            .builder
            .build_struct_gep(layout, object, CLOSURE_CAPTURES, "")?;
        let address = self.builder.build_struct_gep(fields, all, field, "")?;
        Ok(Some((address, field_type)))
    }

    /// Returns the address of the code of the function object `object` and the type of the
    /// call: the arguments' types, the first the object's, and `result`, in the raising
    /// form. The checker gave the arguments the types of the value's `Callable` type, which
    /// the code's own types are held alike to (NFR-19).
    pub(crate) fn value_code(
        &mut self,
        arguments: &[ValueId],
        result: TyirType,
    ) -> Result<(PointerValue<'ctx>, FunctionType<'ctx>), CodegenError> {
        let context = self.shared.context;
        let pointer = context.ptr_type(AddressSpace::default());
        let object = self
            .value(*arguments.first().ok_or_else(|| {
                CodegenError("a call through a value has no function object".to_owned())
            })?)?
            .into_pointer_value();
        let field =
            self.builder
                .build_struct_gep(object_type(context), object, OBJECT_DESCRIPTOR, "")?;
        let descriptor = self
            .builder
            .build_load(pointer, field, "")?
            .into_pointer_value();
        let field = self.builder.build_struct_gep(
            descriptor_type(context),
            descriptor,
            DESCRIPTOR_CODE,
            "",
        )?;
        let code = self
            .builder
            .build_load(pointer, field, "")?
            .into_pointer_value();
        let parameters: Vec<TyirType> = arguments
            .iter()
            .map(|argument| self.function.value(*argument).ty)
            .collect();
        let ty = function_type(
            context,
            &self.shared.tyir.tuple_types,
            &parameters,
            result,
            true,
            false,
        );
        Ok((code, ty))
    }
}
