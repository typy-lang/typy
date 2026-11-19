//! Emits generators (SMT-11, ADR-0063): the descriptor of each generator function, the
//! object its call makes, and its resume function, whose slots are fields of the frame in
//! the object, which starts by jumping to the block after the `yield` it stopped at, and
//! which ends a step at a `yield` by noting it and returning.

use inkwell::AddressSpace;
use inkwell::types::{BasicTypeEnum, StructType};
use inkwell::values::{BasicValueEnum, IntValue, PointerValue};
use typy_tyir::{BlockId, FunctionId, GeneratorFrame, TyirType, ValueId};

use crate::abi::value_type;
use crate::error::CodegenError;
use crate::function::{FunctionEmitter, Shared};
use crate::statics::Statics;

/// The field of a generator object that holds its state.
const STATE: u32 = 2;
/// The field that holds the `yield` its body stopped at, numbered from one.
const POINT: u32 = 3;
/// The field that holds its frame.
const FRAME: u32 = 9;
/// The state of a generator stopped at a `yield`.
const SUSPENDED: u64 = 1;

/// Returns the layout of a generator object whose frame is `frame`: the header, the address
/// of its descriptor, its state and the yield it stopped at, its current item, what `send`
/// and `throw` pass, what its body returned, the room for an item held inline, then the
/// frame, as the runtime lays it out.
fn object_type<'ctx>(
    context: &'ctx inkwell::context::Context,
    frame: StructType<'ctx>,
) -> StructType<'ctx> {
    let pointer = context.ptr_type(AddressSpace::default());
    let word = context.i64_type();
    let half = context.i32_type();
    context.struct_type(
        &[
            word.into(),
            pointer.into(),
            half.into(),
            half.into(),
            word.into(),
            word.into(),
            pointer.into(),
            word.into(),
            pointer.into(),
            frame.into(),
        ],
        false,
    )
}

/// Returns the frame of the resume function `id`.
fn frame_of<'a>(
    shared: &'a Shared<'_, '_>,
    id: FunctionId,
) -> Result<&'a GeneratorFrame, CodegenError> {
    shared
        .tyir
        .functions
        .get(id.0 as usize)
        .and_then(|function| function.generator.as_ref())
        .ok_or_else(|| CodegenError(format!("fn{} is no resume function", id.0)))
}

/// Returns the descriptor of the resume function `id`, emitting it the first time it is
/// used: its names, its code, the layout of its frame, the number of arguments, and how its
/// items are held.
fn descriptor<'ctx>(
    shared: &mut Shared<'_, 'ctx>,
    id: FunctionId,
) -> Result<PointerValue<'ctx>, CodegenError> {
    if let Some(descriptor) = shared.statics.generator_descriptor(id.0) {
        return Ok(descriptor);
    }
    let context = shared.context;
    let frame = frame_of(shared, id)?.clone();
    let function = &shared.tyir.functions[id.0 as usize];
    let name = shared.statics.string(shared.module, &function.name);
    let qualified_name = shared
        .statics
        .string(shared.module, &function.qualified_name);
    let layout = crate::descriptor::tuple_descriptor(shared, frame.frame)?;
    let tuples = &shared.tyir.tuple_types;
    let kind = crate::descriptor::kind_code(frame.yields, tuples)?;
    let pointer = context.ptr_type(AddressSpace::default());
    let item_tuple = match frame.yields {
        TyirType::Tuple(tuple) | TyirType::Optional(typy_tyir::OptionalBase::Tuple(tuple)) => {
            crate::descriptor::tuple_descriptor(shared, tuple)?
        }
        _ => pointer.const_null(),
    };
    let word = context.i64_type();
    let code = shared.functions[id.0 as usize];
    let descriptor_type = context.struct_type(
        &[
            pointer.into(),
            pointer.into(),
            pointer.into(),
            pointer.into(),
            word.into(),
            word.into(),
            pointer.into(),
        ],
        false,
    );
    let arguments = u64::try_from(frame.arguments.len()).unwrap_or(0);
    let value = descriptor_type.const_named_struct(&[
        name.into(),
        qualified_name.into(),
        code.as_global_value().as_pointer_value().into(),
        layout.into(),
        word.const_int(arguments, false).into(),
        word.const_int(kind, false).into(),
        item_tuple.into(),
    ]);
    let descriptor = Statics::global(
        shared.module,
        value.into(),
        &format!("typy.generator.descriptor.{}", id.0),
    );
    shared.statics.add_generator_descriptor(id.0, descriptor);
    Ok(descriptor)
}

impl<'ctx> FunctionEmitter<'_, '_, 'ctx> {
    /// Returns the layout of the frame of the resume function `id` and of its object.
    fn generator_types(
        &self,
        id: FunctionId,
    ) -> Result<(StructType<'ctx>, StructType<'ctx>), CodegenError> {
        let context = self.shared.context;
        let frame = frame_of(self.shared, id)?;
        let frame_type = value_type(
            context,
            &self.shared.tyir.tuple_types,
            TyirType::Tuple(frame.frame),
        )
        .ok_or_else(|| CodegenError("a frame has a type".to_owned()))?
        .into_struct_type();
        Ok((frame_type, object_type(context, frame_type)))
    }

    /// Returns the address of field `index` of the frame of the generator `object` of the
    /// resume function `id`, with its type, or `None` for a field of type `None`.
    pub(crate) fn frame_field(
        &self,
        object: PointerValue<'ctx>,
        id: FunctionId,
        index: usize,
    ) -> Result<Option<(PointerValue<'ctx>, BasicTypeEnum<'ctx>)>, CodegenError> {
        let tuples = &self.shared.tyir.tuple_types;
        let frame = frame_of(self.shared, id)?;
        let items = tuples.items(frame.frame).unwrap_or_default().to_vec();
        let Some(field) = crate::abi::tuple_field(tuples, &items, index) else {
            return Ok(None);
        };
        let (frame_type, object) = (self.generator_types(id)?.0, object);
        let field_type = frame_type
            .get_field_type_at_index(field)
            .ok_or_else(|| CodegenError(format!("frame field {index} has a type")))?;
        let (_, object_type) = self.generator_types(id)?;
        let all = self
            .builder
            .build_struct_gep(object_type, object, FRAME, "")?;
        let address = self.builder.build_struct_gep(frame_type, all, field, "")?;
        Ok(Some((address, field_type)))
    }

    /// Returns a new generator of the resume function `function`, not started, from the
    /// runtime.
    pub(crate) fn new_generator(
        &mut self,
        function: FunctionId,
    ) -> Result<BasicValueEnum<'ctx>, CodegenError> {
        let descriptor = descriptor(self.shared, function)?;
        let pointer = self.shared.context.ptr_type(AddressSpace::default());
        let allocate = crate::class::runtime_symbol(
            self.shared,
            "typy_rt_generator_new",
            pointer.fn_type(&[pointer.into()], false),
        );
        self.builder
            .build_call(allocate, &[descriptor.into()], "")?
            .try_as_basic_value()
            .basic()
            .ok_or_else(|| CodegenError("the runtime returns the generator".to_owned()))
    }

    /// Stores `value` as argument `index` of the generator `generator`, which takes over its
    /// reference.
    pub(crate) fn set_generator_argument(
        &mut self,
        generator: ValueId,
        function: FunctionId,
        index: u32,
        value: ValueId,
    ) -> Result<(), CodegenError> {
        let object = self.value(generator)?.into_pointer_value();
        if let Some((address, _)) = self.frame_field(object, function, index as usize)? {
            let value = self.value(value)?;
            self.builder.build_store(address, value)?;
        }
        Ok(())
    }

    /// Returns argument `index` of the running generator, taking it out of the frame with
    /// its reference and leaving zero.
    pub(crate) fn generator_argument(
        &mut self,
        generator: ValueId,
        function: FunctionId,
        index: u32,
    ) -> Result<Option<BasicValueEnum<'ctx>>, CodegenError> {
        let object = self.value(generator)?.into_pointer_value();
        let Some((address, ty)) = self.frame_field(object, function, index as usize)? else {
            return Ok(None);
        };
        let value = self.builder.build_load(ty, address, "")?;
        self.builder.build_store(address, ty.const_zero())?;
        Ok(Some(value))
    }

    /// Returns the closure the running nested generator function keeps in its frame,
    /// borrowed.
    pub(crate) fn generator_closure(
        &mut self,
        generator: ValueId,
        function: FunctionId,
    ) -> Result<BasicValueEnum<'ctx>, CodegenError> {
        let object = self.value(generator)?.into_pointer_value();
        let (address, ty) = self
            .frame_field(object, function, 0)?
            .ok_or_else(|| CodegenError("a nested generator keeps its closure".to_owned()))?;
        Ok(self.builder.build_load(ty, address, "")?)
    }

    /// Makes `item`, which a `yield` gives, the current item of the running generator, which
    /// the runtime copies when it is held inline.
    pub(crate) fn set_generator_item(
        &mut self,
        generator: ValueId,
        item: ValueId,
    ) -> Result<(), CodegenError> {
        let context = self.shared.context;
        let pointer = context.ptr_type(AddressSpace::default());
        let hold = crate::class::runtime_symbol(
            self.shared,
            "typy_rt_generator_hold",
            context
                .void_type()
                .fn_type(&[pointer.into(), context.i64_type().into()], false),
        );
        let object = self.value(generator)?;
        let item = self.value(item)?;
        self.builder
            .build_call(hold, &[object.into(), item.into()], "")?;
        Ok(())
    }

    /// Returns the address of each slot of the running resume function: a field of the
    /// frame after the arguments.
    pub(crate) fn generator_slots(
        &mut self,
        generator: PointerValue<'ctx>,
    ) -> Result<Vec<Option<PointerValue<'ctx>>>, CodegenError> {
        let frame = frame_of(self.shared, self.function.id)?;
        let arguments = frame.arguments.len();
        (0..self.function.slots.len())
            .map(|slot| {
                Ok(self
                    .frame_field(generator, self.function.id, arguments + slot)?
                    .map(|(address, _)| address))
            })
            .collect()
    }

    /// Emits the start of a step: it reads the `yield` the body stopped at and continues in
    /// the block after it, or in `start` for the first step.
    pub(crate) fn resume(
        &mut self,
        generator: ValueId,
        start: BlockId,
        points: &[BlockId],
    ) -> Result<(), CodegenError> {
        let context = self.shared.context;
        let object = self.value(generator)?.into_pointer_value();
        let (_, object_type) = self.generator_types(self.function.id)?;
        let address = self
            .builder
            .build_struct_gep(object_type, object, POINT, "")?;
        let point = self
            .builder
            .build_load(context.i32_type(), address, "point")?
            .into_int_value();
        let half = context.i32_type();
        let cases: Vec<(IntValue<'ctx>, inkwell::basic_block::BasicBlock<'ctx>)> = points
            .iter()
            .enumerate()
            .map(|(position, block)| {
                let number = u64::try_from(position + 1).unwrap_or(0);
                (half.const_int(number, false), self.blocks[block.0 as usize])
            })
            .collect();
        self.builder
            .build_switch(point, self.blocks[start.0 as usize], &cases)?;
        Ok(())
    }

    /// Emits the end of a step at a `yield`: it notes the generator suspended at the yield
    /// whose next block is `resume`, and returns without an exception.
    pub(crate) fn suspend(
        &mut self,
        generator: ValueId,
        resume: BlockId,
    ) -> Result<(), CodegenError> {
        let context = self.shared.context;
        let points = match self.function.blocks.first().map(|block| &block.terminator) {
            Some(typy_tyir::Terminator::Resume { points, .. }) => points.clone(),
            _ => return Err(CodegenError("a yield follows a resume".to_owned())),
        };
        let position = points
            .iter()
            .position(|point| *point == resume)
            .ok_or_else(|| CodegenError(format!("b{} is no resume point", resume.0)))?;
        let object = self.value(generator)?.into_pointer_value();
        let (_, object_type) = self.generator_types(self.function.id)?;
        let half = context.i32_type();
        let state = self
            .builder
            .build_struct_gep(object_type, object, STATE, "")?;
        self.builder
            .build_store(state, half.const_int(SUSPENDED, false))?;
        let point = self
            .builder
            .build_struct_gep(object_type, object, POINT, "")?;
        let number = u64::try_from(position + 1).unwrap_or(0);
        self.builder
            .build_store(point, half.const_int(number, false))?;
        let null = context.ptr_type(AddressSpace::default()).const_null();
        self.builder.build_return(Some(&null))?;
        Ok(())
    }
}
