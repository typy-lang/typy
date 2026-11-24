//! Lowers `bytearray` (LIB-28, ADR-0108): its buffer, a `bytes`, is lent to the functions of
//! `bytes`, and a new `bytes` one of them gives becomes the buffer of a new bytearray.

use typy_check::Method;
use typy_diagnostics::Span;
use typy_tyir::{Constant, Operation, RuntimeFunction, TyirType};

use crate::function::FunctionLowerer;
use crate::ownership::Lowered;

impl FunctionLowerer<'_> {
    /// Returns the bytes a bytes-like value holds as a borrowed `bytes`: the value itself, or
    /// a bytearray's buffer, which the bytearray keeps.
    pub(crate) fn bytes_view(&mut self, value: Lowered, span: Span) -> Lowered {
        if value.ty != TyirType::BYTEARRAY {
            return Self::borrowed(value.value, value.ty);
        }
        let bits = self.call_runtime(RuntimeFunction::BytearrayBuffer, vec![value.value], span);
        let buffer = self.builder.push_value(
            Operation::FromBits {
                bits: bits.value,
                ty: TyirType::BYTES,
            },
            TyirType::BYTES,
            span,
        );
        Self::borrowed(buffer, TyirType::BYTES)
    }

    /// Returns a new bytearray of `data`, the new `bytes` a function of `bytes` gave, whose
    /// reference the runtime takes over: the `bytes` becomes the buffer, or is copied when
    /// another reference to it exists (ADR-0108).
    pub(crate) fn bytearray_of_bytes(&mut self, data: Lowered, span: Span) -> Lowered {
        debug_assert!(data.owned, "a function of `bytes` gives a new `bytes`");
        self.untrack(data.value);
        self.handed_over.insert(data.value);
        self.call_runtime(RuntimeFunction::BytearrayAdopt, vec![data.value], span)
    }

    /// Returns `bytearray(source)`: zeros for a count, the bytes of a list of `int`s, or a copy
    /// of a `bytes` or a bytearray. The source is not released.
    pub(crate) fn bytearray_value(&mut self, source: Lowered, span: Span) -> Lowered {
        let function = match source.ty {
            TyirType::Int => RuntimeFunction::BytearrayNew,
            TyirType::LIST => RuntimeFunction::BytearrayFromList,
            TyirType::BYTEARRAY => RuntimeFunction::BytearrayCopy,
            _ => RuntimeFunction::BytearrayFromBytes,
        };
        self.call_runtime(function, vec![source.value], span)
    }

    /// Calls `array.method(given)`: a method of a list that changes the bytearray, or a method
    /// of `bytes` called on its buffer, whose `bytes` results become bytearrays. Nothing is
    /// released.
    pub(crate) fn bytearray_method(
        &mut self,
        array: Lowered,
        method: Method,
        given: &[Option<Lowered>],
        result: TyirType,
        span: Span,
    ) -> Lowered {
        let arguments: Vec<Lowered> = given.iter().flatten().copied().collect();
        let (function, extra) = match (method, arguments.as_slice()) {
            (Method::ListAppend, [value]) => (RuntimeFunction::BytearrayAppend, vec![*value]),
            (Method::ListInsert, [index, value]) => {
                (RuntimeFunction::BytearrayInsert, vec![*index, *value])
            }
            (Method::ListPop, []) => {
                let last = self.constant(Constant::Int("-1".to_owned()), span);
                let last = Self::borrowed(last, TyirType::Int);
                (RuntimeFunction::BytearrayPop, vec![last])
            }
            (Method::ListPop, [index]) => (RuntimeFunction::BytearrayPop, vec![*index]),
            (Method::ListRemove, [value]) => (RuntimeFunction::BytearrayRemove, vec![*value]),
            (Method::ListReverse, []) => (RuntimeFunction::BytearrayReverse, Vec::new()),
            (Method::ListClear, []) => (RuntimeFunction::BytearrayClear, Vec::new()),
            (Method::ListCopy, []) => (RuntimeFunction::BytearrayCopy, Vec::new()),
            // A list and a `tuple[int, ...]` are both list objects.
            (Method::ListExtend, [source]) if source.ty == TyirType::LIST => {
                (RuntimeFunction::BytearrayExtendList, vec![*source])
            }
            (Method::ListExtend, [source]) => {
                let source = self.bytes_view(*source, span);
                (RuntimeFunction::BytearrayExtend, vec![source])
            }
            _ => return self.bytearray_bytes_method(array, method, given, result, span),
        };
        let operands = std::iter::once(array.value)
            .chain(extra.iter().map(|value| value.value))
            .collect();
        self.call_runtime(function, operands, span)
    }

    /// Calls the method of `bytes` on the buffer of `array`, with each bytearray argument read
    /// through its buffer, and gives each `bytes` of the result as a bytearray.
    fn bytearray_bytes_method(
        &mut self,
        array: Lowered,
        method: Method,
        given: &[Option<Lowered>],
        result: TyirType,
        span: Span,
    ) -> Lowered {
        let text = self.bytes_view(array, span);
        let given: Vec<Option<Lowered>> = given
            .iter()
            .map(|argument| argument.map(|argument| self.bytes_view(argument, span)))
            .collect();
        if matches!(method, Method::StrPartition | Method::StrRpartition) {
            let backwards = i64::from(method == Method::StrRpartition).to_string();
            let backwards = self.constant(Constant::Int(backwards), span);
            let separator = given
                .first()
                .copied()
                .flatten()
                .unwrap_or_else(|| unreachable!("`partition` takes a separator"));
            let parts = self.text_call(
                true,
                RuntimeFunction::StrPartition,
                vec![text.value, separator.value, backwards],
                span,
            );
            self.call_runtime(
                RuntimeFunction::BytearrayConvertList,
                vec![parts.value],
                span,
            );
            return self.three_parts(parts, result, span);
        }
        let bytes_result = if result == TyirType::BYTEARRAY {
            TyirType::BYTES
        } else {
            result
        };
        let value = self.str_method(text.value, true, method, &given, bytes_result, span);
        if result == TyirType::BYTEARRAY {
            return self.bytearray_of_bytes(value, span);
        }
        if result == TyirType::LIST {
            // `split`, `rsplit` and `splitlines` give a new list of `bytes`.
            self.call_runtime(
                RuntimeFunction::BytearrayConvertList,
                vec![value.value],
                span,
            );
        }
        value
    }

    /// Stores `value`, an `int`, at `array[index]`; the bytearray and the index are evaluated
    /// after the value, as for a list item (DAT-13), and everything is released afterwards.
    pub(crate) fn assign_byte(
        &mut self,
        array: &typy_check::TypedExpr,
        index: &typy_check::TypedExpr,
        value: Lowered,
        span: Span,
    ) {
        let array = self.operand_before(array, &[index], span);
        let index = self.expression(index);
        self.call_runtime(
            RuntimeFunction::BytearraySet,
            vec![array.value, index.value, value.value],
            span,
        );
        self.release(value, span);
        self.release(index, span);
        self.release(array, span);
    }

    /// Returns `left + right` or `left * right` where an operand is a bytearray: a new
    /// bytearray, or a `bytes` for `bytes + bytearray`, as CPython gives the type of the left
    /// operand. Neither operand is released.
    pub(crate) fn bytearray_binary(
        &mut self,
        left: Lowered,
        right: Lowered,
        span: Span,
    ) -> Lowered {
        let (function, operands) = match (left.ty, right.ty) {
            (TyirType::BYTEARRAY, TyirType::Int) => (
                RuntimeFunction::BytearrayRepeat,
                vec![left.value, right.value],
            ),
            (TyirType::Int, _) => (
                RuntimeFunction::BytearrayRepeat,
                vec![right.value, left.value],
            ),
            (TyirType::BYTEARRAY, _) => {
                let added = self.bytes_view(right, span);
                (
                    RuntimeFunction::BytearrayConcat,
                    vec![left.value, added.value],
                )
            }
            _ => {
                let added = self.bytes_view(right, span);
                (RuntimeFunction::BytesConcat, vec![left.value, added.value])
            }
        };
        self.call_runtime(function, operands, span)
    }
}
