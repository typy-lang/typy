//! Emits static objects: string and large-integer constants, and C strings for tracebacks.

use std::collections::BTreeMap;

use inkwell::context::Context;
use inkwell::module::{Linkage, Module};
use inkwell::values::{BasicValueEnum, IntValue, PointerValue};

use crate::layout::{
    BIG_INT_TYPE_ID, IMMORTAL, IntEncoding, STR_TYPE_ID, StrKind, encode_int, encode_str,
};

/// Creates each static object once per module.
pub(crate) struct Statics<'ctx> {
    context: &'ctx Context,
    strings: BTreeMap<String, PointerValue<'ctx>>,
    c_strings: BTreeMap<String, PointerValue<'ctx>>,
    big_ints: BTreeMap<String, PointerValue<'ctx>>,
    descriptors: BTreeMap<u32, PointerValue<'ctx>>,
}

impl<'ctx> Statics<'ctx> {
    pub(crate) fn new(context: &'ctx Context) -> Self {
        Self {
            context,
            strings: BTreeMap::new(),
            c_strings: BTreeMap::new(),
            big_ints: BTreeMap::new(),
            descriptors: BTreeMap::new(),
        }
    }

    /// Returns the descriptor of tuple type `id`, if it was emitted already.
    pub(crate) fn descriptor(&self, id: u32) -> Option<PointerValue<'ctx>> {
        self.descriptors.get(&id).copied()
    }

    /// Records the descriptor of tuple type `id`.
    pub(crate) fn add_descriptor(&mut self, id: u32, pointer: PointerValue<'ctx>) {
        self.descriptors.insert(id, pointer);
    }

    /// Adds a private constant global holding `value`, and returns its address.
    pub(crate) fn global(
        module: &Module<'ctx>,
        value: BasicValueEnum<'ctx>,
        name: &str,
    ) -> PointerValue<'ctx> {
        let global = module.add_global(value.get_type(), None, name);
        global.set_initializer(&value);
        global.set_constant(true);
        global.set_linkage(Linkage::Private);
        global.set_unnamed_addr(true);
        global.as_pointer_value()
    }

    /// Returns a `str` object: header, length in code points, kind, then the code points (detailed design section 14.1).
    pub(crate) fn string(&mut self, module: &Module<'ctx>, text: &str) -> PointerValue<'ctx> {
        if let Some(pointer) = self.strings.get(text) {
            return *pointer;
        }
        let context = self.context;
        let (kind, points) = encode_str(text);
        let unit = match kind {
            StrKind::Latin1 => context.i8_type(),
            StrKind::Ucs2 => context.i16_type(),
            StrKind::Ucs4 => context.i32_type(),
        };
        let data: Vec<IntValue<'ctx>> = points
            .iter()
            .map(|point| unit.const_int(u64::from(*point), false))
            .collect();
        let value = context.const_struct(
            &[
                context
                    .i32_type()
                    .const_int(u64::from(IMMORTAL), false)
                    .into(),
                context
                    .i32_type()
                    .const_int(u64::from(STR_TYPE_ID), false)
                    .into(),
                context
                    .i64_type()
                    .const_int(points.len() as u64, false)
                    .into(),
                context.i32_type().const_int(kind as u64, false).into(),
                context.i32_type().const_zero().into(),
                unit.const_array(&data).into(),
            ],
            false,
        );
        let pointer = Self::global(
            module,
            value.into(),
            &format!("typy.str.{}", self.strings.len()),
        );
        self.strings.insert(text.to_owned(), pointer);
        pointer
    }

    /// Returns a null-terminated byte string, for the names in traceback entries.
    pub(crate) fn c_string(&mut self, module: &Module<'ctx>, text: &str) -> PointerValue<'ctx> {
        if let Some(pointer) = self.c_strings.get(text) {
            return *pointer;
        }
        let value = self.context.const_string(text.as_bytes(), true);
        let pointer = Self::global(
            module,
            value.into(),
            &format!("typy.cstr.{}", self.c_strings.len()),
        );
        self.c_strings.insert(text.to_owned(), pointer);
        pointer
    }

    /// Returns an `int` constant as a word: a tagged value, or the address of a static large integer.
    pub(crate) fn int(&mut self, module: &Module<'ctx>, digits: &str) -> IntEncodingValue<'ctx> {
        match encode_int(digits) {
            #[allow(
                clippy::cast_sign_loss,
                reason = "the tagged word is passed on bit for bit"
            )]
            IntEncoding::Inline(word) => {
                IntEncodingValue::Word(self.context.i64_type().const_int(word as u64, true))
            }
            IntEncoding::Big { negative, limbs } => {
                if let Some(pointer) = self.big_ints.get(digits) {
                    return IntEncodingValue::Object(*pointer);
                }
                let context = self.context;
                let length = i64::try_from(limbs.len()).unwrap_or(i64::MAX);
                let sign_and_length = if negative { -length } else { length };
                let data: Vec<IntValue<'ctx>> = limbs
                    .iter()
                    .map(|limb| context.i64_type().const_int(*limb, false))
                    .collect();
                #[allow(
                    clippy::cast_sign_loss,
                    reason = "the signed length is stored bit for bit"
                )]
                let value = context.const_struct(
                    &[
                        context
                            .i32_type()
                            .const_int(u64::from(IMMORTAL), false)
                            .into(),
                        context
                            .i32_type()
                            .const_int(u64::from(BIG_INT_TYPE_ID), false)
                            .into(),
                        context
                            .i32_type()
                            .const_int(sign_and_length as u64, true)
                            .into(),
                        context.i32_type().const_zero().into(),
                        context.i64_type().const_array(&data).into(),
                    ],
                    false,
                );
                let pointer = Self::global(
                    module,
                    value.into(),
                    &format!("typy.int.{}", self.big_ints.len()),
                );
                self.big_ints.insert(digits.to_owned(), pointer);
                IntEncodingValue::Object(pointer)
            }
        }
    }
}

/// An `int` constant: a tagged word, or a static large integer to convert to a word.
pub(crate) enum IntEncodingValue<'ctx> {
    Word(IntValue<'ctx>),
    Object(PointerValue<'ctx>),
}
