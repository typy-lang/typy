//! Checks `format % values` with a literal format (LIB-10): the format is read when the
//! program is checked, and each conversion must fit the type of its value, since CPython
//! would raise `TypeError` or `ValueError` for any program that does not fit. Messages are
//! CPython 3.15's, the newest wording (OI-14).

use typy_diagnostics::{Diagnostic, DiagnosticCode, RuleId, Span};
use typy_source::syntax::Expr;

use super::BodyChecker;
use crate::percent_format::{self, Number, Piece};
use crate::typed::{
    PercentConversion, PercentField, PercentNumber, PercentPart, PercentValue, TypedExpr,
    TypedExprKind,
};
use crate::types::Type;

/// Returns E0427 at `span` with `message`.
fn mismatch(span: Span, message: impl Into<String>) -> Box<Diagnostic> {
    Box::new(Diagnostic::new(DiagnosticCode::error(427), span, message).with_rule(RuleId::LIB_10))
}

/// The values a format takes in order, each with where it comes from.
struct Arguments {
    values: Vec<(PercentValue, Type)>,
    next: usize,
}

impl Arguments {
    /// Returns the next value, or CPython's message when there is none.
    fn next(&mut self) -> Result<(PercentValue, Type), String> {
        let value = self.values.get(self.next).cloned().ok_or_else(|| {
            format!(
                "not enough arguments for format string (got {})",
                self.values.len()
            )
        })?;
        self.next += 1;
        Ok(value)
    }

    /// Returns a `*` width or precision, which must be an `int`.
    fn star(&mut self) -> Result<PercentNumber, String> {
        let position = self.next + 1;
        match self.next()? {
            (PercentValue::Item(index), Type::Int | Type::Bool) => Ok(PercentNumber::Item(index)),
            (_, ty) => Err(format!(
                "format argument {position}: * requires int, not {}",
                type_name(&ty)
            )),
        }
    }
}

impl BodyChecker<'_> {
    /// Checks `format % values`, a `str`, where `format` is the text of a literal.
    pub(super) fn percent_format(
        &mut self,
        format: &str,
        format_span: Span,
        right: &Expr,
        span: Span,
    ) -> TypedExpr {
        let values = self.expression(right);
        let pieces = match percent_format::parse(format) {
            Ok(pieces) => pieces,
            Err(message) => return self.error(*mismatch(format_span, message), span),
        };
        if values.ty.has_error() {
            return TypedExpr {
                ty: Type::Error,
                ..values
            };
        }
        match percent_parts(&pieces, &values.ty, values.span) {
            Ok(parts) => Self::typed(
                TypedExprKind::PercentFormat {
                    values: Box::new(values),
                    parts,
                },
                Type::Str,
                span,
            ),
            Err(diagnostic) => self.error(*diagnostic, span),
        }
    }
}

/// Returns the pieces of the text, each conversion with the value it takes; `span` is the
/// right operand's, where a mismatch is reported.
fn percent_parts(
    pieces: &[Piece],
    values: &Type,
    span: Span,
) -> Result<Vec<PercentPart>, Box<Diagnostic>> {
    let mut arguments = Arguments {
        values: percent_arguments(values, span)?,
        next: 0,
    };
    let mapping = matches!(values, Type::Dict(..));
    let mut keyed = false;
    let mut parts = Vec::with_capacity(pieces.len());
    for piece in pieces {
        let conversion = match piece {
            Piece::Literal(text) => {
                parts.push(PercentPart::Literal(text.clone()));
                continue;
            }
            Piece::Conversion(conversion) => conversion,
        };
        let field = if let Some(key) = &conversion.key {
            keyed = true;
            keyed_field(conversion, key, values)
        } else if keyed {
            // A key on a value that is not a dict has already failed, so this is a dict.
            Err(format!(
                "format requires a parenthesised mapping key at position {}",
                conversion.position
            ))
        } else {
            unnamed_field(conversion, &mut arguments)
        };
        parts.push(PercentPart::Field(
            field.map_err(|message| mismatch(span, message))?,
        ));
    }
    // CPython leaves a mapping that the conversions do not use alone.
    if !mapping && arguments.next < arguments.values.len() {
        return Err(mismatch(
            span,
            format!(
                "not all arguments converted during string formatting (required {}, got {})",
                arguments.next,
                arguments.values.len()
            ),
        ));
    }
    Ok(parts)
}

/// Returns the values a format takes in order: the items of a tuple, or the right operand
/// itself.
fn percent_arguments(
    values: &Type,
    span: Span,
) -> Result<Vec<(PercentValue, Type)>, Box<Diagnostic>> {
    Ok(match values {
        Type::Tuple(items) => items
            .iter()
            .enumerate()
            .map(|(index, item)| (PercentValue::Item(index), item.clone()))
            .collect(),
        Type::TupleOf(_) => {
            return Err(Box::new(
                Diagnostic::new(
                    DiagnosticCode::error(201),
                    span,
                    "formatting with `%` a tuple whose length is known only when the program \
                     runs is planned for TyPy 0.4 (LIB-11)",
                )
                .with_rule(RuleId::LIB_11)
                .with_help("give the values as a tuple display, such as `(a, b)`"),
            ));
        }
        other => vec![(PercentValue::Whole, other.clone())],
    })
}

/// Returns a conversion with a key, `%(key)s`, whose value comes from a dict.
fn keyed_field(
    conversion: &percent_format::Conversion,
    key: &str,
    values: &Type,
) -> Result<PercentField, String> {
    let Type::Dict(_, value) = values else {
        return Err(format!(
            "format requires a mapping, not {}",
            type_name(values)
        ));
    };
    let given = |number: Number| match number {
        Number::Given(digits) => Some(PercentNumber::Given(digits)),
        Number::Absent => Some(PercentNumber::Absent),
        Number::Star => None,
    };
    let (Some(width), Some(precision)) = (given(conversion.width), given(conversion.precision))
    else {
        return Err(format!(
            "* cannot be used with a parenthesised mapping key at position {}",
            conversion.position
        ));
    };
    let label = format!(" {}", quoted(key));
    let kind = percent_conversion(conversion.kind, value, &label)?;
    Ok(PercentField {
        value: PercentValue::Key(key.to_owned()),
        flags: conversion.flags.clone(),
        width,
        precision,
        conversion: kind,
        ty: (**value).clone(),
        label: format!("format argument{label}"),
    })
}

/// Returns a conversion without a key, which takes the next values in order.
fn unnamed_field(
    conversion: &percent_format::Conversion,
    arguments: &mut Arguments,
) -> Result<PercentField, String> {
    let mut number = |given: Number| match given {
        Number::Absent => Ok(PercentNumber::Absent),
        Number::Given(digits) => Ok(PercentNumber::Given(digits)),
        Number::Star => arguments.star(),
    };
    let width = number(conversion.width)?;
    let precision = number(conversion.precision)?;
    let (value, ty) = arguments.next()?;
    let label = match value {
        PercentValue::Item(index) => format!(" {}", index + 1),
        PercentValue::Whole | PercentValue::Key(_) => String::new(),
    };
    let kind = percent_conversion(conversion.kind, &ty, &label)?;
    Ok(PercentField {
        value,
        flags: conversion.flags.clone(),
        width,
        precision,
        conversion: kind,
        ty,
        label: format!("format argument{label}"),
    })
}

/// Returns how the conversion character `kind` turns a value of type `ty` into text, or
/// CPython's message, which names the value by `label`, when it cannot.
fn percent_conversion(kind: char, ty: &Type, label: &str) -> Result<PercentConversion, String> {
    let integer = matches!(ty, Type::Int | Type::Bool);
    let real = integer || *ty == Type::Float;
    let byte = u8::try_from(kind).unwrap_or(b's');
    let needs = match kind {
        's' => return Ok(PercentConversion::Str),
        'r' => return Ok(PercentConversion::Repr),
        'a' => return Ok(PercentConversion::Ascii),
        'd' | 'i' | 'u' if real => return Ok(PercentConversion::Integer(b'd')),
        'x' | 'X' | 'o' if integer => return Ok(PercentConversion::Integer(byte)),
        'c' if integer || *ty == Type::Str => return Ok(PercentConversion::Char),
        'x' | 'X' | 'o' => "an integer",
        'c' => "an integer or a unicode character",
        _ if real => return Ok(PercentConversion::Float(byte)),
        _ => "a real number",
    };
    Err(format!(
        "format argument{label}: %{kind} requires {needs}, not {}",
        type_name(ty)
    ))
}

/// Returns the name CPython's message gives the type of a value: its class's name, and
/// `NoneType` for an optional value, which fails when it is `None`.
fn type_name(ty: &Type) -> String {
    match ty {
        Type::List(_) => "list".to_owned(),
        Type::Tuple(_) | Type::TupleOf(_) => "tuple".to_owned(),
        Type::Set(_) => "set".to_owned(),
        Type::Dict(..) => "dict".to_owned(),
        Type::Optional(_) | Type::NoneType => "NoneType".to_owned(),
        Type::Instance(_, name) => name.clone(),
        other => other.to_string(),
    }
}

/// Returns `text` as `repr` writes a `str`: in single quotes unless it holds one and no
/// double quote, with backslashes and the quote escaped.
fn quoted(text: &str) -> String {
    let quote = if text.contains('\'') && !text.contains('"') {
        '"'
    } else {
        '\''
    };
    let mut written = String::from(quote);
    for character in text.chars() {
        if character == '\\' || character == quote {
            written.push('\\');
        }
        written.push(character);
    }
    written.push(quote);
    written
}
