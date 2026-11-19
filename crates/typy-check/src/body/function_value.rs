//! Reads a function of the program as a value, and checks a call through a function value
//! (FUN-05, ADR-0060).

use typy_diagnostics::{Diagnostic, DiagnosticCode, RuleId, Span};
use typy_source::syntax::{Expr, Identifier, Keyword, ParameterKind};

use super::BodyChecker;
use crate::typed::{ClassId, FunctionId, TypedConstant, TypedExpr, TypedExprKind};
use crate::types::Type;

impl BodyChecker<'_> {
    /// Returns the function object of `function`, named `name`, of the `Callable` type of its
    /// parameters and its result. A generic function has a copy for each set of types and no
    /// one function object, and a `Callable` passes every argument by position, so a
    /// function with a keyword-only parameter has no `Callable` type; both are E0202.
    pub(super) fn function_value(
        &mut self,
        function: FunctionId,
        name: &str,
        span: Span,
    ) -> TypedExpr {
        if self.program.generic(function).is_some() {
            return self.error(
                Diagnostic::new(
                    DiagnosticCode::error(202),
                    span,
                    format!(
                        "the generic function `{name}` is a copy for each set of types, so it \
                         has no one value"
                    ),
                )
                .with_rule(RuleId::FUN_05)
                .with_help(format!(
                    "define a function of the types you need that calls `{name}`"
                )),
                span,
            );
        }
        let signature = &self.program.signatures[function.0 as usize];
        if let Some(parameter) = signature
            .parameters
            .iter()
            .find(|parameter| parameter.kind == ParameterKind::KeywordOnly)
        {
            let message = format!(
                "`{name}` takes `{}` by keyword only, and a `Callable` passes every argument \
                 by position, so `{name}` has no `Callable` type",
                parameter.name
            );
            return self.error(
                Diagnostic::new(DiagnosticCode::error(202), span, message)
                    .with_rule(RuleId::FUN_05),
                span,
            );
        }
        let ty = Type::Callable {
            parameters: signature
                .parameters
                .iter()
                .map(|parameter| parameter.ty.clone())
                .collect(),
            returns: Box::new(signature.returns.clone()),
        };
        Self::typed(TypedExprKind::FunctionValue(function), ty, span)
    }

    /// Returns the call of the value the attribute at `index` of `receiver`'s class holds,
    /// as in `self.on_done(result)`: a call through it when it is a function value (FUN-05),
    /// E0401 for any other value. The receiver was read as a value, so an instance whose
    /// `__init__` has not assigned every attribute yet was already refused (CLS-01).
    pub(super) fn attribute_call(
        &mut self,
        receiver: TypedExpr,
        (class, index): (ClassId, usize),
        name: &Identifier,
        arguments: &[Expr],
        keywords: &[Keyword],
        span: Span,
    ) -> TypedExpr {
        let Some(ty) = self.program.attribute_ty(class, index) else {
            for argument in arguments {
                self.expression(argument);
            }
            return self.type_not_known_yet(class, name, span);
        };
        if let Type::Callable { .. } = ty {
            let callee = Self::typed(
                TypedExprKind::Attribute {
                    object: Box::new(receiver),
                    index,
                },
                ty,
                name.span,
            );
            return self.call_value(callee, arguments, keywords, span);
        }
        for argument in arguments {
            self.expression(argument);
        }
        if ty.has_error() {
            return Self::typed(
                TypedExprKind::Constant(TypedConstant::None),
                Type::Error,
                span,
            );
        }
        let info = &self.program.classes[class.0 as usize];
        let message = format!(
            "`{}.{}` is {} and cannot be called",
            info.name,
            name.name,
            ty.with_article()
        );
        self.error(
            Diagnostic::new(DiagnosticCode::error(401), name.span, message)
                .with_rule(RuleId::EXP_02),
            span,
        )
    }

    /// Returns the call through `callee`, a function value: one argument by position for each
    /// parameter of its `Callable` type, each checked and converted as an argument of a call
    /// by name is. A `Callable` names no parameters, so a keyword argument is E0404.
    pub(super) fn call_value(
        &mut self,
        callee: TypedExpr,
        arguments: &[Expr],
        keywords: &[Keyword],
        span: Span,
    ) -> TypedExpr {
        // Each caller calls only a value of a `Callable` type.
        let (parameters, returns) = callee.ty.callable_parts().map_or_else(
            || (Vec::new(), Type::Error),
            |(parameters, returns)| (parameters.to_vec(), returns.clone()),
        );
        if let Some(keyword) = keywords.first() {
            for argument in arguments
                .iter()
                .chain(keywords.iter().map(|each| &each.value))
            {
                self.expression(argument);
            }
            let written = keyword
                .name
                .as_ref()
                .map_or_else(|| "**".to_owned(), |name| format!("{}=", name.name));
            return self.error(
                Diagnostic::new(
                    DiagnosticCode::error(404),
                    keyword.span,
                    format!(
                        "a value of type {} names no parameters, so `{written}` cannot be \
                         passed",
                        callee.ty
                    ),
                )
                .with_rule(RuleId::FUN_05)
                .with_help("pass the argument by position"),
                span,
            );
        }
        if arguments.len() != parameters.len() {
            for argument in arguments {
                self.expression(argument);
            }
            let message = format!(
                "a value of type {} takes {} argument{}, and {} {} given",
                callee.ty,
                parameters.len(),
                if parameters.len() == 1 { "" } else { "s" },
                arguments.len(),
                if arguments.len() == 1 { "is" } else { "are" },
            );
            return self.error(
                Diagnostic::new(DiagnosticCode::error(404), span, message)
                    .with_rule(RuleId::FUN_05),
                span,
            );
        }
        let arguments = arguments
            .iter()
            .zip(&parameters)
            .enumerate()
            .map(|(position, (argument, parameter))| {
                let value = self.expression_expecting(argument, Some(parameter));
                self.expect(
                    &value,
                    parameter.clone(),
                    &format!("argument {}", position + 1),
                );
                value
            })
            .collect();
        Self::typed(
            TypedExprKind::CallValue {
                callee: Box::new(callee),
                arguments,
            },
            returns,
            span,
        )
    }
}
