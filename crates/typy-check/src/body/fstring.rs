//! Checks f-strings: each replacement field formats a value of any type, with its
//! conversion and its format specification, whose own fields are checked the same way
//! (LEX-07).

use typy_diagnostics::Span;
use typy_source::syntax;

use super::BodyChecker;
use crate::typed::{Conversion, FStringPart, FormattedValue, TypedExpr, TypedExprKind};
use crate::types::Type;

impl BodyChecker<'_> {
    /// Checks an f-string, which is a `str`.
    pub(super) fn fstring(&mut self, parts: &[syntax::FStringPart], span: Span) -> TypedExpr {
        let parts = self.fstring_parts(parts);
        Self::typed(TypedExprKind::FString(parts), Type::Str, span)
    }

    fn fstring_parts(&mut self, parts: &[syntax::FStringPart]) -> Vec<FStringPart> {
        let mut checked = Vec::with_capacity(parts.len());
        for part in parts {
            match part {
                syntax::FStringPart::Literal(text) => {
                    checked.push(FStringPart::Literal(text.clone()));
                }
                syntax::FStringPart::Field(field) => {
                    // `{x = }` writes its text, then `repr(x)` unless it has a conversion or
                    // a format specification.
                    if let Some(debug) = &field.debug {
                        checked.push(FStringPart::Literal(debug.clone()));
                    }
                    let value = self.expression(&field.value);
                    let conversion = match field.conversion {
                        Some('s') => Some(Conversion::Str),
                        Some('r') => Some(Conversion::Repr),
                        Some('a') => Some(Conversion::Ascii),
                        _ if field.debug.is_some() && field.spec.is_none() => {
                            Some(Conversion::Repr)
                        }
                        _ => None,
                    };
                    let spec = field.spec.as_ref().map(|spec| self.fstring_parts(spec));
                    checked.push(FStringPart::Field(Box::new(FormattedValue {
                        value,
                        conversion,
                        spec,
                    })));
                }
            }
        }
        checked
    }
}
