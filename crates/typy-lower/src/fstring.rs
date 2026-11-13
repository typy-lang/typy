//! Lowers f-strings (LEX-07): each replacement field converts its value, formats it by its
//! type with its specification, and the pieces are joined in order.

use typy_check::{Conversion, FStringPart, FormattedValue, Type};
use typy_diagnostics::Span;
use typy_tyir::{Constant, RuntimeFunction, TyirType};

use crate::function::FunctionLowerer;
use crate::ownership::Lowered;

impl FunctionLowerer<'_> {
    /// Returns the string an f-string makes.
    pub(crate) fn fstring(&mut self, parts: &[FStringPart], span: Span) -> Lowered {
        let mut joined: Option<Lowered> = None;
        for part in parts {
            let piece = match part {
                FStringPart::Literal(text) => {
                    let value = self.string(text, span);
                    Self::borrowed(value, TyirType::STR)
                }
                FStringPart::Field(field) => self.formatted(field, span),
            };
            joined = Some(match joined {
                None => piece,
                Some(before) => {
                    let both = self.call_runtime(
                        RuntimeFunction::StrConcat,
                        vec![before.value, piece.value],
                        span,
                    );
                    self.release(piece, span);
                    self.release(before, span);
                    both
                }
            });
        }
        joined.unwrap_or_else(|| {
            let empty = self.string("", span);
            Self::borrowed(empty, TyirType::STR)
        })
    }

    /// Returns a text that holds its own reference, so the value it was made from may be
    /// released.
    fn owned_text(&mut self, text: Lowered, span: Span) -> Lowered {
        if text.owned {
            return text;
        }
        let value = self.take_reference(text, span);
        self.produced(value, text.ty)
    }

    /// Returns one replacement field: `format(conversion(value), spec)`.
    fn formatted(&mut self, field: &FormattedValue, span: Span) -> Lowered {
        let value = self.expression(&field.value);
        let text = match (field.conversion, &field.spec) {
            (Some(conversion), spec) => {
                let converted = match conversion {
                    Conversion::Str => self.string_of(value, span),
                    Conversion::Repr => self.repr_of(value, span),
                    Conversion::Ascii => {
                        let written = self.repr_of(value, span);
                        let escaped =
                            self.call_runtime(RuntimeFunction::Ascii, vec![written.value], span);
                        self.release(written, span);
                        escaped
                    }
                };
                let converted = self.owned_text(converted, span);
                match spec {
                    Some(spec) => {
                        let spec = self.fstring(spec, span);
                        let formatted = self.call_runtime(
                            RuntimeFunction::FormatStr,
                            vec![converted.value, spec.value],
                            span,
                        );
                        self.release(spec, span);
                        self.release(converted, span);
                        formatted
                    }
                    None => converted,
                }
            }
            (None, None) => {
                let written = self.string_of(value, span);
                self.owned_text(written, span)
            }
            (None, Some(spec)) => {
                let spec = self.fstring(spec, span);
                let formatted = self.format_value(value, &field.value.ty, spec, span);
                self.release(spec, span);
                formatted
            }
        };
        self.release(value, span);
        text
    }

    /// Returns `format(value, spec)` of a value of the checked type `ty`, by the formatter of
    /// its type; neither is released.
    fn format_value(&mut self, value: Lowered, ty: &Type, spec: Lowered, span: Span) -> Lowered {
        match value.ty {
            TyirType::Int => self.call_runtime(
                RuntimeFunction::FormatInt,
                vec![value.value, spec.value],
                span,
            ),
            TyirType::Float => self.call_runtime(
                RuntimeFunction::FormatFloat,
                vec![value.value, spec.value],
                span,
            ),
            TyirType::Bool => {
                let one = self.constant(Constant::Int("1".to_owned()), span);
                let zero = self.constant(Constant::Int("0".to_owned()), span);
                let word = self.select(value.value, one, zero, TyirType::Int);
                self.call_runtime(RuntimeFunction::FormatBool, vec![word, spec.value], span)
            }
            TyirType::STR => self.call_runtime(
                RuntimeFunction::FormatStr,
                vec![value.value, spec.value],
                span,
            ),
            TyirType::Optional(_) => {
                let Type::Optional(inner) = ty else {
                    unreachable!("an optional value has an optional type");
                };
                self.optional_branch(
                    value,
                    TyirType::STR,
                    span,
                    |lowerer| {
                        let none = lowerer.string("None", span);
                        let none = Self::borrowed(none, TyirType::STR);
                        let name = lowerer.string("NoneType", span);
                        lowerer.format_other(none, name, spec, span)
                    },
                    |lowerer, held| lowerer.format_value(held, inner, spec, span),
                )
            }
            _ => {
                let text = self.string_of(value, span);
                // The error names the value's own class, as `type(value).__name__` does, and a
                // file's type by its full name, as CPython names a type defined in C.
                let name = if *ty == Type::TextFile {
                    Self::borrowed(self.string("_io.TextIOWrapper", span), TyirType::STR)
                } else {
                    let class = self.runtime_type(value, ty, span);
                    self.call_runtime(RuntimeFunction::TypeName, vec![class.value], span)
                };
                let formatted = self.format_other(text, name.value, spec, span);
                for owned in [name, text] {
                    self.release(owned, span);
                }
                formatted
            }
        }
    }

    /// Returns `object.__format__(spec)` of a value already written as `text`.
    fn format_other(
        &mut self,
        text: Lowered,
        type_name: typy_tyir::ValueId,
        spec: Lowered,
        span: Span,
    ) -> Lowered {
        self.call_runtime(
            RuntimeFunction::FormatOther,
            vec![text.value, spec.value, type_name],
            span,
        )
    }
}
