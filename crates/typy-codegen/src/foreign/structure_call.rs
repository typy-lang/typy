//! Passes a `ctypes` structure to a C function and takes one back by value, as the target's C
//! calling convention passes a C struct (FFI-03, FFI-12, ADR-0052).

use inkwell::AddressSpace;
use inkwell::attributes::{Attribute, AttributeLoc};
use inkwell::types::{AnyType, BasicMetadataTypeEnum, BasicType, BasicTypeEnum, IntType};
use inkwell::values::{BasicMetadataValueEnum, BasicValueEnum, CallSiteValue, PointerValue};
use typy_tyir::{CType, ValueId};

use crate::error::CodegenError;
use crate::function::FunctionEmitter;
use crate::structure_abi::{Passing, Piece, Registers, passing};

/// The parameters of a C call as they are built: their LLVM types, the values passed, and
/// the attributes a parameter carries, by its position.
pub(super) struct CallParts<'ctx> {
    pub(super) types: Vec<BasicMetadataTypeEnum<'ctx>>,
    pub(super) values: Vec<BasicMetadataValueEnum<'ctx>>,
    pub(super) attributes: Vec<(u32, Attribute)>,
}

impl<'ctx> CallParts<'ctx> {
    pub(super) const fn new() -> Self {
        Self {
            types: Vec::new(),
            values: Vec::new(),
            attributes: Vec::new(),
        }
    }

    /// Adds a parameter of LLVM type `ty` passed `value`.
    pub(super) fn push(&mut self, ty: BasicTypeEnum<'ctx>, value: BasicValueEnum<'ctx>) {
        self.types.push(ty.into());
        self.values.push(value.into());
    }

    /// Returns the position of the next parameter.
    fn next(&self) -> u32 {
        u32::try_from(self.types.len()).unwrap_or(u32::MAX)
    }

    /// Adds the attributes each parameter carries to the call.
    pub(super) fn attribute(&self, call: CallSiteValue<'ctx>) {
        for (position, attribute) in &self.attributes {
            call.add_attribute(AttributeLoc::Param(*position), *attribute);
        }
    }
}

/// A structure a C function returns: the new object that holds it, the address of its
/// buffer, and how it passes.
pub(super) struct StructureResult<'ctx> {
    object: BasicValueEnum<'ctx>,
    buffer: PointerValue<'ctx>,
    passing: Passing,
}

impl<'ctx> StructureResult<'ctx> {
    /// Returns the LLVM type the C function returns: its one piece, a pair of its two, an
    /// array of its pieces, or `None` when it returns nothing in registers.
    pub(super) fn return_type(
        &self,
        context: &'ctx inkwell::context::Context,
    ) -> Option<BasicTypeEnum<'ctx>> {
        match &self.passing {
            Passing::Registers(pieces) => {
                let types: Vec<BasicTypeEnum<'ctx>> = pieces
                    .iter()
                    .map(|piece| piece.llvm_type(context))
                    .collect();
                match types.as_slice() {
                    [one] => Some(*one),
                    _ => Some(context.struct_type(&types, false).into()),
                }
            }
            Passing::Array(pieces) => Some(array_type(context, pieces).into()),
            Passing::Nothing | Passing::Memory | Passing::Reference => None,
        }
    }
}

impl<'ctx> FunctionEmitter<'_, '_, 'ctx> {
    /// Returns a new object of the structure class at `class`, every byte zero, with the
    /// address of its buffer.
    fn new_structure(
        &mut self,
        class: u32,
    ) -> Result<(BasicValueEnum<'ctx>, PointerValue<'ctx>), CodegenError> {
        let context = self.shared.context;
        let (pointer, word) = (
            context.ptr_type(AddressSpace::default()),
            context.i64_type(),
        );
        // The runtime function takes the class as a word, as TyIR's `CdataNew` passes it.
        let address = crate::cdata::cdata_class(self.shared, class)?;
        let address = self.builder.build_ptr_to_int(address, word, "")?;
        let object = self.runtime_value(
            "typy_rt_cdata_new",
            pointer.fn_type(&[word.into()], false),
            &[address.into()],
        )?;
        let buffer = self.buffer_of(object)?;
        Ok((object, buffer))
    }

    /// Returns the address of the buffer of a `ctypes` object.
    fn buffer_of(
        &mut self,
        object: BasicValueEnum<'ctx>,
    ) -> Result<PointerValue<'ctx>, CodegenError> {
        let pointer = self.shared.context.ptr_type(AddressSpace::default());
        Ok(self
            .runtime_value(
                "typy_rt_cdata_argument",
                pointer.fn_type(&[pointer.into()], false),
                &[object.into()],
            )?
            .into_pointer_value())
    }

    /// Returns the structure result of the class at `class`, made before the call, and adds
    /// the buffer a result passed in memory goes to as the call's first parameter.
    pub(super) fn structure_result(
        &mut self,
        class: u32,
        call: &mut CallParts<'ctx>,
        registers: &mut Registers,
    ) -> Result<StructureResult<'ctx>, CodegenError> {
        let passing = passing(&self.shared.tyir.cdata_classes, class, self.shared.target);
        let (object, buffer) = self.new_structure(class)?;
        if matches!(passing, Passing::Memory | Passing::Reference) {
            let attribute = self.memory_attribute("sret", class);
            call.attributes.push((call.next(), attribute));
            let pointer = self.shared.context.ptr_type(AddressSpace::default());
            call.push(pointer.into(), buffer.into());
            registers.take_scalar(CType::VoidPointer);
        }
        Ok(StructureResult {
            object,
            buffer,
            passing,
        })
    }

    /// Adds the structure object `value`, of the class at `class`, as the parameters it
    /// passes as: its pieces in registers when enough are left, an array of its pieces, a copy
    /// in memory, or the address of a copy.
    pub(super) fn structure_argument(
        &mut self,
        value: ValueId,
        class: u32,
        call: &mut CallParts<'ctx>,
        registers: &mut Registers,
    ) -> Result<(), CodegenError> {
        let object = self.value(value)?;
        let buffer = self.buffer_of(object)?;
        let pointer = self.shared.context.ptr_type(AddressSpace::default());
        match passing(&self.shared.tyir.cdata_classes, class, self.shared.target) {
            Passing::Nothing => {}
            Passing::Registers(pieces) if registers.take(&pieces) => {
                for piece in &pieces {
                    let loaded = self.load_piece(buffer, *piece)?;
                    call.push(piece.llvm_type(self.shared.context), loaded);
                }
            }
            Passing::Registers(_) | Passing::Memory => {
                let attribute = self.memory_attribute("byval", class);
                call.attributes.push((call.next(), attribute));
                call.push(pointer.into(), buffer.into());
            }
            Passing::Array(pieces) => {
                let ty = array_type(self.shared.context, &pieces);
                let mut array = ty.get_undef();
                for (index, piece) in (0_u32..).zip(&pieces) {
                    let loaded = self.load_piece(buffer, *piece)?;
                    array = self
                        .builder
                        .build_insert_value(array, loaded, index, "")?
                        .into_array_value();
                }
                call.push(ty.into(), array.into());
            }
            Passing::Reference => {
                // The callee may change its argument, so it gets a copy of its own.
                let size = self.shared.tyir.cdata_classes[class as usize].size;
                let bytes = self
                    .shared
                    .context
                    .i8_type()
                    .array_type(u32::try_from(size).unwrap_or(u32::MAX));
                let copy = self.entry_alloca(bytes.into())?;
                let length = self.shared.context.i64_type().const_int(size, false);
                self.builder.build_memcpy(copy, 16, buffer, 1, length)?;
                call.push(pointer.into(), copy.into());
            }
        }
        Ok(())
    }

    /// Returns the attribute `kind`, `byval` or `sret`, of a parameter that points to the
    /// bytes of the structure class at `class`.
    fn memory_attribute(&self, kind: &str, class: u32) -> Attribute {
        let context = self.shared.context;
        let size = self.shared.tyir.cdata_classes[class as usize].size;
        let bytes = context
            .i8_type()
            .array_type(u32::try_from(size).unwrap_or(u32::MAX));
        context.create_type_attribute(
            Attribute::get_named_enum_kind_id(kind),
            bytes.as_any_type_enum(),
        )
    }

    /// Returns the structure object a C call gave: its pieces stored into its buffer when it
    /// returned them in registers.
    pub(super) fn structure_value(
        &mut self,
        call: CallSiteValue<'ctx>,
        result: &StructureResult<'ctx>,
    ) -> Result<BasicValueEnum<'ctx>, CodegenError> {
        let (Passing::Registers(pieces) | Passing::Array(pieces)) = &result.passing else {
            return Ok(result.object);
        };
        let returned = call
            .try_as_basic_value()
            .basic()
            .ok_or_else(|| CodegenError("a structure in registers is returned".to_owned()))?;
        for (index, piece) in (0_u32..).zip(pieces) {
            let part = match returned {
                BasicValueEnum::ArrayValue(array) => {
                    self.builder.build_extract_value(array, index, "")?
                }
                BasicValueEnum::StructValue(pair) => {
                    self.builder.build_extract_value(pair, index, "")?
                }
                one => one,
            };
            self.store_piece(result.buffer, *piece, part)?;
        }
        Ok(result.object)
    }

    /// Returns the address of a piece of a buffer.
    fn piece_address(
        &mut self,
        buffer: PointerValue<'ctx>,
        piece: Piece,
    ) -> Result<PointerValue<'ctx>, CodegenError> {
        let context = self.shared.context;
        let word = context.i64_type();
        let start = self.builder.build_ptr_to_int(buffer, word, "")?;
        let offset = word.const_int(piece.offset, false);
        let address = self.builder.build_int_add(start, offset, "")?;
        let pointer = context.ptr_type(AddressSpace::default());
        Ok(self.builder.build_int_to_ptr(address, pointer, "")?)
    }

    /// Returns a piece of a buffer as it passes: a `float` or a `double`, or its bytes widened
    /// to a word, read without reading past them.
    fn load_piece(
        &mut self,
        buffer: PointerValue<'ctx>,
        piece: Piece,
    ) -> Result<BasicValueEnum<'ctx>, CodegenError> {
        let context = self.shared.context;
        let address = self.piece_address(buffer, piece)?;
        let ty: BasicTypeEnum<'ctx> = if piece.vector {
            piece.llvm_type(context)
        } else {
            bytes_type(context, piece.size)?.into()
        };
        let loaded = self.builder.build_load(ty, address, "")?;
        // A field of a structure need not be aligned for the whole word read.
        match loaded {
            BasicValueEnum::IntValue(value) => value.as_instruction(),
            BasicValueEnum::FloatValue(value) => value.as_instruction(),
            _ => None,
        }
        .map(|load| load.set_alignment(1))
        .transpose()
        .map_err(|error| CodegenError(format!("a load is not aligned: {error:?}")))?;
        if piece.vector || piece.size == 8 {
            return Ok(loaded);
        }
        Ok(self
            .builder
            .build_int_z_extend(loaded.into_int_value(), context.i64_type(), "")?
            .into())
    }

    /// Stores a piece of a structure a C function returned into a buffer, its own bytes only.
    fn store_piece(
        &mut self,
        buffer: PointerValue<'ctx>,
        piece: Piece,
        part: BasicValueEnum<'ctx>,
    ) -> Result<(), CodegenError> {
        let context = self.shared.context;
        let address = self.piece_address(buffer, piece)?;
        let stored = if piece.vector || piece.size == 8 {
            part
        } else {
            let narrow = bytes_type(context, piece.size)?;
            self.builder
                .build_int_truncate(part.into_int_value(), narrow, "")?
                .into()
        };
        let store = self.builder.build_store(address, stored)?;
        store
            .set_alignment(1)
            .map_err(|error| CodegenError(format!("a store is not aligned: {error:?}")))?;
        Ok(())
    }
}

/// Returns the array type AAPCS64 passes `pieces` as, all of one LLVM type.
fn array_type<'ctx>(
    context: &'ctx inkwell::context::Context,
    pieces: &[Piece],
) -> inkwell::types::ArrayType<'ctx> {
    let element = pieces.first().map_or_else(
        || context.i64_type().into(),
        |piece| piece.llvm_type(context),
    );
    element.array_type(u32::try_from(pieces.len()).unwrap_or(u32::MAX))
}

/// Returns the integer type of `size` bytes.
fn bytes_type(context: &inkwell::context::Context, size: u64) -> Result<IntType<'_>, CodegenError> {
    let bits = u32::try_from(size * 8)
        .ok()
        .and_then(core::num::NonZeroU32::new)
        .ok_or_else(|| CodegenError(format!("a piece of {size} bytes has no integer type")))?;
    context
        .custom_width_int_type(bits)
        .map_err(|error| CodegenError(error.to_owned()))
}
