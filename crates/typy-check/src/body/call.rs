//! Checks calls of program functions: arity, keywords, defaults and argument types.

use typy_diagnostics::{Diagnostic, DiagnosticCode, RuleId, Span};
use typy_source::syntax::{Expr, Keyword};

use super::BodyChecker;
use crate::builtin::Builtin;
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
            let ty = self.types.get(symbol).copied().unwrap_or(Type::Error);
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
            return self.call_function(function, name, arguments, keywords, span);
        }
        if let Some(builtin) = Builtin::named(name) {
            return self.call_builtin(builtin, arguments, keywords, span);
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
        let signature = self.program.signatures[function.0 as usize].clone();
        let mut bound: Vec<Argument> = Vec::new();
        let mut filled = vec![false; signature.parameters.len()];
        for (index, argument) in arguments.iter().enumerate() {
            let value = self.expression(argument);
            let Some(parameter) = signature.parameters.get(index) else {
                return self.error(
                    Diagnostic::new(
                        DiagnosticCode::error(404),
                        value.span,
                        format!(
                            "`{name}` takes {} arguments, and more are given",
                            signature.parameters.len()
                        ),
                    )
                    .with_rule(RuleId::TYP_01),
                    span,
                );
            };
            self.expect(
                &value,
                parameter.ty,
                &format!("argument `{}`", parameter.name),
            );
            filled[index] = true;
            bound.push(Argument {
                parameter: index,
                value,
            });
        }
        for keyword in keywords {
            let value = self.expression(&keyword.value);
            let Some(keyword_name) = &keyword.name else {
                continue;
            };
            let Some(index) = signature
                .parameters
                .iter()
                .position(|parameter| parameter.name == keyword_name.name)
            else {
                return self.error(
                    Diagnostic::new(
                        DiagnosticCode::error(404),
                        keyword_name.span,
                        format!("`{name}` has no parameter named `{}`", keyword_name.name),
                    )
                    .with_rule(RuleId::TYP_01),
                    span,
                );
            };
            if filled[index] {
                return self.error(
                    Diagnostic::new(
                        DiagnosticCode::error(404),
                        keyword_name.span,
                        format!("argument `{}` is given more than once", keyword_name.name),
                    )
                    .with_rule(RuleId::TYP_01),
                    span,
                );
            }
            self.expect(
                &value,
                signature.parameters[index].ty,
                &format!("argument `{}`", keyword_name.name),
            );
            filled[index] = true;
            bound.push(Argument {
                parameter: index,
                value,
            });
        }
        for (parameter, is_filled) in signature.parameters.iter().zip(&filled) {
            if !is_filled && parameter.default.is_none() {
                return self.error(
                    Diagnostic::new(
                        DiagnosticCode::error(404),
                        span,
                        format!("`{name}` is missing the argument `{}`", parameter.name),
                    )
                    .with_rule(RuleId::TYP_01),
                    span,
                );
            }
        }
        Self::typed(
            TypedExprKind::Call {
                function,
                arguments: bound,
            },
            signature.returns,
            span,
        )
    }
}
