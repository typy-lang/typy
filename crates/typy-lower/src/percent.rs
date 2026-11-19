//! Lowers `format % values` (LIB-10): the right operand is evaluated once, each conversion
//! formats its value by the runtime function for its kind, and the pieces are joined.

use typy_check::{PercentConversion, PercentField, PercentNumber, PercentPart, PercentValue};
use typy_diagnostics::Span;
use typy_tyir::{Constant, RuntimeFunction, TyirType};

use crate::function::FunctionLowerer;
use crate::ownership::Lowered;

impl FunctionLowerer<'_> {
    /// Returns the string `format % values` makes, from its pieces.
    pub(crate) fn percent_format(
        &mut self,
        values: &typy_check::TypedExpr,
        parts: &[PercentPart],
        span: Span,
    ) -> Lowered {
        let held = self.expression(values);
        let mut joined: Option<Lowered> = None;
        for part in parts {
            let piece = match part {
                PercentPart::Literal(text) => {
                    let value = self.string(text, span);
                    Self::borrowed(value, TyirType::STR)
                }
                PercentPart::Field(field) => self.percent_field(held, field, span),
            };
            joined = Some(match joined {
                None => piece,
                Some(before) => self.concatenated(before, piece, span),
            });
        }
        self.release(held, span);
        joined.unwrap_or_else(|| {
            let empty = self.string("", span);
            Self::borrowed(empty, TyirType::STR)
        })
    }

    /// Returns `before + piece`, releasing both.
    fn concatenated(&mut self, before: Lowered, piece: Lowered, span: Span) -> Lowered {
        let both = self.call_runtime(
            RuntimeFunction::StrConcat,
            vec![before.value, piece.value],
            span,
        );
        self.release(piece, span);
        self.release(before, span);
        both
    }

    /// Returns one conversion of its value.
    fn percent_field(&mut self, held: Lowered, field: &PercentField, span: Span) -> Lowered {
        let shared = Self::borrowed(held.value, held.ty);
        let ty = self.lowered_type(&field.ty);
        let value = match &field.value {
            PercentValue::Whole => shared,
            PercentValue::Item(index) => self.borrowed_item(shared, *index, ty, span),
            PercentValue::Key(key) => {
                let key = self.string(key, span);
                let key = Self::borrowed(key, TyirType::STR);
                self.dict_item(shared, key, ty, span)
            }
        };
        let spec = self.percent_spec(shared, field, span);
        let text = match field.conversion {
            PercentConversion::Str | PercentConversion::Repr | PercentConversion::Ascii => {
                let converted = self.percent_text(value, field.conversion, span);
                let text = self.call_runtime(
                    RuntimeFunction::PercentStr,
                    vec![converted.value, spec.value],
                    span,
                );
                self.release(converted, span);
                text
            }
            PercentConversion::Integer(kind) => {
                let integer = self.int_of(value, span);
                let kind = self.constant(Constant::Int(kind.to_string()), span);
                let text = self.call_runtime(
                    RuntimeFunction::PercentInt,
                    vec![integer.value, spec.value, kind],
                    span,
                );
                // Releasing a borrowed value does nothing, so this is the converted `int` only.
                self.release(integer, span);
                text
            }
            PercentConversion::Float(kind) => {
                let real = self.float_of(value, span);
                let kind = self.constant(Constant::Int(kind.to_string()), span);
                let text = self.call_runtime(
                    RuntimeFunction::PercentFloat,
                    vec![real.value, spec.value, kind],
                    span,
                );
                self.release(real, span);
                text
            }
            PercentConversion::Char if value.ty == TyirType::STR => {
                let label = self.string(&field.label, span);
                self.call_runtime(
                    RuntimeFunction::PercentCharStr,
                    vec![value.value, spec.value, label],
                    span,
                )
            }
            PercentConversion::Char => {
                let integer = self.int_of(value, span);
                let label = self.string(&field.label, span);
                let text = self.call_runtime(
                    RuntimeFunction::PercentChar,
                    vec![integer.value, spec.value, label],
                    span,
                );
                self.release(integer, span);
                text
            }
        };
        self.release(spec, span);
        self.release(value, span);
        text
    }

    /// Returns `str(value)`, `repr(value)` or `ascii(value)`.
    fn percent_text(
        &mut self,
        value: Lowered,
        conversion: PercentConversion,
        span: Span,
    ) -> Lowered {
        match conversion {
            PercentConversion::Repr => self.repr_of(value, span),
            PercentConversion::Ascii => {
                let written = self.repr_of(value, span);
                let escaped = self.call_runtime(RuntimeFunction::Ascii, vec![written.value], span);
                self.release(written, span);
                escaped
            }
            _ => self.string_of(value, span),
        }
    }

    /// Returns the flags, width and precision as the runtime reads them, such as `-08.3`; a
    /// `*` takes its digits from an item of the right operand.
    fn percent_spec(&mut self, held: Lowered, field: &PercentField, span: Span) -> Lowered {
        let mut text = field.flags.clone();
        let mut joined: Option<Lowered> = None;
        for (number, dot) in [(field.width, ""), (field.precision, ".")] {
            if number == PercentNumber::Absent {
                continue;
            }
            text.push_str(dot);
            match number {
                PercentNumber::Absent => {}
                PercentNumber::Given(digits) => text.push_str(&digits.to_string()),
                PercentNumber::Item(index) => {
                    let before = self.string(&std::mem::take(&mut text), span);
                    let before = Self::borrowed(before, TyirType::STR);
                    let item = self.borrowed_item(held, index, TyirType::Int, span);
                    let digits = self.string_of(item, span);
                    let piece = self.concatenated(before, digits, span);
                    joined = Some(match joined {
                        None => piece,
                        Some(start) => self.concatenated(start, piece, span),
                    });
                }
            }
        }
        let rest = self.string(&text, span);
        let rest = Self::borrowed(rest, TyirType::STR);
        match joined {
            None => rest,
            Some(start) => self.concatenated(start, rest, span),
        }
    }

    /// Returns `format % values` read by the runtime: a tuple's items are the values, given
    /// with its layout, and any other value is given as a tuple of one, which the runtime
    /// reads as one value or, for a `tuple[T, ...]`, as the items of the list that holds it
    /// (LIB-11).
    pub(crate) fn percent_dynamic(
        &mut self,
        format: &typy_check::TypedExpr,
        values: &typy_check::TypedExpr,
        span: Span,
    ) -> Lowered {
        let format = self.operand_before(format, &[values], span);
        let (layout, tuple, form) = match &values.ty {
            typy_check::Type::Tuple(items) => {
                let id = self.tuple_type_id(items);
                (id, self.expression(values), "0")
            }
            other => {
                let id = self.tuple_type_id(std::slice::from_ref(other));
                let form = if matches!(other, typy_check::Type::TupleOf(_)) {
                    "2"
                } else {
                    "1"
                };
                let tuple =
                    self.tuple_display(std::slice::from_ref(values), TyirType::Tuple(id), span);
                (id, tuple, form)
            }
        };
        let descriptor = self.constant(Constant::TupleDescriptor(layout), span);
        let address = self.bits(Self::borrowed(tuple.value, tuple.ty), span);
        let form = self.constant(Constant::Int(form.to_owned()), span);
        let text = self.call_runtime(
            RuntimeFunction::PercentDynamic,
            vec![format.value, descriptor, address, form],
            span,
        );
        self.release(tuple, span);
        self.release(format, span);
        text
    }
}
