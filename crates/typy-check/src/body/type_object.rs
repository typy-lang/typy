//! Checks type objects as values: `type(x)`, a class name or a built-in type name read as a
//! value, and a type object's `__name__` (CLS-12).

use typy_diagnostics::{Diagnostic, DiagnosticCode, RuleId, Span};
use typy_source::syntax::{Expr, Keyword};

use super::BodyChecker;
use crate::typed::{TypedExpr, TypedExprKind};
use crate::types::{Type, TypeObjectOf};

impl BodyChecker<'_> {
    /// Checks `type(value)`, which gives the type object of the value. The form with three
    /// arguments makes a class while the program runs, which TyPy cannot compile.
    pub(super) fn call_type(
        &mut self,
        arguments: &[Expr],
        keywords: &[Keyword],
        span: Span,
    ) -> TypedExpr {
        let [argument] = arguments else {
            // The three arguments of a class made while the program runs are not checked:
            // the form itself is the error, so one cause gives one error.
            if arguments.len() != 3 {
                for argument in arguments {
                    self.expression(argument);
                }
            }
            let diagnostic = if arguments.len() == 3 {
                Diagnostic::new(
                    DiagnosticCode::error(202),
                    span,
                    "making a class with `type(name, bases, namespace)` cannot be compiled by TyPy with CPython's meaning",
                )
                .with_rule(RuleId::STD_04)
            } else {
                Diagnostic::new(
                    DiagnosticCode::error(404),
                    span,
                    format!("`type` takes 1 argument, and {} are given", arguments.len()),
                )
                .with_rule(RuleId::CLS_12)
            };
            return self.error(diagnostic, span);
        };
        if let Some(keyword) = keywords.first() {
            self.expression(argument);
            return self.error(
                Diagnostic::new(
                    DiagnosticCode::error(404),
                    keyword.span,
                    "`type` takes no keyword arguments",
                )
                .with_rule(RuleId::CLS_12),
                span,
            );
        }
        let value = self.expression(argument);
        // An instance of a class some class derives from may be of that class, so its type
        // object is a `type[C]` read when the program runs, whose calls and class members are
        // planned (CLS-39, ADR-0084).
        let ty = TypeObjectOf::type_of(TypeObjectOf::of(&value.ty));
        Self::typed(TypedExprKind::TypeOf(Box::new(value)), ty, span)
    }

    /// Returns the type object a class name or a built-in type name gives as a value.
    pub(super) fn type_object_named(&self, name: &str, span: Span) -> Option<TypedExpr> {
        let of = match self.program.class_ids.get(name) {
            Some(class) => match self.program.exception(*class) {
                Some(exception) => TypeObjectOf::Exception(exception.clone()),
                None => TypeObjectOf::Instance(*class, name.to_owned()),
            },
            None => TypeObjectOf::named(name)?,
        };
        let ty = TypeObjectOf::type_of(Some(of.clone()));
        Some(Self::typed(
            TypedExprKind::TypeObject(Box::new(of)),
            ty,
            span,
        ))
    }

    /// Checks `value.__name__` of a type object, the only attribute a type object has in
    /// TyPy.
    pub(super) fn type_object_attribute(
        &mut self,
        object: TypedExpr,
        attribute: &typy_source::syntax::Identifier,
        span: Span,
    ) -> TypedExpr {
        if attribute.name == "__name__" {
            return Self::typed(TypedExprKind::TypeName(Box::new(object)), Type::Str, span);
        }
        // A method of a built-in type is a value where a `Callable` is expected (FUN-08).
        if let Type::TypeObject(Some(class)) = &object.ty
            && let Some(receiver) = super::builtin_value::receiver_of(class)
            && crate::method::Method::of(&receiver, &attribute.name).is_some()
        {
            let qualified = format!("{}.{}", class.name(), attribute.name);
            return self.error(
                Diagnostic::new(
                    DiagnosticCode::error(203),
                    attribute.span,
                    format!(
                        "`{qualified}` as a value needs the `Callable` type its context expects"
                    ),
                )
                .with_rule(RuleId::FUN_08)
                .with_help(format!(
                    "annotate the variable with the types you call it with, such as \
                     `change: Callable[[{0}], {0}] = {qualified}`",
                    class.name()
                )),
                span,
            );
        }
        let message = format!(
            "{} has no attribute `{}` in TyPy; it has `__name__`",
            object.ty.with_article(),
            attribute.name
        );
        let mut diagnostic = Diagnostic::new(DiagnosticCode::error(412), attribute.span, message)
            .with_rule(RuleId::CLS_12);
        if object.ty == Type::TypeObject(None) {
            diagnostic = diagnostic.with_help(
                "reading a class variable or a class method through a class known only when \
                 the program runs is planned for TyPy 0.14 (CLS-39)",
            );
        }
        self.error(diagnostic, span)
    }
}
