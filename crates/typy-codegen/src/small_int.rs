//! Writes `int` arithmetic, comparisons and reference counts on inline values in place, and
//! calls the runtime only for a large integer or a result outside the inline range (ADR-0102).
//!
//! An inline word is `2 * value + 1`. Two inline words added as `left + (right - 1)` give the
//! inline word of the sum, and the 64-bit addition overflows exactly when the sum leaves the
//! inline range, so LLVM's overflow intrinsics decide when the runtime takes over.

use inkwell::IntPredicate;
use inkwell::basic_block::BasicBlock;
use inkwell::intrinsics::Intrinsic;
use inkwell::values::{BasicValueEnum, IntValue, ValueKind};
use typy_tyir::{Comparison, IntOperator, IntUnaryOperator};

use crate::error::CodegenError;
use crate::function::FunctionEmitter;
use crate::runtime::{Internal, internal_function};

/// What the inline path of an operation gives: a word, or no word when the result leaves the
/// inline range and the runtime computes it.
struct InPlace<'ctx> {
    value: IntValue<'ctx>,
    overflowed: Option<IntValue<'ctx>>,
}

impl<'ctx> FunctionEmitter<'_, '_, 'ctx> {
    /// Returns `left op right` of two `int` words.
    pub(super) fn int_binary_in_place(
        &mut self,
        operator: IntOperator,
        left: IntValue<'ctx>,
        right: IntValue<'ctx>,
    ) -> Result<IntValue<'ctx>, CodegenError> {
        let function = match operator {
            IntOperator::Add => Internal::IntAdd,
            IntOperator::Subtract => Internal::IntSubtract,
            IntOperator::Multiply => Internal::IntMultiply,
            IntOperator::BitAnd => Internal::IntAnd,
            IntOperator::BitOr => Internal::IntOr,
            IntOperator::BitXor => Internal::IntXor,
        };
        let inline = self.both_inline(left, right)?;
        self.in_place(
            inline,
            |this| this.binary_inline(operator, left, right),
            |this| this.call_word(function, &[left, right]),
        )
    }

    /// Returns `-operand` or `~operand` of an `int` word.
    pub(super) fn int_unary_in_place(
        &mut self,
        operator: IntUnaryOperator,
        operand: IntValue<'ctx>,
    ) -> Result<IntValue<'ctx>, CodegenError> {
        let function = match operator {
            IntUnaryOperator::Negate => Internal::IntNegate,
            IntUnaryOperator::Invert => Internal::IntInvert,
        };
        let inline = self.is_inline(operand)?;
        self.in_place(
            inline,
            |this| {
                let word = this.shared.context.i64_type();
                Ok(match operator {
                    // `2 - (2 * value + 1)` is `2 * -value + 1`; only `-(-2**62)` leaves the range.
                    IntUnaryOperator::Negate => {
                        this.with_overflow("ssub", word.const_int(2, false), operand)?
                    }
                    // Flipping every bit but the lowest gives the word of `~value`.
                    IntUnaryOperator::Invert => InPlace {
                        value: this.builder.build_xor(
                            operand,
                            word.const_int(!1_u64, false),
                            "",
                        )?,
                        overflowed: None,
                    },
                })
            },
            |this| this.call_word(function, &[operand]),
        )
    }

    /// Returns whether `left` and `right`, two `int` words, stand in `comparison`: inline words
    /// order as the values they hold, so two of them compare as words.
    pub(super) fn int_compare_in_place(
        &mut self,
        comparison: Comparison,
        left: IntValue<'ctx>,
        right: IntValue<'ctx>,
    ) -> Result<IntValue<'ctx>, CodegenError> {
        let predicate = match comparison {
            Comparison::Equal => IntPredicate::EQ,
            Comparison::NotEqual => IntPredicate::NE,
            Comparison::Less => IntPredicate::SLT,
            Comparison::LessEqual => IntPredicate::SLE,
            Comparison::Greater => IntPredicate::SGT,
            Comparison::GreaterEqual => IntPredicate::SGE,
        };
        let inline = self.both_inline(left, right)?;
        self.in_place(
            inline,
            |this| {
                Ok(InPlace {
                    value: this.builder.build_int_compare(predicate, left, right, "")?,
                    overflowed: None,
                })
            },
            |this| {
                let callee = internal_function(
                    this.shared.context,
                    this.shared.module,
                    Internal::IntCompare,
                );
                let code = this.tagged(comparison.code());
                let byte = this
                    .builder
                    .build_call(callee, &[left.into(), right.into(), code.into()], "")?
                    .try_as_basic_value();
                match byte {
                    ValueKind::Basic(value) => this.byte_to_bool(value.into_int_value()),
                    ValueKind::Instruction(_) => {
                        Err(CodegenError("int_compare returns a byte".to_owned()))
                    }
                }
            },
        )
    }

    /// Adds or drops a reference to an `int` word: only a word that points to a large integer
    /// has one, so an inline word skips the call.
    pub(super) fn count_int_in_place(
        &mut self,
        word_value: IntValue<'ctx>,
        increment: bool,
    ) -> Result<(), CodegenError> {
        let function = if increment {
            Internal::IntIncrement
        } else {
            Internal::IntDecrement
        };
        let inline = self.is_inline(word_value)?;
        let context = self.shared.context;
        let large = context.append_basic_block(self.llvm_function, "large_int");
        let after = context.append_basic_block(self.llvm_function, "");
        let inline = self.expect_true(inline)?;
        self.builder
            .build_conditional_branch(inline, after, large)?;
        self.builder.position_at_end(large);
        let callee = internal_function(context, self.shared.module, function);
        self.builder.build_call(callee, &[word_value.into()], "")?;
        self.builder.build_unconditional_branch(after)?;
        self.builder.position_at_end(after);
        Ok(())
    }

    /// Returns the inline path's result of `left op right`, both inline.
    fn binary_inline(
        &mut self,
        operator: IntOperator,
        left: IntValue<'ctx>,
        right: IntValue<'ctx>,
    ) -> Result<InPlace<'ctx>, CodegenError> {
        let word = self.shared.context.i64_type();
        let one = word.const_int(1, false);
        let plain = |value: IntValue<'ctx>| InPlace {
            value,
            overflowed: None,
        };
        Ok(match operator {
            IntOperator::Add => {
                let untagged = self.builder.build_int_sub(right, one, "")?;
                self.with_overflow("sadd", left, untagged)?
            }
            IntOperator::Subtract => {
                let untagged = self.builder.build_int_sub(right, one, "")?;
                self.with_overflow("ssub", left, untagged)?
            }
            // `value * (2 * other)` is even and inside the range when it does not overflow, so
            // setting its lowest bit tags it.
            IntOperator::Multiply => {
                let value = self.builder.build_right_shift(left, one, true, "")?;
                let untagged = self.builder.build_int_sub(right, one, "")?;
                let product = self.with_overflow("smul", value, untagged)?;
                InPlace {
                    value: self.builder.build_or(product.value, one, "")?,
                    overflowed: product.overflowed,
                }
            }
            IntOperator::BitAnd => plain(self.builder.build_and(left, right, "")?),
            IntOperator::BitOr => plain(self.builder.build_or(left, right, "")?),
            IntOperator::BitXor => {
                let bits = self.builder.build_xor(left, right, "")?;
                plain(self.builder.build_or(bits, one, "")?)
            }
        })
    }

    /// Returns `llvm.NAME.with.overflow` of two words, the result and whether it overflowed.
    fn with_overflow(
        &mut self,
        name: &str,
        left: IntValue<'ctx>,
        right: IntValue<'ctx>,
    ) -> Result<InPlace<'ctx>, CodegenError> {
        let word = self.shared.context.i64_type();
        let declaration = Intrinsic::find(&format!("llvm.{name}.with.overflow"))
            .and_then(|intrinsic| intrinsic.get_declaration(self.shared.module, &[word.into()]))
            .ok_or_else(|| CodegenError(format!("llvm.{name}.with.overflow is missing")))?;
        let ValueKind::Basic(pair) = self
            .builder
            .build_call(declaration, &[left.into(), right.into()], "")?
            .try_as_basic_value()
        else {
            return Err(CodegenError(format!(
                "llvm.{name}.with.overflow returns no value"
            )));
        };
        let pair = pair.into_struct_value();
        Ok(InPlace {
            value: self
                .builder
                .build_extract_value(pair, 0, "")?
                .into_int_value(),
            overflowed: Some(
                self.builder
                    .build_extract_value(pair, 1, "")?
                    .into_int_value(),
            ),
        })
    }

    /// Emits an operation's inline path where `inline` holds and its call of the runtime
    /// otherwise, or where the inline path overflows, and returns the result where they join.
    fn in_place(
        &mut self,
        inline: IntValue<'ctx>,
        fast: impl FnOnce(&mut Self) -> Result<InPlace<'ctx>, CodegenError>,
        slow: impl FnOnce(&mut Self) -> Result<IntValue<'ctx>, CodegenError>,
    ) -> Result<IntValue<'ctx>, CodegenError> {
        let context = self.shared.context;
        let fast_block = context.append_basic_block(self.llvm_function, "inline_int");
        let slow_block = context.append_basic_block(self.llvm_function, "large_int");
        let join = context.append_basic_block(self.llvm_function, "");
        let inline = self.expect_true(inline)?;
        self.builder
            .build_conditional_branch(inline, fast_block, slow_block)?;
        self.builder.position_at_end(fast_block);
        let result = fast(self)?;
        let fast_end = self.current_block()?;
        match result.overflowed {
            Some(overflowed) => {
                let fits = self.builder.build_not(overflowed, "")?;
                let fits = self.expect_true(fits)?;
                self.builder
                    .build_conditional_branch(fits, join, slow_block)?;
            }
            None => {
                self.builder.build_unconditional_branch(join)?;
            }
        }
        self.builder.position_at_end(slow_block);
        let called = slow(self)?;
        let slow_end = self.current_block()?;
        self.builder.build_unconditional_branch(join)?;
        self.builder.position_at_end(join);
        let phi = self.builder.build_phi(result.value.get_type(), "")?;
        phi.add_incoming(&[(&result.value, fast_end), (&called, slow_end)]);
        Ok(phi.as_basic_value().into_int_value())
    }

    /// Returns the result of a runtime function of `int` words that returns one.
    fn call_word(
        &mut self,
        function: Internal,
        arguments: &[IntValue<'ctx>],
    ) -> Result<IntValue<'ctx>, CodegenError> {
        let callee = internal_function(self.shared.context, self.shared.module, function);
        let arguments: Vec<_> = arguments.iter().map(|value| (*value).into()).collect();
        match self
            .builder
            .build_call(callee, &arguments, "")?
            .try_as_basic_value()
        {
            ValueKind::Basic(BasicValueEnum::IntValue(value)) => Ok(value),
            _ => Err(CodegenError(format!("{function:?} returns no word"))),
        }
    }

    /// Returns whether the word holds an inline value: its lowest bit is set.
    fn is_inline(&self, value: IntValue<'ctx>) -> Result<IntValue<'ctx>, CodegenError> {
        let word = self.shared.context.i64_type();
        let bit = self
            .builder
            .build_and(value, word.const_int(1, false), "")?;
        Ok(self
            .builder
            .build_int_compare(IntPredicate::NE, bit, word.const_zero(), "")?)
    }

    /// Returns whether both words hold inline values.
    fn both_inline(
        &self,
        left: IntValue<'ctx>,
        right: IntValue<'ctx>,
    ) -> Result<IntValue<'ctx>, CodegenError> {
        let both = self.builder.build_and(left, right, "")?;
        self.is_inline(both)
    }

    /// Returns `condition`, marked as almost always true, so the inline path is laid out first.
    fn expect_true(&self, condition: IntValue<'ctx>) -> Result<IntValue<'ctx>, CodegenError> {
        let flag = self.shared.context.bool_type();
        let declaration = Intrinsic::find("llvm.expect")
            .and_then(|intrinsic| intrinsic.get_declaration(self.shared.module, &[flag.into()]))
            .ok_or_else(|| CodegenError("llvm.expect is missing".to_owned()))?;
        match self
            .builder
            .build_call(
                declaration,
                &[condition.into(), flag.const_int(1, false).into()],
                "",
            )?
            .try_as_basic_value()
        {
            ValueKind::Basic(BasicValueEnum::IntValue(value)) => Ok(value),
            _ => Err(CodegenError("llvm.expect returns no flag".to_owned())),
        }
    }

    /// Returns the block the builder writes into.
    fn current_block(&self) -> Result<BasicBlock<'ctx>, CodegenError> {
        self.builder
            .get_insert_block()
            .ok_or_else(|| CodegenError("no current block".to_owned()))
    }
}
