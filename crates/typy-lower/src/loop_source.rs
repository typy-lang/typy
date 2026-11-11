//! Lowers `for` over a list or a `str`, and over `enumerate`, `zip` and `reversed` (CST-03,
//! LIB-02), as CPython's iterators behave. Each sequence is evaluated once and kept in a hidden
//! slot with its position. Taking the next value checks the position against the length read
//! again, so items appended in the body are visited and a list that shrinks ends the loop, then
//! moves the position on before the body runs.

use typy_check::{AssignTarget, Generator, LoopSource, TypedExpr, TypedStmt};
use typy_diagnostics::Span;
use typy_tyir::{
    BlockId, BoolOperator, Comparison, Constant, IntOperator, Operation, RuntimeFunction,
    ScalarKind, SlotId, Terminator, TyirType, ValueId,
};

use crate::function::{FunctionLowerer, LoopTargets};
use crate::ownership::Lowered;

/// What kind of sequence a loop reads, which decides how it reads its length and items.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Kind {
    /// A list or a `tuple[T, ...]`.
    List,
    /// A `str`, whose items are strings of one code point.
    Str,
    /// A `bytes`, whose items are `int` values.
    Bytes,
    /// A set, read slot by slot in its table's order.
    Set,
}

/// The state of one iterator of a loop, kept in hidden slots.
enum Iterator {
    /// A set: its slot, and its number of items when the loop began, which must not change.
    Set {
        set: SlotId,
        slot: SlotId,
        length: SlotId,
        item: TyirType,
    },
    /// A dict or a view of one: the next entry, the number of entries when the loop began, and
    /// how many entries it has given, which CPython's dict iterator checks at every step.
    Dict {
        source: SlotId,
        position: SlotId,
        length: SlotId,
        visited: SlotId,
        shows: typy_check::View,
        through_view: bool,
        ty: TyirType,
    },
    /// A `range`: the next value, the limit it stops at and the step, and whether the step
    /// rises when it is a constant.
    Range {
        counter: SlotId,
        limit: SlotId,
        stride: SlotId,
        rising: Option<bool>,
    },
    /// A list or `str`, read forwards or, for `reversed`, backwards.
    Sequence {
        sequence: SlotId,
        position: SlotId,
        kind: Kind,
        backwards: bool,
        item: TyirType,
    },
    /// `enumerate`: the count, and what is counted.
    Enumerate {
        count: SlotId,
        inner: Box<Iterator>,
        ty: TyirType,
    },
    /// An iterator that `iter()` made: its slot, and the type of its items (CST-09).
    Object { iterator: SlotId, item: TyirType },
    /// An instance of an iterator class: the slot of what `__iter__` returned, and its
    /// `__next__` (CST-08).
    Protocol {
        iterator: SlotId,
        next: typy_check::FunctionId,
        item: TyirType,
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
            cleanups: self.cleanups.len(),
        });
        self.statements(body);
        self.loops.pop();
        self.jump_to(header);
        self.builder.switch_to(otherwise);
        self.statements(orelse);
        self.jump_to(exit);
        self.builder.switch_to(exit);
    }

    /// Returns `list(source)`, a new list of every value the source gives. A list is copied
    /// directly; any other source is read as a `for` loop reads it, appending each value.
    pub(crate) fn list_of(&mut self, source: &LoopSource, span: Span) -> Lowered {
        if let LoopSource::Items(sequence) = source
            && sequence.ty.sequence_item().is_some()
        {
            let list = self.expression(sequence);
            let copy = self.call_runtime(RuntimeFunction::ListCopy, vec![list.value], span);
            self.release(list, span);
            return copy;
        }
        let item = self.lowered_type(&source.item_type());
        let list = self.list_display(&[], item, span);
        self.each_value(source, span, |lowerer, value| {
            let bits = lowerer.bits(value, span);
            lowerer.call_runtime(RuntimeFunction::ListAppend, vec![list.value, bits], span);
        });
        list
    }

    /// Lowers the clauses from the first, each a loop around the next, and calls `innermost` in the
    /// innermost one for each value that meets every condition.
    pub(crate) fn generate(
        &mut self,
        generators: &[Generator],
        span: Span,
        innermost: &mut dyn FnMut(&mut Self),
    ) {
        let Some((first, rest)) = generators.split_first() else {
            innermost(self);
            return;
        };
        let iterator = self.start_iterator(&first.source, span);
        let header = self.builder.create_block();
        let exit = self.builder.create_block();
        self.jump_to(header);
        self.builder.switch_to(header);
        let value = self.next_value(&iterator, exit, span);
        self.assign_target(&first.target, value, span);
        for condition in &first.conditions {
            let tested = self.expression(condition);
            let holds = self.truth(tested, span);
            self.release(tested, span);
            let next = self.builder.create_block();
            self.builder.terminate(typy_tyir::Terminator::Branch {
                condition: holds,
                then: next,
                otherwise: header,
            });
            self.builder.switch_to(next);
        }
        self.generate(rest, span, innermost);
        self.jump_to(header);
        self.builder.switch_to(exit);
    }

    /// Reads `source` as a `for` loop reads it and calls `visit` with each value, which is
    /// released after the call.
    pub(crate) fn each_value(
        &mut self,
        source: &LoopSource,
        span: Span,
        mut visit: impl FnMut(&mut Self, Lowered),
    ) {
        let iterator = self.start_iterator(source, span);
        let header = self.builder.create_block();
        let exit = self.builder.create_block();
        self.jump_to(header);
        self.builder.switch_to(header);
        let value = self.next_value(&iterator, exit, span);
        visit(self, value);
        self.release(value, span);
        self.jump_to(header);
        self.builder.switch_to(exit);
    }

    /// Evaluates the source's expressions in order and keeps them in hidden slots.
    fn start_iterator(&mut self, source: &LoopSource, span: Span) -> Iterator {
        match source {
            LoopSource::Items(sequence) if let typy_check::Type::Iterator(item) = &sequence.ty => {
                let iterator = self.kept(sequence, "iterator", span);
                Iterator::Object {
                    iterator,
                    item: self.lowered_type(item),
                }
            }
            LoopSource::Items(sequence) => self.start_sequence(sequence, false, span),
            LoopSource::Range {
                start,
                stop,
                step,
                reversed,
            } => self.start_range([start, stop, step], *reversed, span),
            LoopSource::Reversed(sequence) => self.start_sequence(sequence, true, span),
            LoopSource::Protocol {
                iterable,
                iter,
                next,
                iterator,
                item,
            } => {
                // CPython calls `__iter__` once, when the loop starts.
                let lowered = self.expression(iterable);
                let iterable_value = Self::borrowed(lowered.value, lowered.ty);
                let ty = self.lowered_type(iterator);
                let made = self.call_function(*iter, vec![(0, iterable_value)], ty, span);
                self.release(lowered, span);
                let slot = self.hidden_slot("iterator", ty);
                self.store(slot, made, span);
                Iterator::Protocol {
                    iterator: slot,
                    next: *next,
                    item: self.lowered_type(item),
                }
            }
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
        if let typy_check::Type::Dict(_, _) | typy_check::Type::DictView(..) = &sequence.ty {
            return self.start_dict(sequence, span);
        }
        if let typy_check::Type::Set(item) = &sequence.ty {
            let item = self.lowered_type(item);
            return self.start_set(sequence, item, span);
        }
        let (kind, item) = match (&sequence.ty, sequence.ty.sequence_item()) {
            (_, Some(item)) => (Kind::List, self.lowered_type(item)),
            (typy_check::Type::Bytes, None) => (Kind::Bytes, TyirType::Int),
            _ => (Kind::Str, TyirType::STR),
        };
        let sequence = self.kept(sequence, "sequence", span);
        let position = self.hidden_slot("position", TyirType::Int);
        let first = if backwards {
            let length = self.sequence_length(sequence, kind, span);
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
            kind,
            backwards,
            item,
        }
    }

    /// Evaluates the arguments of `range` once, in order, raises `ValueError` for a zero step,
    /// and keeps the counter, the limit and the step. `reversed` starts from the last value and
    /// steps back to `start - step`, as CPython's `range_reverse` does.
    fn start_range(&mut self, arguments: [&TypedExpr; 3], reversed: bool, span: Span) -> Iterator {
        let [first, end, increment] = arguments;
        let counter = self.hidden_slot("counter", TyirType::Int);
        let limit = self.hidden_slot("limit", TyirType::Int);
        let stride = self.hidden_slot("stride", TyirType::Int);
        for (slot, value) in [(counter, first), (limit, end), (stride, increment)] {
            let lowered = self.expression(value);
            self.store(slot, lowered, span);
        }
        let constant = match &increment.kind {
            typy_check::TypedExprKind::Constant(typy_check::TypedConstant::Int(digits))
                if digits != "0" =>
            {
                Some(!digits.starts_with('-'))
            }
            _ => None,
        };
        if constant.is_none() {
            let value = self.load(stride, span);
            self.call_runtime(RuntimeFunction::RangeCheckStep, vec![value.value], span);
        }
        if reversed {
            let (first, end, increment) = (
                self.load(counter, span),
                self.load(limit, span),
                self.load(stride, span),
            );
            let last = self.call_runtime(
                RuntimeFunction::RangeLast,
                vec![first.value, end.value, increment.value],
                span,
            );
            let before = self.builder.push_value(
                Operation::IntBinary {
                    operator: IntOperator::Subtract,
                    left: first.value,
                    right: increment.value,
                },
                TyirType::Int,
                span,
            );
            let before = self.produced(before, TyirType::Int);
            let back = self.builder.push_value(
                Operation::IntUnary {
                    operator: typy_tyir::IntUnaryOperator::Negate,
                    operand: increment.value,
                },
                TyirType::Int,
                span,
            );
            let back = self.produced(back, TyirType::Int);
            self.store(limit, before, span);
            self.store(stride, back, span);
            self.store(counter, last, span);
        }
        Iterator::Range {
            counter,
            limit,
            stride,
            rising: constant.map(|rising| rising != reversed),
        }
    }

    /// Returns the range's next value with a reference of its own, or jumps to `exhausted`.
    fn next_in_range(
        &mut self,
        [counter, limit, stride]: [SlotId; 3],
        rising: Option<bool>,
        exhausted: BlockId,
        span: Span,
    ) -> Lowered {
        let condition = self.range_condition([counter, limit, stride], rising, span);
        let fetch = self.builder.create_block();
        self.builder.terminate(Terminator::Branch {
            condition,
            then: fetch,
            otherwise: exhausted,
        });
        self.builder.switch_to(fetch);
        let current = self.load(counter, span);
        self.builder.push(Operation::Increment(current.value), span);
        let value = self.produced(current.value, TyirType::Int);
        let increment = self.load(stride, span);
        let next = self.builder.push_value(
            Operation::IntBinary {
                operator: IntOperator::Add,
                left: current.value,
                right: increment.value,
            },
            TyirType::Int,
            span,
        );
        let next = self.produced(next, TyirType::Int);
        self.store(counter, next, span);
        value
    }

    /// Keeps a dict or a view of one, the first entry to look at, the number of entries and
    /// none visited.
    fn start_dict(&mut self, source: &TypedExpr, span: Span) -> Iterator {
        let (shows, through_view, item) = match &source.ty {
            typy_check::Type::DictView(view, key, value) => (*view, true, view.item(key, value)),
            typy_check::Type::Dict(key, _) => (typy_check::View::Keys, false, key.as_ref().clone()),
            _ => unreachable!("a dict source is a dict or a view"),
        };
        let ty = self.lowered_type(&item);
        let source = self.kept(source, "dict", span);
        let zero = || Constant::Int("0".to_owned());
        let position = self.hidden_slot("position", TyirType::Int);
        let start = Self::borrowed(self.constant(zero(), span), TyirType::Int);
        self.store(position, start, span);
        let visited = self.hidden_slot("visited", TyirType::Int);
        let none = Self::borrowed(self.constant(zero(), span), TyirType::Int);
        self.store(visited, none, span);
        let length = self.hidden_slot("length", TyirType::Int);
        let dict = self.iterated_dict(source, through_view, span);
        let count = self.call_runtime(RuntimeFunction::DictLength, vec![dict], span);
        self.store(length, count, span);
        Iterator::Dict {
            source,
            position,
            length,
            visited,
            shows,
            through_view,
            ty,
        }
    }

    /// Returns the dict a loop reads, borrowed from the dict or view kept in `source`.
    fn iterated_dict(&mut self, source: SlotId, through_view: bool, span: Span) -> ValueId {
        let value = self.load(source, span);
        if !through_view {
            return value.value;
        }
        let dict = self.call_runtime(RuntimeFunction::DictViewDict, vec![value.value], span);
        self.borrowed_result(dict).value
    }

    /// Returns the next key, value or `(key, value)` pair of a dict, or jumps to `exhausted`;
    /// a dict that changed raises `RuntimeError`, as CPython's dict iterator does.
    fn next_dict_entry(&mut self, iterator: &Iterator, exhausted: BlockId, span: Span) -> Lowered {
        let Iterator::Dict {
            source,
            position,
            length,
            visited,
            shows,
            through_view,
            ty,
        } = iterator
        else {
            unreachable!("a dict iterator")
        };
        let dict = self.iterated_dict(*source, *through_view, span);
        let at = self.load(*position, span);
        let count = self.load(*length, span);
        let seen = self.load(*visited, span);
        let found = self.call_runtime(
            RuntimeFunction::DictNext,
            vec![dict, at.value, count.value, seen.value],
            span,
        );
        let zero = self.constant(Constant::Int("0".to_owned()), span);
        let more = self.int_compare(Comparison::GreaterEqual, found.value, zero, span);
        let fetch = self.builder.create_block();
        self.builder.terminate(Terminator::Branch {
            condition: more,
            then: fetch,
            otherwise: exhausted,
        });
        self.builder.switch_to(fetch);
        let read = |lowerer: &mut Self, function, ty| {
            let bits = lowerer.call_runtime(function, vec![dict, found.value], span);
            let value = lowerer.builder.push_value(
                Operation::FromBits {
                    bits: bits.value,
                    ty,
                },
                ty,
                span,
            );
            Self::borrowed(value, ty)
        };
        let fetched = match shows {
            typy_check::View::Keys => read(self, RuntimeFunction::DictKey, *ty),
            typy_check::View::Values => read(self, RuntimeFunction::DictValue, *ty),
            typy_check::View::Items => {
                let TyirType::Tuple(id) = ty else {
                    unreachable!("the items of a dict are pairs")
                };
                let types = self.tuple_types.items(*id).unwrap_or_default().to_vec();
                let key = read(self, RuntimeFunction::DictKey, types[0]);
                let value = read(self, RuntimeFunction::DictValue, types[1]);
                self.make_tuple(*ty, vec![key, value], span)
            }
        };
        let next = self.add_int(found, 1, span);
        self.store(*position, next, span);
        let seen = self.load(*visited, span);
        let seen = self.add_int(seen, 1, span);
        self.store(*visited, seen, span);
        fetched
    }

    /// Keeps a set, the first slot to look at, and its number of items, which CPython's set
    /// iterator checks at every step.
    fn start_set(&mut self, set: &TypedExpr, item: TyirType, span: Span) -> Iterator {
        let set = self.kept(set, "set", span);
        let slot = self.hidden_slot("slot", TyirType::Int);
        let zero = Self::borrowed(
            self.constant(Constant::Int("0".to_owned()), span),
            TyirType::Int,
        );
        self.store(slot, zero, span);
        let length = self.hidden_slot("length", TyirType::Int);
        let count = self.sequence_length(set, Kind::Set, span);
        self.store(length, count, span);
        Iterator::Set {
            set,
            slot,
            length,
            item,
        }
    }

    /// Returns the set's next item, borrowed from the set, or jumps to `exhausted`; a set whose
    /// number of items changed raises `RuntimeError`.
    fn next_set_item(
        &mut self,
        set: SlotId,
        slot: SlotId,
        length: SlotId,
        item: TyirType,
        exhausted: BlockId,
        span: Span,
    ) -> Lowered {
        let value = self.load(set, span);
        let position = self.load(slot, span);
        let count = self.load(length, span);
        let found = self.call_runtime(
            RuntimeFunction::SetNext,
            vec![value.value, position.value, count.value],
            span,
        );
        let zero = self.constant(Constant::Int("0".to_owned()), span);
        let more = self.int_compare(Comparison::GreaterEqual, found.value, zero, span);
        let fetch = self.builder.create_block();
        self.builder.terminate(Terminator::Branch {
            condition: more,
            then: fetch,
            otherwise: exhausted,
        });
        self.builder.switch_to(fetch);
        let bits = self.call_runtime(
            RuntimeFunction::SetItem,
            vec![value.value, found.value],
            span,
        );
        let fetched = self.builder.push_value(
            Operation::FromBits {
                bits: bits.value,
                ty: item,
            },
            item,
            span,
        );
        let next = self.add_int(found, 1, span);
        self.store(slot, next, span);
        Self::borrowed(fetched, item)
    }

    fn sequence_length(&mut self, sequence: SlotId, kind: Kind, span: Span) -> Lowered {
        let value = self.load(sequence, span);
        let function = match kind {
            Kind::List => RuntimeFunction::ListLength,
            Kind::Str => RuntimeFunction::StrLength,
            Kind::Bytes => RuntimeFunction::BytesLength,
            Kind::Set => RuntimeFunction::SetLength,
        };
        self.call_runtime(function, vec![value.value], span)
    }

    /// Returns the iterator's next value, or jumps to `exhausted` when it has none.
    fn next_value(&mut self, iterator: &Iterator, exhausted: BlockId, span: Span) -> Lowered {
        match iterator {
            Iterator::Range {
                counter,
                limit,
                stride,
                rising,
            } => self.next_in_range([*counter, *limit, *stride], *rising, exhausted, span),
            Iterator::Dict { .. } => self.next_dict_entry(iterator, exhausted, span),
            Iterator::Protocol {
                iterator,
                next,
                item,
            } => {
                let current = self.load(*iterator, span);
                self.call_next(*next, current.value, *item, exhausted, span)
            }
            Iterator::Object { iterator, item } => {
                let current = self.load(*iterator, span);
                let keep_going = self.constant(Constant::Int("0".to_owned()), span);
                let found = self.call_runtime(
                    RuntimeFunction::IteratorAdvance,
                    vec![current.value, keep_going],
                    span,
                );
                let fetch = self.builder.create_block();
                self.builder.terminate(Terminator::Branch {
                    condition: found.value,
                    then: fetch,
                    otherwise: exhausted,
                });
                self.builder.switch_to(fetch);
                self.current_item(current.value, *item, span)
            }
            Iterator::Set {
                set,
                slot,
                length,
                item,
            } => self.next_set_item(*set, *slot, *length, *item, exhausted, span),
            Iterator::Sequence {
                sequence,
                position,
                kind,
                backwards,
                item,
            } => self.next_item(
                *sequence, *position, *kind, *backwards, *item, exhausted, span,
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
        kind: Kind,
        backwards: bool,
        item: TyirType,
        exhausted: BlockId,
        span: Span,
    ) -> Lowered {
        let length = self.sequence_length(sequence, kind, span);
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
        let fetched = match kind {
            Kind::Str => self.call_runtime(
                RuntimeFunction::StrIndex,
                vec![value.value, index.value],
                span,
            ),
            Kind::Bytes => self.call_runtime(
                RuntimeFunction::BytesGet,
                vec![value.value, index.value],
                span,
            ),
            Kind::List => self.list_item(value, index, item, span),
            Kind::Set => unreachable!("a set is read by its own iterator"),
        };
        let current = self.load(position, span);
        let next = self.add_int(current, if backwards { -1 } else { 1 }, span);
        self.store(position, next, span);
        fetched
    }

    /// Returns a tuple of `parts`, which takes over a reference to each.
    pub(crate) fn make_tuple(&mut self, ty: TyirType, parts: Vec<Lowered>, span: Span) -> Lowered {
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
