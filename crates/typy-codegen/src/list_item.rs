//! Reads and writes a list item in place when it is one word that needs no reference count of
//! its own, and calls the runtime otherwise, which also raises `IndexError` (ADR-0107).

use inkwell::types::StructType;
use inkwell::values::{IntValue, PointerValue};
use inkwell::{AddressSpace, IntPredicate};
use typy_tyir::{RuntimeFunction, ValueId};

use crate::error::CodegenError;
use crate::function::FunctionEmitter;
use crate::in_place_call::Raised;

/// The field of `ListObject` that holds the number of items, as typy-runtime lays it out:
/// the header's two `u32` fields, then `length`, `capacity`, `kind`, `words`, `items` and
/// `tuple`. The runtime's test `the_list_layout_is_the_one_generated_code_reads` pins it.
const LENGTH: u32 = 2;
/// The field that holds the item kind.
const KIND: u32 = 4;
/// The field that holds the address of the items.
const ITEMS: u32 = 6;
/// The item kinds below this hold each item as one word in its slot: `int`, `float`, `bool`,
/// an object and `None`.
const WORD_KINDS: u64 = 5;
/// The item kinds below this count no reference for an inline value: `int`, counted only
/// when large, `float` and `bool`.
const UNCOUNTED_KINDS: u64 = 3;
/// The item kind of `int`.
const INT_KIND: u64 = 0;

impl<'ctx> FunctionEmitter<'_, '_, 'ctx> {
    /// Returns `list[index]` and no exception, read in place when the index is an inline `int`
    /// inside the list and its items are words; otherwise from `typy_rt_list_get`.
    pub(crate) fn list_get_in_place(
        &mut self,
        arguments: &[ValueId],
    ) -> Result<Raised<'ctx>, CodegenError> {
        let list = self.value(arguments[0])?.into_pointer_value();
        let index = self.value(arguments[1])?.into_int_value();
        let (slot, inside) = self.slot(list, index)?;
        let kind = self.kind_below(list, WORD_KINDS)?;
        let fast = self.builder.build_and(inside, kind, "")?;
        self.in_place_or_call(fast, RuntimeFunction::ListGet, arguments, |this| {
            Ok(this
                .builder
                .build_load(this.shared.context.i64_type(), slot, "")?)
        })
    }

    /// Stores `item` at `list[index]` in place when the index is an inline `int` inside the
    /// list and neither the item nor the one it replaces needs a reference count: a `float`, a
    /// `bool`, or two inline `int` words. Otherwise `function`, `typy_rt_list_set` or
    /// `typy_rt_list_set_moved`, stores it. Returns no result and the exception.
    pub(crate) fn list_set_in_place(
        &mut self,
        function: RuntimeFunction,
        arguments: &[ValueId],
    ) -> Result<Raised<'ctx>, CodegenError> {
        let list = self.value(arguments[0])?.into_pointer_value();
        let index = self.value(arguments[1])?.into_int_value();
        let item = self.value(arguments[2])?.into_int_value();
        let (slot, inside) = self.slot(list, index)?;
        let uncounted = self.kind_below(list, UNCOUNTED_KINDS)?;
        let possible = self.builder.build_and(inside, uncounted, "")?;
        let possible = self.expect_true(possible)?;
        let context = self.shared.context;
        let word = context.i64_type();
        let look = context.append_basic_block(self.llvm_function, "item_replaced");
        let store = context.append_basic_block(self.llvm_function, "item_in_place");
        let call = context.append_basic_block(self.llvm_function, "item_call");
        let join = context.append_basic_block(self.llvm_function, "");
        self.builder
            .build_conditional_branch(possible, look, call)?;
        // The slot is inside the list here, so the item it holds can be read.
        self.builder.position_at_end(look);
        let replaced = self.builder.build_load(word, slot, "")?.into_int_value();
        let kind = self.kind(list)?;
        let not_int = self.builder.build_int_compare(
            IntPredicate::NE,
            kind,
            context.i32_type().const_int(INT_KIND, false),
            "",
        )?;
        let inline = self.both_inline(item, replaced)?;
        let plain = self.builder.build_or(not_int, inline, "")?;
        let plain = self.expect_true(plain)?;
        self.builder.build_conditional_branch(plain, store, call)?;
        self.builder.position_at_end(store);
        self.builder.build_store(slot, item)?;
        let store_end = self.current_block()?;
        self.builder.build_unconditional_branch(join)?;
        self.builder.position_at_end(call);
        let (_, raised) = self.call_raising(function, arguments)?;
        let call_end = self.current_block()?;
        self.builder.build_unconditional_branch(join)?;
        self.builder.position_at_end(join);
        let exception = self.no_exception_or(raised, store_end, call_end)?;
        Ok((None, exception))
    }

    /// Returns the address of item `index` of `list`, whose items are one word each, and
    /// whether the index is an inline `int` inside the list, counting from the end when it is
    /// negative, as Python does. The address is only read where the index is inside.
    fn slot(
        &self,
        list: PointerValue<'ctx>,
        index: IntValue<'ctx>,
    ) -> Result<(PointerValue<'ctx>, IntValue<'ctx>), CodegenError> {
        let context = self.shared.context;
        let word = context.i64_type();
        let layout = self.list_layout();
        let inline = self.is_inline(index)?;
        let position = self
            .builder
            .build_right_shift(index, word.const_int(1, false), true, "")?;
        let length_field = self.builder.build_struct_gep(layout, list, LENGTH, "")?;
        let length = self
            .builder
            .build_load(word, length_field, "")?
            .into_int_value();
        let negative =
            self.builder
                .build_int_compare(IntPredicate::SLT, position, word.const_zero(), "")?;
        let from_end = self.builder.build_int_add(position, length, "")?;
        let position = self
            .builder
            .build_select(negative, from_end, position, "")?
            .into_int_value();
        // One unsigned comparison refuses both a position below zero and one past the end.
        let below = self
            .builder
            .build_int_compare(IntPredicate::ULT, position, length, "")?;
        let inside = self.builder.build_and(inline, below, "")?;
        let items_field = self.builder.build_struct_gep(layout, list, ITEMS, "")?;
        let items = self
            .builder
            .build_load(context.ptr_type(AddressSpace::default()), items_field, "")?
            .into_pointer_value();
        // inkwell builds an element address only in unsafe code, which the compiler forbids,
        // so the address is computed from the integer the pointer holds.
        let base = self.builder.build_ptr_to_int(items, word, "")?;
        let offset = self
            .builder
            .build_left_shift(position, word.const_int(3, false), "")?;
        let address = self.builder.build_int_add(base, offset, "")?;
        let slot = self.builder.build_int_to_ptr(
            address,
            context.ptr_type(AddressSpace::default()),
            "",
        )?;
        Ok((slot, inside))
    }

    /// Returns the item kind of `list`.
    fn kind(&self, list: PointerValue<'ctx>) -> Result<IntValue<'ctx>, CodegenError> {
        let field = self
            .builder
            .build_struct_gep(self.list_layout(), list, KIND, "")?;
        Ok(self
            .builder
            .build_load(self.shared.context.i32_type(), field, "")?
            .into_int_value())
    }

    /// Returns whether the item kind of `list` is below `limit`.
    fn kind_below(
        &self,
        list: PointerValue<'ctx>,
        limit: u64,
    ) -> Result<IntValue<'ctx>, CodegenError> {
        let kind = self.kind(list)?;
        let limit = self.shared.context.i32_type().const_int(limit, false);
        Ok(self
            .builder
            .build_int_compare(IntPredicate::ULT, kind, limit, "")?)
    }

    /// Returns the LLVM type of `ListObject`'s fields.
    fn list_layout(&self) -> StructType<'ctx> {
        let context = self.shared.context;
        let half = context.i32_type().into();
        let word = context.i64_type().into();
        let pointer = context.ptr_type(AddressSpace::default()).into();
        context.struct_type(
            &[half, half, word, word, half, half, pointer, pointer],
            false,
        )
    }
}
