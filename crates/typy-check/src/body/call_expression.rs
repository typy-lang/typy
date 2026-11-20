//! Checks a call expression: it goes to the program's function, method or class it names,
//! to a builtin, or through a function value. A call that unpacks with `*` or `**` binds only
//! to the program's functions, methods and classes and to `print`; any other is planned for
//! 0.10 (EXP-17, EXP-19, ADR-0073).

use typy_diagnostics::{Diagnostic, DiagnosticCode, RuleId, Span};
use typy_source::syntax::{Expr, Keyword};

use super::BodyChecker;
use crate::typed::TypedExpr;
use crate::types::Type;

impl BodyChecker<'_> {
    /// Checks `function(arguments, keywords)`.
    pub(super) fn checked_call(
        &mut self,
        function: &Expr,
        arguments: &[Expr],
        keywords: &[Keyword],
        span: Span,
    ) -> TypedExpr {
        let Some(at) = unpacking(arguments, keywords) else {
            return self.call_expression(function, arguments, keywords, span);
        };
        // A callee that takes the unpacked arguments binds them and notes the call; any other
        // saw each `*value` as an error it does not report, so its diagnostics give way.
        let reported = self.diagnostics.len();
        let call = self.call_expression(function, arguments, keywords, span);
        if self.unpacking_calls.contains(&span) {
            return call;
        }
        self.diagnostics.truncate(reported);
        self.error(
            Diagnostic::new(
                DiagnosticCode::error(201),
                at,
                "`*` and `**` in a call of a builtin other than `print`, of a function value, \
                 of a protocol method or of an exception class are planned for TyPy 0.10 \
                 (EXP-19)",
            )
            .with_rule(RuleId::EXP_19)
            .with_help("unpack the values into variables first, and pass them one by one"),
            span,
        )
    }

    fn call_expression(
        &mut self,
        function: &Expr,
        arguments: &[Expr],
        keywords: &[Keyword],
        span: Span,
    ) -> TypedExpr {
        match function {
            // A module's declaration of a library holds nothing to evaluate (FFI-01).
            _ if let Some(library) = self.foreign_library(span) => library,
            Expr::Name {
                id,
                span: name_span,
            } => self.call(id, *name_span, arguments, keywords, span),
            Expr::Attribute {
                value, attribute, ..
            } => self.method_call(value, attribute, arguments, keywords, span),
            // A generic class given its type arguments, such as `Box[int](1)`.
            Expr::Subscript { value, .. }
                if matches!(value.as_ref(), Expr::Name { id, .. }
                if self.program.class_ids.contains_key(id)) =>
            {
                self.applied_class_call(function, arguments, keywords, span)
            }
            // A class `POINTER(T)` or `T * length` called where it is made (FFI-03), or a
            // function value (FUN-05).
            _ => {
                let class = self.expression_or_class(function);
                match class.ty {
                    Type::CDataClass(class) => self.cdata_new(&class, arguments, keywords, span),
                    Type::Error => {
                        for argument in arguments {
                            self.expression(argument);
                        }
                        TypedExpr { span, ..class }
                    }
                    Type::Callable { .. } => self.call_value(class, arguments, keywords, span),
                    _ => {
                        for argument in arguments {
                            self.expression(argument);
                        }
                        self.not_callable((None, function.span()), &class.ty, span)
                    }
                }
            }
        }
    }
}

/// Returns where the first `*value` or `**value` of a call is, if it has one.
pub(super) fn unpacking(arguments: &[Expr], keywords: &[Keyword]) -> Option<Span> {
    arguments
        .iter()
        .find_map(|argument| match argument {
            Expr::Starred { span, .. } => Some(*span),
            _ => None,
        })
        .or_else(|| {
            keywords
                .iter()
                .find(|keyword| keyword.name.is_none())
                .map(|keyword| keyword.span)
        })
}
