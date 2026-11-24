//! Checks calls of the builtin `hash` (LIB-34).

use typy_diagnostics::{Diagnostic, DiagnosticCode, RuleId, Span};
use typy_source::syntax::{Expr, Keyword};

use super::BodyChecker;
use crate::exception::{BuiltinException, ExceptionClass};
use crate::typed::{TypedConstant, TypedExpr, TypedExprKind};
use crate::types::Type;

impl BodyChecker<'_> {
    /// Returns `hash(value)`: the hash a dict keeps the value as a key by, which CPython's
    /// equals for an `int`, a `float`, a `bool`, `None` and a tuple of them (LIB-34). A value
    /// of a type that has no hash is refused with E0413, except in a copy of a generic
    /// function, whose types a call chose: there it raises CPython's `TypeError` where it
    /// runs, as a `frozendict` of lists does only once it is hashed.
    pub(super) fn hash_call(
        &mut self,
        arguments: &[Expr],
        keywords: &[Keyword],
        span: Span,
    ) -> TypedExpr {
        let ([value], []) = (arguments, keywords) else {
            for argument in arguments
                .iter()
                .chain(keywords.iter().map(|keyword| &keyword.value))
            {
                self.expression(argument);
            }
            let message = if keywords.is_empty() {
                format!(
                    "hash() takes exactly one argument ({} given)",
                    arguments.len()
                )
            } else {
                "hash() takes no keyword arguments".to_owned()
            };
            let diagnostic = Diagnostic::new(
                DiagnosticCode::error(404),
                span,
                format!("{message}: CPython raises `TypeError`"),
            )
            .with_rule(RuleId::LIB_34);
            return self.error(diagnostic, span);
        };
        let value = self.expression(value);
        if value.ty.has_error() || value.ty.is_hashable() {
            return Self::typed(TypedExprKind::Hash(Box::new(value)), Type::Int, span);
        }
        if self.program.generics.depth > 0 {
            return Self::unhashable_raised(value, span);
        }
        let diagnostic = Diagnostic::new(
            DiagnosticCode::error(413),
            span,
            format!(
                "`hash` of {}, which has no hash: CPython raises `TypeError`",
                value.ty.with_article()
            ),
        )
        .with_rule(RuleId::LIB_34)
        .with_help("a list, a set or a dict changes, so it has no hash; use a tuple of its items");
        self.error(diagnostic, span)
    }

    /// Returns `value`, then `TypeError: unhashable type: 'NAME'` raised, typed as the `int`
    /// the hash would be.
    fn unhashable_raised(value: TypedExpr, span: Span) -> TypedExpr {
        let message = format!("unhashable type: '{}'", unhashable_name(&value.ty));
        let class = ExceptionClass::Builtin(BuiltinException::TypeError);
        let exception = Self::typed(
            TypedExprKind::NewException {
                class: class.clone(),
                arguments: vec![Self::typed(
                    TypedExprKind::Constant(TypedConstant::Str(message)),
                    Type::Str,
                    span,
                )],
                init: None,
            },
            Type::Exception(class),
            span,
        );
        let raised = Self::typed(TypedExprKind::Raise(Box::new(exception)), Type::Int, span);
        Self::typed(
            TypedExprKind::Sequence {
                first: Box::new(value),
                then: Box::new(raised),
            },
            Type::Int,
            span,
        )
    }
}

/// Returns the name CPython's `TypeError` gives the class that has no hash in a value of
/// `ty`: the list, set, dict, view or bytearray itself, or the one a tuple or an optional
/// holds.
fn unhashable_name(ty: &Type) -> String {
    match ty {
        Type::Tuple(items) => items
            .iter()
            .find(|item| !item.is_hashable())
            .map_or_else(|| ty.class_name(), unhashable_name),
        Type::TupleOf(item) | Type::Optional(item) => unhashable_name(item),
        Type::Bytearray => "bytearray".to_owned(),
        Type::DictView(view, ..) => view.name().to_owned(),
        other => other.class_name(),
    }
}

#[cfg(test)]
mod tests;
