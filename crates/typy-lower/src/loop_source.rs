//! Lowers `for` over a list or a `str`, and over `enumerate`, `zip` and `reversed` (CST-03,
//! LIB-02), as CPython's iterators behave. Each sequence is evaluated once and kept in a hidden
//! slot with its position. Taking the next value checks the position against the length read
//! again, so items appended in the body are visited and a list that shrinks ends the loop, then
//! moves the position on before the body runs.

use typy_check::{AssignTarget, LoopSource, TypedExpr, TypedStmt};
use typy_diagnostics::Span;
use typy_tyir::{
    BlockId, BoolOperator, Comparison, Constant, IntOperator, Operation, RuntimeFunction,
    ScalarKind, SlotId, Terminator, TyirType, ValueId,
};

use crate::function::{FunctionLowerer, LoopTargets};
use crate::ownership::Lowered;

/// The state of one iterator of a loop, kept in hidden slots.
enum Iterator {
    /// A list or `str`, read forwards or, for `reversed`, backwards.
    Sequence {
        sequence: SlotId,
        position: SlotId,
        is_str: bool,
        backwards: bool,
        item: TyirType,
    },
    /// `enumerate`: the count, and what is counted.
    Enumerate {
        count: SlotId,
        inner: Box<Iterator>,
        ty: TyirType,
    },
    /// `zip`: the iterators, and the slot of `strict` when it is given.
    Zip {
        inners: Vec<Iterator>,
        strict: Option<SlotId>,
        ty: TyirType,
    },
}

impl FunctionLowerer<'_> {
    /// Lowers `for target in source`, with its body and `else` clause.
    pub(crate) fn for_each(
        &mut self,
        target: &AssignTarget,
        source: &LoopSource,
        body: &[TypedStmt],
        orelse: &[TypedStmt],
        span: Span,
    ) {
        let iterator = self.start_iterator(source, span);
        let header = self.builder.create_block();
        let otherwise = self.builder.create_block();
        let exit = self.builder.create_block();
        self.jump_to(header);
        self.builder.switch_to(header);
        let value = self.next_value(&iterator, otherwise, span);
        self.assign_target(target, value, span);
        self.loops.push(LoopTargets {
            break_target: exit,
            continue_target: header,
        });
        self.statements(body);
        self.loops.pop();
        self.jump_to(header);
        self.builder.switch_to(otherwise);
        self.statements(orelse);
        self.jump_to(exit);
        self.builder.switch_to(exit);
    }

    /// Evaluates the source's expressions in order and keeps them in hidden slots.
    fn start_iterator(&mut self, source: &LoopSource, span: Span) -> Iterator {
        match source {
            LoopSource::Items(sequence) => self.start_sequence(sequence, false, span),
            LoopSource::Reversed(sequence) => self.start_sequence(sequence, true, span),
            LoopSource::Enumerate {
                source: inner,
                start,
            } => {
                let inner = self.start_iterator(inner, span);
                let count = self.kept(start, "count", span);
                let ty = self.lowered_type(&source.item_type());
                Iterator::Enumerate {
                    count,
                    inner: Box::new(inner),
                    ty,
                }
            }
            LoopSource::Zip { sources, strict } => {
                let inners = sources
                    .iter()
                    .map(|inner| self.start_iterator(inner, span))
                    .collect();
                let strict = strict
                    .as_ref()
                    .map(|strict| self.kept(strict, "strict", span));
                let ty = self.lowered_type(&source.item_type());
                Iterator::Zip { inners, strict, ty }
            }
        }
    }

    fn kept(&mut self, expression: &TypedExpr, name: &str, span: Span) -> SlotId {
        let ty = self.lowered_type(&expression.ty);
        let slot = self.hidden_slot(name, ty);
        let lowered = self.expression(expression);
        self.store(slot, lowered, span);
        slot
    }

    /// Keeps a sequence and its first position: 0, or for `reversed` its length less one, as
    /// CPython's `reversed` reads the length once when it is called.
    fn start_sequence(&mut self, sequence: &TypedExpr, backwards: bool, span: Span) -> Iterator {
        let item = match &sequence.ty {
            typy_check::Type::List(item) => self.lowered_type(item),
            _ => TyirType::STR,
        };
        let is_str = item == TyirType::STR && !matches!(sequence.ty, typy_check::Type::List(_));
        let sequence = self.kept(sequence, "sequence", span);
        let position = self.hidden_slot("position", TyirType::Int);
        let first = if backwards {
            let length = self.sequence_length(sequence, is_str, span);
            self.add_int(length, -1, span)
        } else {
            Self::borrowed(
                self.constant(Constant::Int("0".to_owned()), span),
                TyirType::Int,
            )
        };
        self.store(position, first, span);
        Iterator::Sequence {
            sequence,
            position,
            is_str,
            backwards,
            item,
        }
    }

    fn sequence_length(&mut self, sequence: SlotId, is_str: bool, span: Span) -> Lowered {
        let value = self.load(sequence, span);
        let function = if is_str {
            RuntimeFunction::StrLength
        } else {
            RuntimeFunction::ListLength
        };
        self.call_runtime(function, vec![value.value], span)
    }

    /// Returns the iterator's next value, or jumps to `exhausted` when it has none.
    fn next_value(&mut self, iterator: &Iterator, exhausted: BlockId, span: Span) -> Lowered {
        match iterator {
            Iterator::Sequence {
                sequence,
                position,
                is_str,
                backwards,
                item,
            } => self.next_item(
                *sequence, *position, *is_str, *backwards, *item, exhausted, span,
            ),
            Iterator::Enumerate { count, inner, ty } => {
                let value = self.next_value(inner, exhausted, span);
                let current = self.load(*count, span);
                let pair = self.make_tuple(*ty, vec![current, value], span);
                let current = self.load(*count, span);
                let next = self.add_int(current, 1, span);
                self.store(*count, next, span);
                pair
            }
            Iterator::Zip { inners, strict, ty } => {
                if inners.is_empty() {
                    self.builder.terminate(Terminator::Jump {
                        target: exhausted,
                        arguments: Vec::new(),
                    });
                    let unreachable = self.builder.create_block();
                    self.builder.switch_to(unreachable);
                    return self.make_tuple(*ty, Vec::new(), span);
                }
                let mut values: Vec<Lowered> = Vec::with_capacity(inners.len());
                for (index, inner) in inners.iter().enumerate() {
                    let ended = self.builder.create_block();
                    // The path where this source ran out has only what was tracked before it
                    // was asked, so the value it would have given is not tracked there.
                    let before = self.temporaries.clone();
                    let value = self.next_value(inner, ended, span);
                    let after = self.builder.create_block();
                    self.jump_to(after);
                    let fetched = std::mem::replace(&mut self.temporaries, before);
                    self.builder.switch_to(ended);
                    let drop = self.builder.create_block();
                    if let Some(strict) = strict {
                        self.check_strict(*strict, inners, index, drop, span);
                    }
                    self.jump_to(drop);
                    self.builder.switch_to(drop);
                    // The values taken from the earlier sources this step are not bound.
                    for taken in &values {
                        self.release(*taken, span);
                    }
                    self.jump_to(exhausted);
                    self.temporaries = fetched;
                    self.builder.switch_to(after);
                    values.push(value);
                }
                self.make_tuple(*ty, values, span)
            }
        }
    }

    #[allow(
        clippy::too_many_arguments,
        reason = "the fields of one sequence iterator"
    )]
    fn next_item(
        &mut self,
        sequence: SlotId,
        position: SlotId,
        is_str: bool,
        backwards: bool,
        item: TyirType,
        exhausted: BlockId,
        span: Span,
    ) -> Lowered {
        let length = self.sequence_length(sequence, is_str, span);
        let index = self.load(position, span);
        let mut more = self.int_compare(Comparison::Less, index.value, length.value, span);
        self.release(length, span);
        if backwards {
            let zero = self.constant(Constant::Int("0".to_owned()), span);
            let not_negative = self.int_compare(Comparison::GreaterEqual, index.value, zero, span);
            more = self.builder.push_value(
                Operation::BoolBinary {
                    operator: BoolOperator::And,
                    left: more,
                    right: not_negative,
                },
                TyirType::Bool,
                span,
            );
        }
        let fetch = self.builder.create_block();
        self.builder.terminate(Terminator::Branch {
            condition: more,
            then: fetch,
            otherwise: exhausted,
        });
        self.builder.switch_to(fetch);
        let value = self.load(sequence, span);
        let index = self.load(position, span);
        let fetched = if is_str {
            self.call_runtime(
                RuntimeFunction::StrIndex,
                vec![value.value, index.value],
                span,
            )
        } else {
            self.list_item(value, index, item, span)
        };
        let current = self.load(position, span);
        let next = self.add_int(current, if backwards { -1 } else { 1 }, span);
        self.store(position, next, span);
        fetched
    }

    /// Returns a tuple of `parts`, which takes over a reference to each.
    fn make_tuple(&mut self, ty: TyirType, parts: Vec<Lowered>, span: Span) -> Lowered {
        let TyirType::Tuple(id) = ty else {
            unreachable!("`enumerate` and `zip` give tuples")
        };
        let items = parts
            .into_iter()
            .map(|part| self.take_reference(part, span))
            .collect();
        let value = self
            .builder
            .push_value(Operation::MakeTuple { ty: id, items }, ty, span);
        self.produced(value, ty)
    }

    /// Where source `ended` of a strict `zip` ran out: a later source is shorter than the ones
    /// before it; for the first, each other source that still gives a value is longer. Either
    /// raises `ValueError`, as CPython does; otherwise the loop goes on to `exhausted`.
    fn check_strict(
        &mut self,
        strict: SlotId,
        inners: &[Iterator],
        ended: usize,
        exhausted: BlockId,
        span: Span,
    ) {
        let strict = self.load(strict, span);
        let check = self.builder.create_block();
        self.builder.terminate(Terminator::Branch {
            condition: strict.value,
            then: check,
            otherwise: exhausted,
        });
        self.builder.switch_to(check);
        if ended > 0 {
            self.raise_unequal(ended, false, span);
            return;
        }
        for (index, inner) in inners.iter().enumerate().skip(1) {
            let shorter = self.builder.create_block();
            let before = self.temporaries.clone();
            let value = self.next_value(inner, shorter, span);
            self.release(value, span);
            self.raise_unequal(index, true, span);
            self.jump_to(exhausted);
            self.temporaries = before;
            self.builder.switch_to(shorter);
        }
    }

    fn raise_unequal(&mut self, index: usize, longer: bool, span: Span) {
        let index = self.constant(Constant::Int(index.to_string()), span);
        let longer = self.constant(Constant::Int(u8::from(longer).to_string()), span);
        self.call_runtime(RuntimeFunction::ZipUnequal, vec![index, longer], span);
    }

    fn int_compare(
        &mut self,
        comparison: Comparison,
        left: ValueId,
        right: ValueId,
        span: Span,
    ) -> ValueId {
        self.builder.push_value(
            Operation::Compare {
                kind: ScalarKind::Int,
                comparison,
                left,
                right,
            },
            TyirType::Bool,
            span,
        )
    }

    /// Returns `value + amount` for an `int`, releasing `value`.
    fn add_int(&mut self, value: Lowered, amount: i64, span: Span) -> Lowered {
        let amount = self.constant(Constant::Int(amount.to_string()), span);
        let sum = self.builder.push_value(
            Operation::IntBinary {
                operator: IntOperator::Add,
                left: value.value,
                right: amount,
            },
            TyirType::Int,
            span,
        );
        self.release(value, span);
        self.produced(sum, TyirType::Int)
    }
}
