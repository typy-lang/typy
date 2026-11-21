//! Fits a nested function, a method or a `lambda` with defaults, `*args` or `**kwargs` to the
//! `Callable` its context expects (FUN-19, ADR-0080). An adapter of the `Callable`'s
//! parameters calls the function as a call binds arguments, and stands for it to `is`, `==`,
//! `hash` and `repr`: a nested function's or a `lambda`'s captures its function object, a
//! method's binds what the method binds and calls the method on it, and a function read
//! through its class is called directly.

use typy_diagnostics::{Diagnostic, DiagnosticCode, Diagnostics, RuleId, Span};
use typy_source::syntax::{Expr, ParameterKind, Stmt};

use super::BodyChecker;
use super::builtin_value::{Called, argument_types, hidden_parameters, hidden_signature};
use super::nested_function::Closure;
use crate::check::Definition;
use crate::typed::{
    BuiltinValue, ClassId, FunctionId, FunctionSignature, SignatureParameter, TypedConstant,
    TypedExpr, TypedExprKind,
};
use crate::types::Type;

/// The `Callable` a context expects: its parameters and its result.
pub(super) type Expected = (Vec<Type>, Option<Type>);

/// Returns true when a function of `signature` takes the arguments of a `Callable` of
/// `parameters`, after its first `skip` parameters, only through an adapter: it has a default,
/// `*args`, `**kwargs` or a keyword-only parameter, and its parameters are not the
/// `Callable`'s one for one.
pub(super) fn needs_adapter(
    signature: &FunctionSignature,
    skip: usize,
    parameters: &[Type],
) -> bool {
    let own = &signature.parameters[skip.min(signature.parameters.len())..];
    let flexible = own.iter().any(|parameter| {
        parameter.default.is_some() || parameter.default_call.is_some() || !positional(parameter)
    });
    let exact = own.len() == parameters.len() && own.iter().all(positional);
    flexible && !exact
}

/// Returns true when a call may pass `parameter` by position, and must pass it.
fn positional(parameter: &SignatureParameter) -> bool {
    matches!(
        parameter.kind,
        ParameterKind::PositionalOnly | ParameterKind::Normal
    )
}

impl BodyChecker<'_> {
    /// Returns the nested function `id` names as a value of the `Callable` of `parameters`
    /// and `returns`, through an adapter, when it takes the `Callable`'s arguments only
    /// through one; `None` for any other name.
    pub(super) fn adapted_nested_function(
        &mut self,
        id: &str,
        span: Span,
        (parameters, returns): (&[Type], Option<&Type>),
    ) -> Option<TypedExpr> {
        let function = self
            .function_held(id)
            .filter(|function| self.program.generic(*function).is_none())?;
        if !needs_adapter(&self.program.signatures[function.0 as usize], 0, parameters) {
            return None;
        }
        let original = self.local_function(id, span);
        if original.ty.has_error() {
            return Some(original);
        }
        Some(self.closure_adapter(original, (function, id), (parameters, returns)))
    }

    /// Returns a new adapter of `original`, the function object of the nested function or
    /// `lambda` `function`, named `name`, as a value of the `Callable` of `parameters` and
    /// `returns`: a function of those parameters that captures `original` and calls it by its
    /// name, which binds the arguments as a call by name does.
    pub(super) fn closure_adapter(
        &mut self,
        original: TypedExpr,
        (function, name): (FunctionId, &str),
        (parameters, returns): (&[Type], Option<&Type>),
    ) -> TypedExpr {
        let span = original.span;
        let adapter = self.program.next_function();
        let names = hidden_parameters(parameters.len());
        let signature = hidden_signature(&names, parameters);
        self.program.signatures.push(signature.clone());
        self.program.functions.push(None);
        let closure = Closure::adapting(name, function, original.ty.clone());
        self.program.closures.insert(adapter, closure.variables());
        let call = Expr::Call {
            function: Box::new(Expr::Name {
                id: name.to_owned(),
                span,
            }),
            arguments: names
                .iter()
                .map(|each| Expr::Name {
                    id: each.clone(),
                    span,
                })
                .collect(),
            keywords: Vec::new(),
            span,
        };
        let statements = [Stmt::Return {
            value: Some(call),
            span,
        }];
        // The adapter takes the original's names once every function is checked.
        let definition = Definition {
            name: name.to_owned(),
            qualified_name: name.to_owned(),
            class: None,
            takes_instance: false,
            copy_of: None,
            home: None,
            signature,
            type_parameters: Vec::new(),
            parameters: names.iter().map(|each| (each.clone(), span)).collect(),
            body: &statements,
            span,
        };
        let mut found = Diagnostics::new();
        Self::check_body_of(
            self.program,
            adapter,
            &definition,
            Some(closure),
            &mut found,
        );
        let result = self.settle_lambda_result(adapter);
        if !found.is_empty() {
            for diagnostic in found.iter() {
                self.diagnostics.push(diagnostic.clone());
            }
            return Self::typed(
                TypedExprKind::Constant(TypedConstant::None),
                Type::Error,
                span,
            );
        }
        if let Some(wanted) = returns
            && !result.matches(wanted)
        {
            let message = format!(
                "`{name}` called with {} gives {}, and this `Callable` returns {wanted}",
                argument_types(parameters),
                result.with_article()
            );
            return self.error(
                Diagnostic::new(DiagnosticCode::error(404), span, message)
                    .with_rule(RuleId::FUN_19),
                span,
            );
        }
        self.program.add_builtin_value(
            &format!("<closure adapter {}>", adapter.0),
            parameters.to_vec(),
            adapter,
            BuiltinValue::Adapter(function),
        );
        Self::typed(
            TypedExprKind::Lambda {
                function: adapter,
                captures: vec![original],
            },
            Type::Callable {
                parameters: parameters.to_vec(),
                returns: Box::new(result),
            },
            span,
        )
    }

    /// Returns the `Callable` the context expects of the method value read at `span`, when it
    /// gives one and `method`, after its first `skip` parameters, takes the `Callable`'s
    /// arguments only through an adapter.
    pub(super) fn adapted_callable(
        &self,
        method: FunctionId,
        skip: usize,
        span: Span,
    ) -> Option<Expected> {
        let (at, parameters, returns) = self.expected_method.as_ref()?;
        (*at == span
            && needs_adapter(
                &self.program.signatures[method.0 as usize],
                skip,
                parameters,
            ))
        .then(|| (parameters.clone(), returns.clone()))
    }

    /// Returns the adapter of `function`, the method `name` of `class`, to the `Callable` of
    /// `parameters` and `returns`, which calls it directly: with `binds`, a hidden function of
    /// what the method binds, then the `Callable`'s arguments; otherwise one of the
    /// `Callable`'s arguments. The error of a call that does not fit is returned as it is.
    pub(super) fn method_adapter(
        &mut self,
        (class, function): (ClassId, FunctionId),
        (name, binds): (&str, bool),
        (parameters, returns): (&[Type], Option<&Type>),
        span: Span,
    ) -> Result<FunctionId, TypedExpr> {
        let all: Vec<Type> = if binds {
            let receiver = self.program.signatures[function.0 as usize]
                .parameters
                .first()
                .map_or(Type::Error, |parameter| parameter.ty.clone());
            std::iter::once(receiver)
                .chain(parameters.iter().cloned())
                .collect()
        } else {
            parameters.to_vec()
        };
        let value = self.adapted_function_value((class, function), name, (&all, None), span);
        let TypedExprKind::FunctionValue(adapter) = value.kind else {
            return Err(value);
        };
        let result = &self.program.signatures[adapter.0 as usize].returns;
        match returns {
            Some(wanted) if !result.matches(wanted) => {
                let message = format!(
                    "`{}.{name}` called with {} gives {}, and this `Callable` returns {wanted}",
                    self.program.classes[class.0 as usize].name,
                    argument_types(parameters),
                    result.with_article()
                );
                Err(self.error(
                    Diagnostic::new(DiagnosticCode::error(404), span, message)
                        .with_rule(RuleId::FUN_19),
                    span,
                ))
            }
            _ => Ok(adapter),
        }
    }

    /// Returns `function`, the method or static method `name` of `class` read through the
    /// class, as a value of the `Callable` of `parameters` and `returns`, through an adapter
    /// that calls it directly.
    pub(super) fn adapted_function_value(
        &mut self,
        (class, function): (ClassId, FunctionId),
        name: &str,
        (parameters, returns): (&[Type], Option<&Type>),
        span: Span,
    ) -> TypedExpr {
        let qualified = format!("{}.{name}", self.program.classes[class.0 as usize].name);
        let called = Called::Direct {
            name,
            qualified: &qualified,
            function,
        };
        self.builtin_value(called, span, parameters, returns)
    }

    /// Returns `object.name`, a bound method of `object` and the method of its class among
    /// `methods`, through the adapter of each to the `Callable` `expected`. With `binds_class`
    /// the methods are copies of a class method, which bind the object's class.
    pub(super) fn adapted_bound_method(
        &mut self,
        object: TypedExpr,
        (methods, binds_class): (Vec<(ClassId, FunctionId)>, bool),
        name: &str,
        (parameters, returns): &Expected,
        span: Span,
    ) -> TypedExpr {
        let mut entries = Vec::with_capacity(methods.len());
        for each in methods {
            let callable = (parameters.as_slice(), returns.as_ref());
            match self.method_adapter(each, (name, true), callable, span) {
                Ok(adapter) => entries.push((each.0, adapter)),
                Err(error) => return error,
            }
        }
        let ty = self.adapted_type(parameters, entries.first().map(|(_, adapter)| *adapter));
        Self::typed(
            TypedExprKind::BoundMethod {
                object: Box::new(object),
                methods: entries,
                binds_class,
            },
            ty,
            span,
        )
    }

    /// Returns `object.name` of a static method some classes among `functions` define again as
    /// a method, through the adapter of each to the `Callable` `expected`: the method of a
    /// class in `binds` binds the object, and a static method is called directly.
    pub(super) fn adapted_chosen_function(
        &mut self,
        object: TypedExpr,
        (functions, binds): (Vec<(ClassId, FunctionId)>, Box<[ClassId]>),
        name: &str,
        (parameters, returns): &Expected,
        span: Span,
    ) -> TypedExpr {
        let mut entries = Vec::with_capacity(functions.len());
        for each in functions {
            let callable = (parameters.as_slice(), returns.as_ref());
            let bound = binds.contains(&each.0);
            match self.method_adapter(each, (name, bound), callable, span) {
                Ok(adapter) => entries.push((each.0, adapter)),
                Err(error) => return error,
            }
        }
        let ty = self.adapted_type(parameters, entries.first().map(|(_, adapter)| *adapter));
        Self::typed(
            TypedExprKind::ChosenFunction {
                object: Box::new(object),
                functions: entries,
                binds,
            },
            ty,
            span,
        )
    }

    /// Returns the `Callable` type of `parameters` and the result of `adapter`.
    pub(super) fn adapted_type(&self, parameters: &[Type], adapter: Option<FunctionId>) -> Type {
        let returns = adapter.map_or(Type::Error, |adapter| {
            self.program.signatures[adapter.0 as usize].returns.clone()
        });
        Type::Callable {
            parameters: parameters.to_vec(),
            returns: Box::new(returns),
        }
    }
}
