//! Checks calls of program functions and classes: arity, keywords, defaults and argument
//! types.

use typy_diagnostics::{Diagnostic, DiagnosticCode, RuleId, Span};
use typy_source::syntax::{Expr, Keyword};

use super::BodyChecker;
use crate::builtin::Builtin;
use crate::exception::BuiltinException;
use crate::typed::{Argument, FunctionId, TypedExpr, TypedExprKind};
use crate::types::Type;

impl BodyChecker<'_> {
    pub(super) fn call(
        &mut self,
        name: &str,
        name_span: Span,
        arguments: &[Expr],
        keywords: &[Keyword],
        span: Span,
    ) -> TypedExpr {
        if let Some(symbol) = self.locals.get(name) {
            let ty = self.types.get(symbol).cloned().unwrap_or(Type::Error);
            return self.error(
                Diagnostic::new(
                    DiagnosticCode::error(401),
                    name_span,
                    format!("`{name}` is {} and cannot be called", ty.with_article()),
                )
                .with_rule(RuleId::EXP_02),
                span,
            );
        }
        if let Some(function) = self.program.function_ids.get(name).copied() {
            let call = self.call_function(function, name, arguments, keywords, span);
            return self.through_lazy(name, call);
        }
        if let Some(class) = self.program.class_ids.get(name).copied() {
            let instance = self.new_instance(class, arguments, keywords, span);
            return self.through_lazy(name, instance);
        }
        if let Some(builtin) = Builtin::named(name) {
            return self.call_builtin(builtin, arguments, keywords, span);
        }
        if let Some(class) = BuiltinException::named(name) {
            return self.new_exception(class, arguments, keywords, span);
        }
        if self.in_function() && self.program.module_variables.contains(name) {
            return self.name(name, name_span);
        }
        let result = self.unknown_name(name, name_span);
        for argument in arguments {
            self.expression(argument);
        }
        TypedExpr { span, ..result }
    }

    pub(super) fn call_function(
        &mut self,
        function: FunctionId,
        name: &str,
        arguments: &[Expr],
        keywords: &[Keyword],
        span: Span,
    ) -> TypedExpr {
        match self.bind_arguments(function, 0, name, arguments, keywords, span) {
            Ok(bound) => Self::typed(
                TypedExprKind::Call {
                    function,
                    arguments: bound,
                },
                self.program.signatures[function.0 as usize].returns.clone(),
                span,
            ),
            Err(error) => error,
        }
    }

    /// Binds the arguments of a call of `function` to its parameters after the first `skip`,
    /// which a method's instance fills; returns the bound arguments, or the error expression
    /// after reporting what does not fit.
    pub(super) fn bind_arguments(
        &mut self,
        function: FunctionId,
        skip: usize,
        name: &str,
        arguments: &[Expr],
        keywords: &[Keyword],
        span: Span,
    ) -> Result<Vec<Argument>, TypedExpr> {
        let signature = self.program.signatures[function.0 as usize].clone();
        let parameters = &signature.parameters[skip.min(signature.parameters.len())..];
        let mut bound: Vec<Argument> = Vec::new();
        let mut filled = vec![false; parameters.len()];
        for (index, argument) in arguments.iter().enumerate() {
            let expected = parameters.get(index).map(|parameter| &parameter.ty);
            let value = self.expression_expecting(argument, expected);
            let Some(parameter) = parameters.get(index) else {
                return Err(self.error(
                    Diagnostic::new(
                        DiagnosticCode::error(404),
                        value.span,
                        format!(
                            "`{name}` takes {} arguments, and more are given",
                            parameters.len()
                        ),
                    )
                    .with_rule(RuleId::TYP_01),
                    span,
                ));
            };
            self.expect(
                &value,
                parameter.ty.clone(),
                &format!("argument `{}`", parameter.name),
            );
            filled[index] = true;
            bound.push(Argument {
                parameter: index + skip,
                value,
            });
        }
        for keyword in keywords {
            let position = keyword.name.as_ref().and_then(|keyword_name| {
                parameters
                    .iter()
                    .position(|parameter| parameter.name == keyword_name.name)
            });
            let expected = position.map(|index| &parameters[index].ty);
            let value = self.expression_expecting(&keyword.value, expected);
            let Some(keyword_name) = &keyword.name else {
                continue;
            };
            let Some(index) = position else {
                return Err(self.error(
                    Diagnostic::new(
                        DiagnosticCode::error(404),
                        keyword_name.span,
                        format!("`{name}` has no parameter named `{}`", keyword_name.name),
                    )
                    .with_rule(RuleId::TYP_01),
                    span,
                ));
            };
            if filled[index] {
                return Err(self.error(
                    Diagnostic::new(
                        DiagnosticCode::error(404),
                        keyword_name.span,
                        format!("argument `{}` is given more than once", keyword_name.name),
                    )
                    .with_rule(RuleId::TYP_01),
                    span,
                ));
            }
            self.expect(
                &value,
                parameters[index].ty.clone(),
                &format!("argument `{}`", keyword_name.name),
            );
            filled[index] = true;
            bound.push(Argument {
                parameter: index + skip,
                value,
            });
        }
        for (parameter, is_filled) in parameters.iter().zip(&filled) {
            if !is_filled && parameter.default.is_none() {
                return Err(self.error(
                    Diagnostic::new(
                        DiagnosticCode::error(404),
                        span,
                        format!("`{name}` is missing the argument `{}`", parameter.name),
                    )
                    .with_rule(RuleId::TYP_01),
                    span,
                ));
            }
        }
        Ok(bound)
    }
}
