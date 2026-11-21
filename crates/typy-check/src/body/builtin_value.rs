//! Reads a built-in function, or a method of a built-in type read through the type, as a
//! value (FUN-08, ADR-0077): a hidden function of the `Callable` it fills, whose body calls
//! the built-in, made once for each built-in and `Callable` type of a module. Its object is
//! written as CPython writes the built-in.

use typy_diagnostics::{Diagnostic, DiagnosticCode, Diagnostics, RuleId, Span};
use typy_source::syntax::{Expr, Identifier, ParameterKind, Stmt};

use super::BodyChecker;
use crate::builtin::Builtin;
use crate::check::Definition;
use crate::method::Method;
use crate::typed::{
    FunctionId, FunctionSignature, SignatureParameter, TypedConstant, TypedExpr, TypedExprKind,
};
use crate::types::{Type, TypeObjectOf};

/// The built-in a hidden function calls.
#[derive(Clone, Copy)]
enum Called<'name> {
    /// A built-in function, such as `len`, called with every argument.
    Function(&'name str),
    /// A method of the built-in type `class`, such as `str.upper`, called on the first
    /// argument with the others.
    Method {
        class: &'name str,
        method: &'name str,
    },
}

impl Called<'_> {
    /// Returns the name the hidden function keeps: `len`, or `str.upper` for a method.
    fn qualified_name(self) -> String {
        match self {
            Self::Function(name) => name.to_owned(),
            Self::Method { class, method } => format!("{class}.{method}"),
        }
    }
}

impl BodyChecker<'_> {
    /// Returns the built-in function, or the method of a built-in type, that `expression`
    /// names as a value of the `Callable` whose parameters are `parameters` and whose result
    /// is `returns`, when it names one the program does not bind, or `None` for any other
    /// expression.
    pub(super) fn builtin_as_callable(
        &mut self,
        expression: &Expr,
        parameters: &[Type],
        returns: Option<&Type>,
    ) -> Option<TypedExpr> {
        match expression {
            Expr::Name { id, span } => {
                let builtin = self.unbound_builtin_function(id, *span)?;
                let called = Called::Function(builtin.name());
                Some(self.builtin_value(called, *span, parameters, returns))
            }
            Expr::Attribute {
                value,
                attribute,
                span,
            } => {
                let Expr::Name {
                    id,
                    span: name_span,
                } = value.as_ref()
                else {
                    return None;
                };
                let class = TypeObjectOf::named(id).filter(|_| !self.binds(id, *name_span))?;
                self.builtin_method_value((&class, attribute), *span, parameters, returns)
            }
            _ => None,
        }
    }

    /// Returns the method `attribute` of the built-in type `class` as a value of the
    /// `Callable` of `parameters`, whose first parameter is the receiver; `None` when the type
    /// has no such method, and E0404 when the first parameter is of another type.
    fn builtin_method_value(
        &mut self,
        (class, attribute): (&TypeObjectOf, &Identifier),
        span: Span,
        parameters: &[Type],
        returns: Option<&Type>,
    ) -> Option<TypedExpr> {
        let class_name = class.name();
        let qualified = format!("{class_name}.{}", attribute.name);
        let receiver = parameters.first();
        if let Some(receiver) = receiver
            && TypeObjectOf::of(receiver).as_ref() == Some(class)
        {
            Method::of(receiver, &attribute.name)?;
            let called = Called::Method {
                class: class_name,
                method: &attribute.name,
            };
            return Some(self.builtin_value(called, span, parameters, returns));
        }
        let given = receiver.map_or_else(|| "no argument".to_owned(), Type::with_article);
        Some(
            self.error(
                Diagnostic::new(
                    DiagnosticCode::error(404),
                    span,
                    format!(
                        "`{qualified}` takes a {class_name} as its first argument, and this \
                     `Callable` passes {given}"
                    ),
                )
                .with_rule(RuleId::FUN_08),
                span,
            ),
        )
    }

    /// Returns true when `expression` names a built-in function, or a method of a built-in
    /// type read through the type, that the program does not bind.
    pub(super) fn names_builtin(&self, expression: &Expr) -> bool {
        match expression {
            Expr::Name { id, span } => self.unbound_builtin_function(id, *span).is_some(),
            Expr::Attribute {
                value, attribute, ..
            } => match value.as_ref() {
                Expr::Name { id, span } => TypeObjectOf::named(id)
                    .filter(|_| !self.binds(id, *span))
                    .and_then(|class| receiver_of(&class))
                    .is_some_and(|receiver| Method::of(&receiver, &attribute.name).is_some()),
                _ => false,
            },
            _ => false,
        }
    }

    /// Returns the built-in function `id` names where the program binds nothing of that name.
    pub(super) fn unbound_builtin_function(&self, id: &str, span: Span) -> Option<Builtin> {
        let builtin = Builtin::named(id).filter(|builtin| builtin.is_function())?;
        (!self.binds(id, span)).then_some(builtin)
    }

    /// Returns true when the program binds `id` where it is read, so it names no built-in.
    fn binds(&self, id: &str, span: Span) -> bool {
        self.locals.contains_key(id)
            || self.closure_read(id, span).is_some()
            || self.program.module_symbols.contains_key(id)
            || self.program.function_ids.contains_key(id)
            || self.program.class_ids.contains_key(id)
            || self.program.module_functions.contains_key(id)
    }

    /// Returns `builtin` as a value where no `Callable` is expected: of its one type for a
    /// built-in of one signature, such as `chr`, and E0203 for one whose arguments may be of
    /// several types, which a `Callable` type of the variable chooses.
    pub(super) fn builtin_without_context(&mut self, builtin: Builtin, span: Span) -> TypedExpr {
        let fixed = match builtin {
            Builtin::Chr => Some((Type::Int, Type::Str)),
            Builtin::Ord => Some((Type::Str, Type::Int)),
            _ => None,
        };
        let name = builtin.name();
        if let Some((parameter, result)) = fixed {
            return self.builtin_value(Called::Function(name), span, &[parameter], Some(&result));
        }
        self.error(
            Diagnostic::new(
                DiagnosticCode::error(203),
                span,
                format!(
                    "`{name}` takes arguments of more than one type, so as a value it needs the \
                     `Callable` type its context expects"
                ),
            )
            .with_rule(RuleId::FUN_08)
            .with_help(format!(
                "annotate the variable with the types you call it with, such as \
                 `measure: Callable[[str], int] = {name}`"
            )),
            span,
        )
    }

    /// Returns the hidden function that calls `called` for the `Callable` of `parameters`
    /// and `returns`, made the first time the module needs it, as a value. A call the built-in
    /// refuses is reported at `span`, and a result that does not pass unchanged as `returns`
    /// is E0404 (NFR-19).
    fn builtin_value(
        &mut self,
        called: Called<'_>,
        span: Span,
        parameters: &[Type],
        returns: Option<&Type>,
    ) -> TypedExpr {
        let name = called.qualified_name();
        let function = match self.program.builtin_value(&name, parameters) {
            Some(function) => function,
            None => match self.make_builtin_value(called, span, parameters) {
                Some(function) => function,
                None => {
                    return Self::typed(
                        TypedExprKind::Constant(TypedConstant::None),
                        Type::Error,
                        span,
                    );
                }
            },
        };
        let result = self.program.signatures[function.0 as usize].returns.clone();
        if let Some(wanted) = returns
            && !result.matches(wanted)
        {
            return self.error(
                Diagnostic::new(
                    DiagnosticCode::error(404),
                    span,
                    format!(
                        "`{name}` called with {} gives {}, and this `Callable` returns {wanted}",
                        argument_types(parameters),
                        result.with_article()
                    ),
                )
                .with_rule(RuleId::FUN_08),
                span,
            );
        }
        let ty = Type::Callable {
            parameters: parameters.to_vec(),
            returns: Box::new(result),
        };
        Self::typed(TypedExprKind::FunctionValue(function), ty, span)
    }

    /// Checks the hidden function of `parameters` that returns the call of `called`, whose
    /// result takes the call's type, and keeps it for the module; reports what the call
    /// refuses at `span` and returns `None` then.
    fn make_builtin_value(
        &mut self,
        called: Called<'_>,
        span: Span,
        parameters: &[Type],
    ) -> Option<FunctionId> {
        let function = self.program.next_function();
        let names: Vec<String> = (0..parameters.len())
            .map(|index| format!("value{index}"))
            .collect();
        let signature = FunctionSignature {
            parameters: names
                .iter()
                .zip(parameters)
                .map(|(each, ty)| SignatureParameter {
                    name: each.clone(),
                    ty: ty.clone(),
                    default: None,
                    default_call: None,
                    kind: ParameterKind::PositionalOnly,
                })
                .collect(),
            // The result takes the call's type once the body is checked.
            returns: Type::Error,
        };
        self.program.signatures.push(signature.clone());
        self.program.functions.push(None);
        let read = |each: &String| Expr::Name {
            id: each.clone(),
            span,
        };
        let (short_name, target, arguments) = match called {
            Called::Function(name) => (
                name.to_owned(),
                Expr::Name {
                    id: name.to_owned(),
                    span,
                },
                names.iter().map(read).collect(),
            ),
            Called::Method { method, .. } => (
                method.to_owned(),
                Expr::Attribute {
                    value: Box::new(read(&names[0])),
                    attribute: Identifier {
                        name: method.to_owned(),
                        span,
                    },
                    span,
                },
                names[1..].iter().map(read).collect(),
            ),
        };
        let statements = [Stmt::Return {
            value: Some(Expr::Call {
                function: Box::new(target),
                arguments,
                keywords: Vec::new(),
                span,
            }),
            span,
        }];
        let qualified_name = called.qualified_name();
        let definition = Definition {
            name: short_name,
            qualified_name: qualified_name.clone(),
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
        Self::check_body_of(self.program, function, &definition, None, &mut found);
        self.settle_lambda_result(function);
        if !found.is_empty() {
            for diagnostic in found.iter() {
                self.diagnostics.push(diagnostic.clone());
            }
            return None;
        }
        self.program
            .add_builtin_value(&qualified_name, parameters.to_vec(), function);
        Some(function)
    }
}

/// Returns a type of the values of the built-in type `class` whose methods TyPy gives, to look
/// a method up by, or `None` for a type without methods.
pub(super) fn receiver_of(class: &TypeObjectOf) -> Option<Type> {
    Some(match class {
        TypeObjectOf::Str => Type::Str,
        TypeObjectOf::Bytes => Type::Bytes,
        TypeObjectOf::List => Type::List(Box::new(Type::Int)),
        TypeObjectOf::Set => Type::Set(Box::new(Type::Int)),
        TypeObjectOf::Dict => Type::Dict(Box::new(Type::Int), Box::new(Type::Int)),
        _ => return None,
    })
}

/// Returns the types of a call's arguments as a message writes them: `an int and a str`.
fn argument_types(parameters: &[Type]) -> String {
    match parameters {
        [] => "no arguments".to_owned(),
        [one] => one.with_article(),
        [first @ .., last] => format!(
            "{} and {}",
            first
                .iter()
                .map(Type::with_article)
                .collect::<Vec<_>>()
                .join(", "),
            last.with_article()
        ),
    }
}
