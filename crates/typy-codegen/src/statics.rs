//! Emits static objects: string and large-integer constants, and C strings for tracebacks.

use std::collections::BTreeMap;

use inkwell::context::Context;
use inkwell::module::{Linkage, Module};
use inkwell::values::{BasicValueEnum, IntValue, PointerValue};

use crate::layout::{
    BIG_INT_TYPE_ID, BYTES_TYPE_ID, IMMORTAL, IntEncoding, STR_TYPE_ID, StrKind, encode_int,
    encode_str, str_kind,
};

/// Creates each static object once per module.
pub(crate) struct Statics<'ctx> {
    context: &'ctx Context,
    strings: BTreeMap<String, PointerValue<'ctx>>,
    c_strings: BTreeMap<String, PointerValue<'ctx>>,
    big_ints: BTreeMap<String, PointerValue<'ctx>>,
    bytes: BTreeMap<String, PointerValue<'ctx>>,
    point_strings: BTreeMap<u32, PointerValue<'ctx>>,
    descriptors: BTreeMap<u32, PointerValue<'ctx>>,
    type_objects: BTreeMap<u32, PointerValue<'ctx>>,
    modules: BTreeMap<u32, PointerValue<'ctx>>,
    type_aliases: BTreeMap<u32, PointerValue<'ctx>>,
    typing_values: BTreeMap<u32, PointerValue<'ctx>>,
    function_objects: BTreeMap<u32, PointerValue<'ctx>>,
    method_descriptors: BTreeMap<u32, PointerValue<'ctx>>,
    closure_descriptors: BTreeMap<u32, PointerValue<'ctx>>,
    generator_descriptors: BTreeMap<u32, PointerValue<'ctx>>,
    builtin_identities: BTreeMap<String, PointerValue<'ctx>>,
    function_descriptors: BTreeMap<u32, PointerValue<'ctx>>,
}

impl<'ctx> Statics<'ctx> {
    pub(crate) fn new(context: &'ctx Context) -> Self {
        Self {
            context,
            strings: BTreeMap::new(),
            c_strings: BTreeMap::new(),
            big_ints: BTreeMap::new(),
            bytes: BTreeMap::new(),
            point_strings: BTreeMap::new(),
            descriptors: BTreeMap::new(),
            type_objects: BTreeMap::new(),
            modules: BTreeMap::new(),
            type_aliases: BTreeMap::new(),
            typing_values: BTreeMap::new(),
            function_objects: BTreeMap::new(),
            method_descriptors: BTreeMap::new(),
            closure_descriptors: BTreeMap::new(),
            generator_descriptors: BTreeMap::new(),
            builtin_identities: BTreeMap::new(),
            function_descriptors: BTreeMap::new(),
        }
    }

    /// Returns the descriptor of nested function `id`, if it was emitted already (FUN-04).
    pub(crate) fn closure_descriptor(&self, id: u32) -> Option<PointerValue<'ctx>> {
        self.closure_descriptors.get(&id).copied()
    }

    /// Records the descriptor of nested function `id`.
    pub(crate) fn add_closure_descriptor(&mut self, id: u32, pointer: PointerValue<'ctx>) {
        self.closure_descriptors.insert(id, pointer);
    }

    /// Returns the descriptor of the resume function `id`, if it was emitted already (SMT-11).
    pub(crate) fn generator_descriptor(&self, id: u32) -> Option<PointerValue<'ctx>> {
        self.generator_descriptors.get(&id).copied()
    }

    /// Records the descriptor of the resume function `id`.
    pub(crate) fn add_generator_descriptor(&mut self, id: u32, pointer: PointerValue<'ctx>) {
        self.generator_descriptors.insert(id, pointer);
    }

    /// Returns the identity the hidden functions of the built-in `name` share: the descriptor
    /// of the first one emitted, which is `own` for that first one (ADR-0076).
    pub(crate) fn builtin_identity(
        &mut self,
        name: &str,
        own: PointerValue<'ctx>,
    ) -> PointerValue<'ctx> {
        *self
            .builtin_identities
            .entry(name.to_owned())
            .or_insert(own)
    }

    /// Returns the descriptor of the function object of function `id`, if it was emitted.
    pub(crate) fn function_descriptor(&self, id: u32) -> Option<PointerValue<'ctx>> {
        self.function_descriptors.get(&id).copied()
    }

    /// Records the descriptor of the function object of function `id`.
    pub(crate) fn add_function_descriptor(&mut self, id: u32, pointer: PointerValue<'ctx>) {
        self.function_descriptors.insert(id, pointer);
    }

    /// Returns the bound method descriptor of method `id`, if it was emitted already.
    pub(crate) fn method_descriptor(&self, id: u32) -> Option<PointerValue<'ctx>> {
        self.method_descriptors.get(&id).copied()
    }

    /// Records the bound method descriptor of method `id`.
    pub(crate) fn add_method_descriptor(&mut self, id: u32, pointer: PointerValue<'ctx>) {
        self.method_descriptors.insert(id, pointer);
    }

    /// Returns the function object of function `id`, if it was emitted already (FUN-05).
    pub(crate) fn function_object(&self, id: u32) -> Option<PointerValue<'ctx>> {
        self.function_objects.get(&id).copied()
    }

    /// Records the function object of function `id`.
    pub(crate) fn add_function_object(&mut self, id: u32, pointer: PointerValue<'ctx>) {
        self.function_objects.insert(id, pointer);
    }

    /// Returns the descriptor of tuple type `id`, if it was emitted already.
    pub(crate) fn descriptor(&self, id: u32) -> Option<PointerValue<'ctx>> {
        self.descriptors.get(&id).copied()
    }

    /// Records the descriptor of tuple type `id`.
    pub(crate) fn add_descriptor(&mut self, id: u32, pointer: PointerValue<'ctx>) {
        self.descriptors.insert(id, pointer);
    }

    /// Returns the type object of class `index`, if it was emitted already.
    pub(crate) fn type_object(&self, index: u32) -> Option<PointerValue<'ctx>> {
        self.type_objects.get(&index).copied()
    }

    /// Records the type object of class `index`.
    pub(crate) fn add_type_object(&mut self, index: u32, pointer: PointerValue<'ctx>) {
        self.type_objects.insert(index, pointer);
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
        let (_, points) = encode_str(text);
        let pointer = self.text_object(
            module,
            &points,
            STR_TYPE_ID,
            &format!("typy.str.{}", self.strings.len()),
        );
        self.strings.insert(text.to_owned(), pointer);
        pointer
    }

    /// Returns the `str` object of the string at `index` of the module's strings that hold a
    /// surrogate, given as its code points (LEX-10).
    pub(crate) fn str_points(
        &mut self,
        module: &Module<'ctx>,
        index: u32,
        points: &[u32],
    ) -> PointerValue<'ctx> {
        if let Some(pointer) = self.point_strings.get(&index) {
            return *pointer;
        }
        let pointer = self.text_object(
            module,
            points,
            STR_TYPE_ID,
            &format!("typy.str.points.{index}"),
        );
        self.point_strings.insert(index, pointer);
        pointer
    }

    /// Returns a `bytes` object: a `str` object of one byte per code point under the `bytes`
    /// type ID, each byte held as the code point of its value in `text`.
    pub(crate) fn bytes(&mut self, module: &Module<'ctx>, text: &str) -> PointerValue<'ctx> {
        if let Some(pointer) = self.bytes.get(text) {
            return *pointer;
        }
        let (_, points) = encode_str(text);
        let pointer = self.text_object(
            module,
            &points,
            BYTES_TYPE_ID,
            &format!("typy.bytes.{}", self.bytes.len()),
        );
        self.bytes.insert(text.to_owned(), pointer);
        pointer
    }

    fn text_object(
        &mut self,
        module: &Module<'ctx>,
        points: &[u32],
        type_id: u32,
        name: &str,
    ) -> PointerValue<'ctx> {
        let context = self.context;
        let kind = str_kind(points);
        let unit = match kind {
            StrKind::Latin1 => context.i8_type(),
            StrKind::Ucs2 => context.i16_type(),
            StrKind::Ucs4 => context.i32_type(),
        };
        // A zero unit follows the data, as it follows every string the runtime makes, so a
        // `bytes` constant is a string C reads (ADR-0047).
        let data: Vec<IntValue<'ctx>> = points
            .iter()
            .map(|point| unit.const_int(u64::from(*point), false))
            .chain([unit.const_zero()])
            .collect();
        let value = context.const_struct(
            &[
                context
                    .i32_type()
                    .const_int(u64::from(IMMORTAL), false)
                    .into(),
                context
                    .i32_type()
                    .const_int(u64::from(type_id), false)
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
        Self::global(module, value.into(), name)
    }

    /// Returns the module object of the module at `index`: an immortal header, then its name
    /// and its path as `str` objects (MOD-10). Each module has one, so `is` compares them.
    pub(crate) fn module_object(
        &mut self,
        module: &Module<'ctx>,
        index: u32,
        name: &str,
        path: &str,
    ) -> PointerValue<'ctx> {
        if let Some(pointer) = self.modules.get(&index) {
            return *pointer;
        }
        let pointer = self.string_object(
            module,
            crate::layout::MODULE_TYPE_ID,
            &[name, path],
            &format!("typy.module.{index}"),
        );
        self.modules.insert(index, pointer);
        pointer
    }

    /// Returns the type alias object at `index`: an immortal header, then the alias's name,
    /// its module's name and its qualified name as `str` objects (SMT-12, TYP-18). Each alias
    /// of a module has one, so `is` compares them; one a function defines is its description,
    /// which each run of the `type` statement copies.
    pub(crate) fn type_alias_object(
        &mut self,
        module: &Module<'ctx>,
        index: u32,
        [name, module_name, qualified_name]: [&str; 3],
    ) -> PointerValue<'ctx> {
        if let Some(pointer) = self.type_aliases.get(&index) {
            return *pointer;
        }
        let pointer = self.string_object(
            module,
            crate::layout::TYPE_ALIAS_TYPE_ID,
            &[name, module_name, qualified_name],
            &format!("typy.type_alias.{index}"),
        );
        self.type_aliases.insert(index, pointer);
        pointer
    }

    /// Returns the typing value at `index`: an immortal header, then its text and its key as
    /// `str` objects (TYP-18). Each distinct value has one.
    pub(crate) fn typing_value_object(
        &mut self,
        module: &Module<'ctx>,
        index: u32,
        text: &str,
        key: &str,
    ) -> PointerValue<'ctx> {
        if let Some(pointer) = self.typing_values.get(&index) {
            return *pointer;
        }
        let pointer = self.string_object(
            module,
            crate::layout::TYPING_VALUE_TYPE_ID,
            &[text, key],
            &format!("typy.typing_value.{index}"),
        );
        self.typing_values.insert(index, pointer);
        pointer
    }

    /// Returns a new immortal static object of type `type_id` that holds `str` objects of
    /// `texts`.
    fn string_object(
        &mut self,
        module: &Module<'ctx>,
        type_id: u32,
        texts: &[&str],
        symbol: &str,
    ) -> PointerValue<'ctx> {
        let context = self.context;
        let mut fields: Vec<inkwell::values::BasicValueEnum<'ctx>> = vec![
            context
                .i32_type()
                .const_int(u64::from(IMMORTAL), false)
                .into(),
            context
                .i32_type()
                .const_int(u64::from(type_id), false)
                .into(),
        ];
        for text in texts {
            fields.push(self.string(module, text).into());
        }
        let value = context.const_struct(&fields, false);
        let global = module.add_global(value.get_type(), None, symbol);
        global.set_initializer(&value);
        global.set_constant(true);
        global.set_linkage(Linkage::Private);
        global.as_pointer_value()
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
