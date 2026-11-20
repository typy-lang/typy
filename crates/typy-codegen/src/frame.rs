//! Emits the instances escape analysis holds in their function's frame: a block of the
//! instance's size for each site, its header cleared when the function starts, made through
//! the runtime and released before every return (MEM-03, ADR-0042).

use inkwell::AddressSpace;
use inkwell::values::{BasicValueEnum, PointerValue};
use typy_tyir::{Instruction, Operation, TyirType, ValueRepresentation};

use crate::abi::value_type;
use crate::class::{runtime_symbol, type_object};
use crate::descriptor::{item_size, target_data};
use crate::error::CodegenError;
use crate::function::FunctionEmitter;

/// The bytes before an instance's attributes: its header and its class.
const PREFIX: u64 = 16;

/// The alignment of a frame block, that of the allocator's memory.
const ALIGNMENT: u32 = 16;

impl<'ctx> FunctionEmitter<'_, '_, 'ctx> {
    /// Adds a block, in the entry block, for each instance the function holds in its frame, and
    /// clears its header, which tells the runtime the block holds no instance yet.
    pub(crate) fn prepare_frames(&mut self) -> Result<(), CodegenError> {
        let function = self.function;
        let sites = function
            .blocks
            .iter()
            .flat_map(|block| &block.instructions)
            .filter_map(
                |instruction| match (instruction.result, &instruction.operation) {
                    (Some(result), Operation::NewInstance { class })
                        if function.value(result).representation == ValueRepresentation::Stack =>
                    {
                        Some((result, *class))
                    }
                    _ => None,
                },
            );
        let word = self.shared.context.i64_type();
        let data = target_data(self.shared.module);
        for (result, class) in sites {
            let fields = self.shared.tyir.classes[class as usize].fields;
            let attributes = value_type(
                self.shared.context,
                &self.shared.tyir.tuple_types,
                TyirType::Tuple(fields),
            )
            .ok_or_else(|| CodegenError("attributes have a type".to_owned()))?
            .into_struct_type();
            let words = (PREFIX + item_size(&data, attributes)) / 8;
            let block = self.builder.build_alloca(
                word.array_type(u32::try_from(words).unwrap_or(u32::MAX)),
                "",
            )?;
            block
                .as_instruction()
                .map(|alloca| alloca.set_alignment(ALIGNMENT))
                .transpose()
                .map_err(|error| {
                    CodegenError(format!("a frame block is not aligned: {error:?}"))
                })?;
            self.builder.build_store(block, word.const_zero())?;
            self.frames.insert(result, block);
        }
        Ok(())
    }

    /// Returns the frame block an instruction makes its instance in, with the instance's class,
    /// or `None` for every other instruction.
    pub(crate) fn frame_site(
        &self,
        instruction: &Instruction,
    ) -> Option<(PointerValue<'ctx>, u32)> {
        let Operation::NewInstance { class } = instruction.operation else {
            return None;
        };
        Some((*self.frames.get(&instruction.result?)?, class))
    }

    /// Returns a new instance of `class` in `block`, or from the heap when the block's instance
    /// before is still held.
    pub(crate) fn frame_instance(
        &mut self,
        block: PointerValue<'ctx>,
        class: u32,
    ) -> Result<BasicValueEnum<'ctx>, CodegenError> {
        let type_object = type_object(self.shared, class)?;
        let pointer = self.shared.context.ptr_type(AddressSpace::default());
        let make = runtime_symbol(
            self.shared,
            "typy_rt_instance_on_stack",
            pointer.fn_type(&[pointer.into(), pointer.into()], false),
        );
        self.builder
            .build_call(make, &[block.into(), type_object.into()], "")?
            .try_as_basic_value()
            .basic()
            .ok_or_else(|| CodegenError("the runtime returns the instance".to_owned()))
    }

    /// Releases the attributes of the instance each frame block holds, before the function
    /// returns, then closes the generators that dropped (FUN-11).
    pub(crate) fn release_frames(&mut self) -> Result<(), CodegenError> {
        if self.frames.is_empty() {
            return Ok(());
        }
        let context = self.shared.context;
        let pointer = context.ptr_type(AddressSpace::default());
        let release = runtime_symbol(
            self.shared,
            "typy_rt_instance_off_stack",
            context.void_type().fn_type(&[pointer.into()], false),
        );
        let blocks: Vec<PointerValue<'ctx>> = self.frames.values().copied().collect();
        for block in blocks {
            self.builder.build_call(release, &[block.into()], "")?;
        }
        self.close_dropped()
    }
}
