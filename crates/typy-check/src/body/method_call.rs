//! Checks calls of the methods of built-in types: binds each argument to its parameter, by
//! position or by name, and checks its type (LIB-03).

use typy_diagnostics::{Diagnostic, DiagnosticCode, RuleId, Span};
use typy_source::syntax::{Constant, Expr, Identifier, Keyword};

use super::BodyChecker;
use crate::method::{Method, MethodSignature, Parameter, ParameterKind, ParameterType};
use crate::typed::{Argument, TypedConstant, TypedExpr, TypedExprKind};
use crate::types::Type;

impl BodyChecker<'_> {
    /// Checks `receiver.name(arguments)`, a method of a built-in type (LIB-03).
    pub(super) fn method_call(
        &mut self,
        receiver: &Expr,
        name: &Identifier,
        arguments: &[Expr],
        keywords: &[Keyword],
        span: Span,
    ) -> TypedExpr {
        let receiver = self.expression(receiver);
        if receiver.ty.has_error() {
            return Self::invalid_call(span);
        }
        let Some(method) = Method::of(&receiver.ty, &name.name) else {
            return self.error(
                Diagnostic::new(
                    DiagnosticCode::error(412),
                    name.span,
                    format!(
                        "{} has no method `{}` in TyPy",
                        receiver.ty.with_article(),
                        name.name
                    ),
                )
                .with_rule(RuleId::LIB_03),
                span,
            );
        };
        if matches!(method, Method::DictGet | Method::DictSetdefault)
            && arguments.len() == 1
            && keywords.is_empty()
        {
            for argument in arguments {
                self.expression(argument);
            }
            return self.error(
                Diagnostic::new(
                    DiagnosticCode::error(201),
                    span,
                    format!(
                        "`{}` without a default gives `V | None`, which is planned for TyPy 0.2 (TYP-06)",
                        method.name()
                    ),
                )
                .with_rule(RuleId::TYP_06)
                .with_help("give a default of the value type, such as `counts.get(word, 0)`"),
                span,
            );
        }
        let signature = method.signature(&receiver.ty);
        let Some(bound) = self.bind(method, &signature, arguments, keywords, span) else {
            return Self::invalid_call(span);
        };
        let text = if receiver.ty == Type::Bytes {
            Type::Bytes
        } else {
            Type::Str
        };
        let arguments = bound
            .into_iter()
            .map(|(parameter, expression)| Argument {
                parameter,
                value: self.method_argument(
                    method,
                    &signature.parameters[parameter],
                    expression,
                    &text,
                ),
            })
            .collect();
        Self::typed(
            TypedExprKind::Method {
                receiver: Box::new(receiver),
                method,
                arguments,
            },
            signature.result,
            span,
        )
    }

    fn invalid_call(span: Span) -> TypedExpr {
        Self::typed(
            TypedExprKind::Constant(TypedConstant::None),
            Type::Error,
            span,
        )
    }

    /// Binds the arguments of a call to the parameters, in source order: the positional ones in
    /// turn, then each keyword by its name. Returns `None` after reporting a call that does not
    /// fit the signature.
    fn bind<'call>(
        &mut self,
        method: Method,
        signature: &MethodSignature,
        arguments: &'call [Expr],
        keywords: &'call [Keyword],
        span: Span,
    ) -> Option<Vec<(usize, &'call Expr)>> {
        let parameters = &signature.parameters;
        let rest = parameters
            .iter()
            .position(|parameter| parameter.kind == ParameterKind::Rest);
        let positional = parameters
            .iter()
            .filter(|parameter| parameter.kind != ParameterKind::Keyword)
            .count();
        let required = parameters
            .iter()
            .filter(|parameter| parameter.required)
            .count();
        if rest.is_none() && arguments.len() > positional {
            self.arity_error(method, positional, required, arguments.len(), span);
            return None;
        }
        // Every argument from the rest parameter on binds to it.
        let mut bound: Vec<(usize, &Expr)> = arguments
            .iter()
            .enumerate()
            .map(|(index, argument)| (rest.map_or(index, |rest| index.min(rest)), argument))
            .collect();
        for keyword in keywords {
            let name = keyword.name.as_ref().map(|name| name.name.as_str());
            let index = parameters.iter().position(|parameter| {
                Some(parameter.name) == name
                    && matches!(
                        parameter.kind,
                        ParameterKind::Either | ParameterKind::Keyword
                    )
            });
            let message = match (name, index) {
                (Some(_), Some(index)) if bound.iter().all(|(given, _)| *given != index) => {
                    bound.push((index, &keyword.value));
                    continue;
                }
                (Some(name), Some(_)) => format!("`{name}` is given twice"),
                (Some("key"), None) if method == Method::ListSort => {
                    self.error(
                        Diagnostic::new(
                            DiagnosticCode::error(201),
                            keyword.span,
                            "the `key` argument of `sort` is planned for TyPy 1.0, with function values (FUN-05)",
                        )
                        .with_rule(RuleId::FUN_05),
                        span,
                    );
                    return None;
                }
                (Some(name), None) if parameters.iter().any(|parameter| parameter.name == name) => {
                    format!("`{name}` of `{}` is given by position only", method.name())
                }
                (Some(name), None) => {
                    format!("`{}` has no parameter named `{name}`", method.name())
                }
                (None, _) => format!("`{}` takes no `**` arguments", method.name()),
            };
            self.error(
                Diagnostic::new(DiagnosticCode::error(404), keyword.span, message)
                    .with_rule(RuleId::LIB_03),
                span,
            );
            return None;
        }
        let missing = parameters.iter().enumerate().any(|(index, parameter)| {
            parameter.required && bound.iter().all(|(given, _)| *given != index)
        });
        if missing {
            self.arity_error(method, positional, required, bound.len(), span);
            return None;
        }
        Some(bound)
    }

    fn arity_error(
        &mut self,
        method: Method,
        positional: usize,
        required: usize,
        given: usize,
        span: Span,
    ) {
        let expected = if positional == 0 && required == 0 && method == Method::ListSort {
            "no positional arguments".to_owned()
        } else if required == positional {
            counted(positional, "argument")
        } else if required == 0 {
            format!("at most {}", counted(positional, "argument"))
        } else {
            format!("{required} to {positional} arguments")
        };
        let verb = if given == 1 { "is" } else { "are" };
        self.diagnostics.push(
            Diagnostic::new(
                DiagnosticCode::error(404),
                span,
                format!(
                    "`{}` takes {expected}, and {given} {verb} given",
                    method.name()
                ),
            )
            .with_rule(RuleId::LIB_03),
        );
    }

    /// Checks one argument against what its parameter accepts.
    fn method_argument(
        &mut self,
        method: Method,
        parameter: &Parameter,
        argument: &Expr,
        text: &Type,
    ) -> TypedExpr {
        let what = format!("`{}` of `{}`", parameter.name, method.name());
        match &parameter.ty {
            ParameterType::Exact(ty) => {
                let value = self.expression_expecting(argument, Some(ty));
                self.expect(&value, ty.clone(), &what);
                value
            }
            ParameterType::StrOrNone => {
                let value = self.expression(argument);
                if !matches!(
                    argument,
                    Expr::Constant {
                        value: Constant::None,
                        ..
                    }
                ) {
                    self.expect(&value, text.clone(), &what);
                }
                value
            }
            ParameterType::Affixes => {
                let wanted = Type::TupleOf(Box::new(text.clone()));
                let value = self.expression_expecting(argument, Some(&wanted));
                if value.ty != wanted {
                    self.expect(&value, text.clone(), &what);
                }
                value
            }
            ParameterType::Codec | ParameterType::Handler { .. } => {
                self.codec_argument(&parameter.ty, argument, &what)
            }
            ParameterType::Items(item) => self.items_argument(argument, item, &what),
            ParameterType::Pairs(key, value) => self.pairs_argument(argument, key, value, &what),
            ParameterType::Strings => {
                let value = self.iterable_argument(argument, crate::builtin::Builtin::List);
                if let Some(item) = value.ty.sequence_item()
                    && item != text
                    && !item.has_error()
                {
                    self.diagnostics.push(
                        Diagnostic::new(
                            DiagnosticCode::error(404),
                            value.span,
                            format!(
                                "`join` joins items of type {text}, and these items are {item}"
                            ),
                        )
                        .with_rule(RuleId::LIB_03),
                    );
                }
                value
            }
        }
    }
}

impl BodyChecker<'_> {
    /// Checks the name of a codec or an error handler, which is a string literal so it can be
    /// resolved when compiling, and returns its number as an `int` constant.
    pub(super) fn codec_argument(
        &mut self,
        kind: &ParameterType,
        argument: &Expr,
        what: &str,
    ) -> TypedExpr {
        let span = argument.span();
        let Expr::Constant {
            value: Constant::Str(name),
            ..
        } = argument
        else {
            self.expression(argument);
            return self.error(
                Diagnostic::new(
                    DiagnosticCode::error(404),
                    span,
                    format!("{what} is a string literal, so TyPy can check it when compiling"),
                )
                .with_rule(RuleId::LIB_03),
                span,
            );
        };
        let (number, provided) = match kind {
            ParameterType::Handler { decoding } => (
                crate::codec::handler(name, *decoding),
                if *decoding {
                    "the error handlers `strict`, `ignore`, `replace`, `backslashreplace` and `surrogateescape`"
                } else {
                    "the error handlers `strict`, `ignore`, `replace`, `backslashreplace`, `xmlcharrefreplace` and `surrogateescape`"
                },
            ),
            _ => (
                crate::codec::codec(name),
                "the codecs `utf-8`, `ascii` and `latin-1`",
            ),
        };
        let Some(number) = number else {
            return self.error(
                Diagnostic::new(
                    DiagnosticCode::error(404),
                    span,
                    format!("TyPy provides {provided}, and {name:?} is none of them"),
                )
                .with_rule(RuleId::LIB_03),
                span,
            );
        };
        Self::typed(
            TypedExprKind::Constant(TypedConstant::Int(number.to_string())),
            Type::Int,
            span,
        )
    }
}

/// Returns `count` with the noun, plural unless the count is one: "1 argument", "2 arguments".
fn counted(count: usize, noun: &str) -> String {
    if count == 1 {
        format!("1 {noun}")
    } else {
        format!("{count} {noun}s")
    }
}
