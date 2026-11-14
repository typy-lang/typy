//! `OSError` and the classes derived from it built with two to five arguments, which CPython
//! reads as `errno`, `strerror`, `filename`, `winerror` and `filename2`, and their attributes
//! (EXC-14).

use typy_diagnostics::{Diagnostic, DiagnosticCode, RuleId, Span};
use typy_source::syntax::Identifier;

use super::BodyChecker;
use crate::exception::{BuiltinException, ExceptionClass};
use crate::typed::{OsErrorField, TypedExpr, TypedExprKind};
use crate::types::Type;

impl BodyChecker<'_> {
    /// Refuses the arguments of `OSError` or a class derived from it that TyPy does not give
    /// attributes of known types: with two to five, `errno` is an `int`, `strerror` a `str`,
    /// `filename` and `filename2` a `str` or `None`, and `winerror` an `int` or `None`. Two or
    /// more through `super().__init__` set no attributes in CPython, and are refused too.
    pub(super) fn os_error_arguments(
        &mut self,
        builtin: BuiltinException,
        values: &[TypedExpr],
        through_init: bool,
        span: Span,
    ) -> Option<TypedExpr> {
        if !builtin.is_os_error() || values.len() < 2 {
            return None;
        }
        let name = |ty: &Type| {
            matches!(ty, Type::Str | Type::NoneType)
                || matches!(ty, Type::Optional(inner) if **inner == Type::Str)
        };
        let fits = values.len() <= 5
            && !through_init
            && values.iter().enumerate().all(|(index, value)| match index {
                0 => matches!(value.ty, Type::Int | Type::Error),
                1 => matches!(value.ty, Type::Str | Type::Error),
                3 => matches!(value.ty, Type::Int | Type::NoneType | Type::Error),
                _ => name(&value.ty) || value.ty == Type::Error,
            });
        if fits {
            return None;
        }
        let what = if through_init {
            "an `OSError` given two or more arguments through `super().__init__`"
        } else {
            "an `OSError` of two or more arguments other than an `int` errno, a `str` \
             strerror and file names that are a `str` or `None`"
        };
        Some(
            self.error(
                Diagnostic::new(
                    DiagnosticCode::error(201),
                    values[1].span,
                    format!("{what} is planned for TyPy 0.5 (EXC-16)"),
                )
                .with_rule(RuleId::EXC_16),
                span,
            ),
        )
    }

    /// Returns `errno`, `strerror`, `filename` or `filename2` of an `OSError`, when `object`
    /// is one and the name is one of them, and its class declares no attribute of that name.
    pub(super) fn os_error_field(
        &self,
        object: &TypedExpr,
        attribute: &Identifier,
        span: Span,
    ) -> Option<TypedExpr> {
        let Type::Exception(class) = &object.ty else {
            return None;
        };
        if !class.builtin().is_os_error() {
            return None;
        }
        if let ExceptionClass::Class(own) = class
            && self.program.classes[own.id.0 as usize]
                .attribute(&attribute.name)
                .is_some()
        {
            return None;
        }
        let (field, ty) = match attribute.name.as_str() {
            "errno" => (OsErrorField::Errno, Type::Int),
            "strerror" => (OsErrorField::Strerror, Type::Str),
            "filename" => (OsErrorField::Filename, Type::Str),
            "filename2" => (OsErrorField::Filename2, Type::Str),
            _ => return None,
        };
        Some(Self::typed(
            TypedExprKind::OsErrorField {
                exception: Box::new(object.clone()),
                field,
            },
            Type::optional(ty),
            span,
        ))
    }

    /// Refuses two or more arguments to a class derived from `OSError` with an `__init__` of
    /// its own: `OSError.__new__` reads them as `errno` and the rest before `__init__` runs.
    pub(super) fn os_error_with_init(
        &mut self,
        exception: &ExceptionClass,
        name: &str,
        arguments: &[typy_source::syntax::Expr],
        span: Span,
    ) -> Option<TypedExpr> {
        if !exception.builtin().is_os_error() || arguments.len() < 2 {
            return None;
        }
        for argument in arguments {
            self.expression(argument);
        }
        Some(
            self.error(
                Diagnostic::new(
                    DiagnosticCode::error(201),
                    span,
                    format!(
                        "`{name}`, an `OSError` with an `__init__` of its own, given two or \
                         more arguments is planned for TyPy 0.5 (EXC-16)"
                    ),
                )
                .with_rule(RuleId::EXC_16),
                span,
            ),
        )
    }
}
