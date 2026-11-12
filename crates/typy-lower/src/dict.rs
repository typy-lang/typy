//! Lowers dicts: displays, `dict(source)`, entries read and assigned, operators, views and
//! methods (DAT-09, LIB-03). Each runtime dict function borrows its arguments; one that stores a
//! key or a value takes a reference of its own.

use typy_check::{LoopSource, Method, Type, TypedExpr, View};
use typy_diagnostics::Span;
use typy_tyir::{Constant, Operation, RuntimeFunction, TyirType, ValueId};

use crate::function::FunctionLowerer;
use crate::ownership::Lowered;

impl FunctionLowerer<'_> {
    /// Returns the TyIR types of the keys and values of a dict or view type.
    pub(crate) fn dict_types(&mut self, ty: &Type) -> (TyirType, TyirType) {
        match ty {
            Type::Dict(key, value) | Type::DictView(_, key, value) => {
                (self.lowered_type(key), self.lowered_type(value))
            }
            _ => unreachable!("a dict expression has a dict type"),
        }
    }

    /// Returns a new empty dict with a table for `capacity` entries.
    pub(crate) fn new_dict(
        &mut self,
        key: TyirType,
        value: TyirType,
        capacity: usize,
        span: Span,
    ) -> Lowered {
        let (key_kind, key_descriptor) = self.item_layout(key, span);
        let (value_kind, value_descriptor) = self.item_layout(value, span);
        let capacity = self.constant(Constant::Int(capacity.to_string()), span);
        self.call_runtime(
            RuntimeFunction::DictNew,
            vec![
                key_kind,
                key_descriptor,
                value_kind,
                value_descriptor,
                capacity,
            ],
            span,
        )
    }

    /// Lowers a dict display: a dict presized for its entries, as CPython's `BUILD_MAP` makes
    /// it, then each key and value set in order.
    pub(crate) fn dict_display(
        &mut self,
        keys: &[TypedExpr],
        values: &[TypedExpr],
        ty: &Type,
        span: Span,
    ) -> Lowered {
        let (key_type, value_type) = self.dict_types(ty);
        let dict = self.new_dict(key_type, value_type, keys.len(), span);
        for (key, value) in keys.iter().zip(values) {
            let key = self.expression(key);
            let value = self.expression(value);
            self.set_entry(dict.value, key, value, span);
            self.release(value, span);
            self.release(key, span);
        }
        dict
    }

    /// Sets `dict[key] = value`; neither is released, and an owned value is handed to the
    /// dict, which releasing it afterwards passes over (MEM-09).
    pub(crate) fn set_entry(&mut self, dict: ValueId, key: Lowered, value: Lowered, span: Span) {
        let key = self.bits(key, span);
        let bits = self.bits(value, span);
        self.store_call(
            RuntimeFunction::DictSet,
            RuntimeFunction::DictSetMoved,
            vec![dict, key, bits],
            value,
            span,
        );
    }

    /// Returns `dict(source)`: a dict merged from a dict, as CPython merges one, or the pairs
    /// of any other source set in order.
    pub(crate) fn dict_of(&mut self, source: &LoopSource, ty: &Type, span: Span) -> Lowered {
        let (key_type, value_type) = self.dict_types(ty);
        let dict = self.new_dict(key_type, value_type, 0, span);
        if let LoopSource::Items(sequence) = source
            && matches!(sequence.ty, Type::Dict(..))
        {
            let other = self.expression(sequence);
            let address = self.address(other.value, span);
            self.call_runtime(RuntimeFunction::DictUpdate, vec![dict.value, address], span);
            self.release(other, span);
            return dict;
        }
        self.each_value(source, span, |lowerer, pair| {
            let key = lowerer.borrowed_item(pair, 0, key_type, span);
            let value = lowerer.borrowed_item(pair, 1, value_type, span);
            lowerer.set_entry(dict.value, key, value, span);
        });
        dict
    }

    /// Returns `dict[key]`, or raises `KeyError`. A value of a dict the function owns gets its
    /// own reference before the dict is released; the key is released.
    pub(crate) fn dict_item(
        &mut self,
        dict: Lowered,
        key: Lowered,
        value: TyirType,
        span: Span,
    ) -> Lowered {
        let key_bits = self.bits(key, span);
        let bits = self.call_runtime(RuntimeFunction::DictGet, vec![dict.value, key_bits], span);
        self.release(key, span);
        let found = self.value_of_bits(bits.value, value, span);
        if !dict.owned {
            return Self::borrowed(found, value);
        }
        let kept = self.owned_copy(found, value, span);
        self.release(dict, span);
        kept
    }

    /// Returns a value of type `ty` made from the bits a runtime function gave.
    fn value_of_bits(&mut self, bits: ValueId, ty: TyirType, span: Span) -> ValueId {
        self.builder
            .push_value(Operation::FromBits { bits, ty }, ty, span)
    }

    /// Returns a borrowed value with a reference of its own, so it outlives what it was read
    /// from.
    fn owned_copy(&mut self, value: ValueId, ty: TyirType, span: Span) -> Lowered {
        if !self.counted(ty) {
            return Self::borrowed(value, ty);
        }
        self.builder.push(Operation::Increment(value), span);
        self.produced(value, ty)
    }

    /// Stores a value into `dict[key]`; the value was evaluated first, then the dict and the
    /// key are, as CPython does. The dict takes references of its own, so all are released.
    pub(crate) fn assign_entry(
        &mut self,
        dict: &TypedExpr,
        key: &TypedExpr,
        value: Lowered,
        span: Span,
    ) {
        let dict = self.expression(dict);
        let key = self.expression(key);
        self.set_entry(dict.value, key, value, span);
        self.release(value, span);
        self.release(key, span);
        self.release(dict, span);
    }

    /// Lowers `dict[key] op= value`: the dict and the key are evaluated once, and the entry is
    /// read, raising `KeyError` when it is missing, before the value is evaluated.
    pub(crate) fn augmented_entry(
        &mut self,
        dict: &TypedExpr,
        key: &TypedExpr,
        compute: impl FnOnce(&mut Self, Lowered) -> Lowered,
        span: Span,
    ) {
        let (_, value_type) = self.dict_types(&dict.ty);
        let dict = self.expression(dict);
        let key = self.expression(key);
        let key_bits = self.bits(key, span);
        let bits = self.call_runtime(RuntimeFunction::DictGet, vec![dict.value, key_bits], span);
        let current = self.value_of_bits(bits.value, value_type, span);
        // The entry keeps a reference of its own while the value is evaluated, which may change
        // the dict.
        let current = self.owned_copy(current, value_type, span);
        let result = compute(self, current);
        self.set_entry(dict.value, key, result, span);
        self.release(result, span);
        self.release(key, span);
        self.release(dict, span);
    }

    /// Returns `left | right`, a copy of `left` updated with `right`; the operands are not
    /// released.
    pub(crate) fn dict_merge(&mut self, left: ValueId, right: ValueId, span: Span) -> Lowered {
        let merged = self.call_runtime(RuntimeFunction::DictCopy, vec![left], span);
        self.dict_update(merged.value, right, span);
        merged
    }

    /// Sets each entry of `other`, a dict or a list of pairs, in `dict`; neither is released.
    pub(crate) fn dict_update(&mut self, dict: ValueId, other: ValueId, span: Span) {
        let address = self.address(other, span);
        self.call_runtime(RuntimeFunction::DictUpdate, vec![dict, address], span);
    }

    /// Returns whether `item` is a key of a dict, or is in a view of one, which reads its own
    /// kind; the operands are not released.
    pub(crate) fn dict_contains(
        &mut self,
        item: Lowered,
        container: Lowered,
        span: Span,
    ) -> ValueId {
        let bits = self.bits(item, span);
        if container.ty == TyirType::DICT {
            return self
                .call_runtime(
                    RuntimeFunction::DictContains,
                    vec![container.value, bits],
                    span,
                )
                .value;
        }
        // A pair looked for among the items is read through its tuple type's descriptor.
        let pair = if let TyirType::Tuple(id) = item.ty {
            self.constant(Constant::TupleDescriptor(id), span)
        } else {
            let none = self.constant(Constant::Unit, span);
            self.builder
                .push_value(Operation::ToBits(none), TyirType::Bits, span)
        };
        self.call_runtime(
            RuntimeFunction::DictViewContains,
            vec![container.value, bits, pair],
            span,
        )
        .value
    }

    /// Calls a method of a dict with its arguments in source order; the result has type
    /// `result`.
    pub(crate) fn dict_method(
        &mut self,
        dict: ValueId,
        method: Method,
        arguments: &[Lowered],
        result: TyirType,
        span: Span,
    ) -> Lowered {
        match method {
            Method::DictGet | Method::DictSetdefault | Method::DictPop => {
                self.dict_lookup(dict, method, arguments, result, span)
            }
            Method::DictPopitem => {
                let TyirType::Tuple(id) = result else {
                    unreachable!("`popitem` gives a pair")
                };
                let types = self.tuple_types.items(id).unwrap_or_default().to_vec();
                let position = self.call_runtime(RuntimeFunction::DictPopitem, vec![dict], span);
                let key =
                    self.call_runtime(RuntimeFunction::DictKey, vec![dict, position.value], span);
                let value =
                    self.call_runtime(RuntimeFunction::DictValue, vec![dict, position.value], span);
                self.release(position, span);
                // The dict gives up its references to the key and the value.
                let key = self.value_of_bits(key.value, types[0], span);
                let key = self.produced(key, types[0]);
                let value = self.value_of_bits(value.value, types[1], span);
                let value = self.produced(value, types[1]);
                self.make_tuple(result, vec![key, value], span)
            }
            Method::DictKeys | Method::DictValues | Method::DictItems => {
                let view = match method {
                    Method::DictKeys => View::Keys,
                    Method::DictValues => View::Values,
                    _ => View::Items,
                };
                let code = self.constant(Constant::Int(view.code().to_string()), span);
                self.call_runtime(RuntimeFunction::DictView, vec![dict, code], span)
            }
            Method::DictUpdate => {
                if let Some(other) = arguments.first() {
                    self.dict_update(dict, other.value, span);
                }
                Self::borrowed(self.constant(Constant::Unit, span), TyirType::Unit)
            }
            Method::DictClear => self.call_runtime(RuntimeFunction::DictClear, vec![dict], span),
            Method::DictCopy => self.call_runtime(RuntimeFunction::DictCopy, vec![dict], span),
            method => unreachable!("`{}` is not a dict method", method.name()),
        }
    }

    /// Lowers `get`, `setdefault` and `pop`, which find a key and give its value.
    fn dict_lookup(
        &mut self,
        dict: ValueId,
        method: Method,
        arguments: &[Lowered],
        result: TyirType,
        span: Span,
    ) -> Lowered {
        if method == Method::DictGet && arguments.len() == 1 {
            return self.get_optional(dict, arguments[0], result, span);
        }
        let key = self.bits(arguments[0], span);
        let default = match arguments.get(1) {
            Some(default) => Some(self.bits(*default, span)),
            // `setdefault` without a default stores `None` of an optional value type.
            None if method == Method::DictSetdefault => {
                let none = self.none_of(result, span);
                Some(
                    self.builder
                        .push_value(Operation::ToBits(none), TyirType::Bits, span),
                )
            }
            None => None,
        };
        let (function, owned) = match (method, default) {
            (Method::DictPop, None) => (RuntimeFunction::DictPop, true),
            (Method::DictPop, Some(_)) => (RuntimeFunction::DictPopOr, true),
            (Method::DictGet, _) => (RuntimeFunction::DictGetOr, false),
            _ => (RuntimeFunction::DictSetdefault, false),
        };
        let arguments = std::iter::once(dict)
            .chain(std::iter::once(key))
            .chain(default)
            .collect();
        let bits = self.call_runtime(function, arguments, span);
        let found = self.value_of_bits(bits.value, result, span);
        if owned {
            // The dict gives up its reference to the value.
            return self.produced(found, result);
        }
        // The value is borrowed from the dict or the default, both released after the call, so
        // it takes a reference of its own.
        self.owned_copy(found, result, span)
    }

    /// Returns `dict.get(key)`, a `V | None`: the value of `key` with a reference of its own,
    /// or `None` when it is missing (TYP-06).
    fn get_optional(
        &mut self,
        dict: ValueId,
        key: Lowered,
        result: TyirType,
        span: Span,
    ) -> Lowered {
        let key = self.bits(key, span);
        let position = self.call_runtime(RuntimeFunction::DictFind, vec![dict, key], span);
        let zero = self.constant(Constant::Int("0".to_owned()), span);
        let found = self.builder.push_value(
            Operation::Compare {
                kind: typy_tyir::ScalarKind::Int,
                comparison: typy_tyir::Comparison::GreaterEqual,
                left: position.value,
                right: zero,
            },
            TyirType::Bool,
            span,
        );
        let (some, none, join) = (
            self.builder.create_block(),
            self.builder.create_block(),
            self.builder.create_block(),
        );
        let joined = self.builder.add_block_parameter(join, result);
        self.builder.terminate(typy_tyir::Terminator::Branch {
            condition: found,
            then: some,
            otherwise: none,
        });
        self.builder.switch_to(some);
        let base = self.base_type(result);
        let bits = self.call_runtime(RuntimeFunction::DictValue, vec![dict, position.value], span);
        let value = self.value_of_bits(bits.value, base, span);
        let wrapped = self
            .builder
            .push_value(Operation::Wrap { value, ty: result }, result, span);
        let wrapped = self.take_reference(Self::borrowed(wrapped, result), span);
        self.builder.terminate(typy_tyir::Terminator::Jump {
            target: join,
            arguments: vec![wrapped],
        });
        self.builder.switch_to(none);
        let missing = self.none_of(result, span);
        self.builder.terminate(typy_tyir::Terminator::Jump {
            target: join,
            arguments: vec![missing],
        });
        self.builder.switch_to(join);
        self.release(position, span);
        self.produced(joined, result)
    }
}
