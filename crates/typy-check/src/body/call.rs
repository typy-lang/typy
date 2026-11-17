//! Checks calls of program functions and classes: arity, keywords, defaults and argument
//! types.

use typy_diagnostics::{Diagnostic, DiagnosticCode, RuleId, Span};
use typy_source::syntax::{Expr, Keyword};

use super::BodyChecker;
use crate::builtin::Builtin;
use crate::exception::BuiltinException;
use crate::typed::{Argument, FunctionId, TypedConstant, TypedExpr, TypedExprKind};
use crate::types::Type;

impl BodyChecker<'_> {
    /// Returns the call of a variable that holds a class: a variable of `type[C]` holds the
    /// type object of `C` itself, such as `cls` in a copy of a class method, so calling it
    /// makes an instance of `C` (ADR-0025); one bound to a `ctypes` class makes an object of
    /// it (FFI-03). Returns `None` for a variable of any other type.
    fn call_class_value(
        &mut self,
        ty: &Type,
        arguments: &[Expr],
        keywords: &[Keyword],
        span: Span,
    ) -> Option<TypedExpr> {
        match ty {
            Type::TypeObject(Some(of)) => {
                let class = Self::program_class(of)?;
                Some(self.new_instance(class, arguments, keywords, span))
            }
            Type::CDataClass(class) => Some(self.cdata_new(class, arguments, keywords, span)),
            _ => None,
        }
    }

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
            if let Some(made) = self.call_class_value(&ty, arguments, keywords, span) {
                return made;
            }
            // A variable whose type is an error was reported where the error arose.
            if ty.has_error() {
                for argument in arguments {
                    self.expression(argument);
                }
                return Self::typed(
                    TypedExprKind::Constant(TypedConstant::None),
                    Type::Error,
                    span,
                );
            }
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
        if let Some(call) = self.rebound_call(name, arguments, keywords, span) {
            return call;
        }
        if let ("<hash>", [value]) = (name, arguments) {
            return self.hash_of(value, span);
        }
        if let Some(function) = self.program.function_ids.get(name).copied() {
            let call = self.call_function(function, name, arguments, keywords, span);
            return self.through_lazy(name, call);
        }
        if let Some(function) = self.program.module_functions.get(name).copied() {
            let call = self.module_function_call(function, arguments, keywords, span);
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
        // A type alias is a `typing.TypeAliasType`, which CPython does not call (SMT-12).
        if let Some(alias) = self.type_alias_value(name, name_span) {
            for argument in arguments {
                self.expression(argument);
            }
            if alias.ty == Type::Error {
                return TypedExpr { span, ..alias };
            }
            return self.error(
                Diagnostic::new(
                    DiagnosticCode::error(401),
                    name_span,
                    "'typing.TypeAliasType' object is not callable: CPython raises `TypeError`",
                )
                .with_rule(RuleId::SMT_12),
                span,
            );
        }
        // A module variable a function reads holds a value, which TyPy calls only when it is
        // a class's type object; CPython raises `TypeError` for an `int` or a `str`.
        if self.in_function() && self.program.module_variables.contains(name) {
            let read = self.name(name, name_span);
            if let Some(made) = self.call_class_value(&read.ty, arguments, keywords, span) {
                return made;
            }
            for argument in arguments {
                self.expression(argument);
            }
            if read.ty == Type::Error {
                return TypedExpr { span, ..read };
            }
            return self.error(
                Diagnostic::new(
                    DiagnosticCode::error(401),
                    name_span,
                    format!(
                        "`{name}` is {} and cannot be called",
                        read.ty.with_article()
                    ),
                )
                .with_rule(RuleId::EXP_02),
                span,
            );
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
        if self.program.generic(function).is_some() {
            return self.generic_call(function, name, arguments, keywords, span);
        }
        match self.bind_arguments(function, 0, name, arguments, keywords, span) {
            Ok(bound) => Self::typed(
                TypedExprKind::Call {
                    function,
                    arguments: bound,
                    dispatch: false,
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
    /// Returns the hash of `value`, which the `__hash__` of a dataclass returns (CLS-16).
    fn hash_of(&mut self, value: &Expr, span: Span) -> TypedExpr {
        let value = self.expression(value);
        if !value.ty.has_error() && !value.ty.is_hashable() {
            return self.error(crate::check::unhashable(&value.ty, span), span);
        }
        Self::typed(TypedExprKind::Hash(Box::new(value)), Type::Int, span)
    }

    /// Returns the call of the function that gives a left-out argument its default each
    /// time, a dataclass field's `default_factory` (CLS-15). The function is of the module
    /// that defines the dataclass, by a name no other function has.
    fn default_call(&mut self, factory: &str, ty: &Type, span: Span) -> TypedExpr {
        let function = self.program.function_ids.get(factory).copied().or_else(|| {
            self.program
                .scopes
                .values()
                .find_map(|scope| scope.function_ids.get(factory).copied())
        });
        let Some(function) = function else {
            return Self::typed(
                TypedExprKind::Constant(TypedConstant::None),
                Type::Error,
                span,
            );
        };
        let call = Self::typed(
            TypedExprKind::Call {
                function,
                arguments: Vec::new(),
                dispatch: false,
            },
            self.program.signatures[function.0 as usize].returns.clone(),
            span,
        );
        Self::converted(call, Some(ty))
    }

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
        self.bind_to(&signature, skip, name, arguments, keywords, span)
    }

    /// Binds the arguments of a call to the parameters of `signature` after the first `skip`.
    pub(super) fn bind_to(
        &mut self,
        signature: &crate::typed::FunctionSignature,
        skip: usize,
        name: &str,
        arguments: &[Expr],
        keywords: &[Keyword],
        span: Span,
    ) -> Result<Vec<Argument>, TypedExpr> {
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
        for (index, (parameter, is_filled)) in parameters.iter().zip(&filled).enumerate() {
            if !is_filled && let Some(factory) = &parameter.default_call {
                let value = self.default_call(factory, &parameter.ty, span);
                bound.push(Argument {
                    parameter: index + skip,
                    value,
                });
                continue;
            }
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
