//! Reads a built-in function, or a method of a built-in type read through the type, as a
//! value (FUN-08, ADR-0077): a hidden function of the `Callable` it fills, whose body calls
//! the built-in, made once for each built-in and `Callable` type of a module. Its object is
//! written as CPython writes the built-in.

use typy_diagnostics::{Diagnostic, DiagnosticCode, Diagnostics, RuleId, Span};
use typy_source::syntax::{Alias, Expr, Identifier, ParameterKind, Stmt};

use super::BodyChecker;
use super::builtin_module::{BuiltinName, ModuleFunction};
use crate::builtin::Builtin;
use crate::check::Definition;
use crate::method::Method;
use crate::typed::{
    BuiltinValue, FunctionId, FunctionSignature, SignatureParameter, TypedConstant, TypedExpr,
    TypedExprKind,
};
use crate::types::{Type, TypeObjectOf};

/// The built-in a hidden function calls.
#[derive(Clone, Copy)]
enum Called<'name> {
    /// A built-in function, such as `len`, called with every argument.
    Function(&'name str),
    /// A built-in type, such as `int`, called with every argument (FUN-14).
    Type(&'name str),
    /// A function of the module, `name`, called with every argument by position, so its
    /// defaults, `*args` and `**kwargs` take what the `Callable` does not pass (FUN-13).
    Adapter {
        name: &'name str,
        qualified: &'name str,
        function: FunctionId,
    },
    /// A method of the built-in type `class`, such as `str.upper`, called on the first
    /// argument with the others.
    Method {
        class: &'name str,
        method: &'name str,
    },
    /// A function of a module TyPy provides, such as `math.sqrt`, called with every argument
    /// after the hidden function imports the module itself.
    Module {
        module: &'name str,
        name: &'name str,
    },
}

impl<'name> Called<'name> {
    /// Returns the name a module's hidden functions are kept by: `len`, `str.upper` or
    /// `math.sqrt`.
    fn key(self) -> String {
        match self {
            Self::Function(name) | Self::Type(name) => name.to_owned(),
            Self::Adapter { function, .. } => format!("<adapter {}>", function.0),
            Self::Method { class, method } => format!("{class}.{method}"),
            Self::Module { module, name, .. } => format!("{module}.{name}"),
        }
    }

    /// Returns the `__qualname__` CPython gives the built-in: `len`, `str.upper` or `sqrt`.
    fn qualified_name(self) -> String {
        match self {
            Self::Module { name, .. } => name.to_owned(),
            Self::Adapter { qualified, .. } => qualified.to_owned(),
            other => other.key(),
        }
    }

    /// Returns the `__name__` CPython gives the built-in: `len`, `upper` or `sqrt`.
    const fn short_name(self) -> &'name str {
        match self {
            Self::Function(name)
            | Self::Type(name)
            | Self::Module { name, .. }
            | Self::Adapter { name, .. } => name,
            Self::Method { method, .. } => method,
        }
    }

    /// Returns what the hidden function stands for.
    const fn kind(self) -> BuiltinValue {
        match self {
            Self::Method { .. } => BuiltinValue::Method,
            Self::Type(_) => BuiltinValue::Type,
            Self::Adapter { function, .. } => BuiltinValue::Adapter(function),
            Self::Function(_) | Self::Module { .. } => BuiltinValue::Function,
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
        if let Some(adapted) = self.adapted_function(expression, parameters) {
            return Some(adapted(self, parameters, returns));
        }
        if let Some((module, name)) = self.module_function_named(expression) {
            let called = Called::Module {
                module: &module,
                name,
            };
            return Some(self.builtin_value(called, expression.span(), parameters, returns));
        }
        match expression {
            Expr::Name { id, span } => {
                if let Some(builtin) = self.unbound_builtin_function(id, *span) {
                    let called = Called::Function(builtin.name());
                    return Some(self.builtin_value(called, *span, parameters, returns));
                }
                // A class of the program, or an exception class, as a function is planned
                // (FUN-16).
                if self.program.class_ids.contains_key(id)
                    || crate::exception::BuiltinException::named(id).is_some()
                {
                    let message = format!(
                        "the class `{id}` as a `Callable` value is planned for TyPy 0.14 (FUN-16)"
                    );
                    return Some(
                        self.error(
                            Diagnostic::new(DiagnosticCode::error(201), *span, message)
                                .with_rule(RuleId::FUN_16)
                                .with_help("pass a function that makes the instance"),
                            *span,
                        ),
                    );
                }
                // A built-in type is a function of its arguments too (FUN-14).
                let builtin = Builtin::named(id)
                    .filter(|builtin| builtin.is_callable_type() && !self.binds(id, *span))?;
                let called = Called::Type(builtin.name());
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

    /// Returns, for a name of a module's function whose parameters do not take a `Callable`
    /// of `parameters` exactly, a maker of the adapter that calls it with every argument by
    /// position (FUN-13, ADR-0080); `None` for a function that fits as it is, which stays its
    /// own value, and for any other expression.
    #[allow(
        clippy::type_complexity,
        reason = "the maker borrows the name it was found by"
    )]
    fn adapted_function<'name>(
        &self,
        expression: &'name Expr,
        parameters: &[Type],
    ) -> Option<impl FnOnce(&mut Self, &[Type], Option<&Type>) -> TypedExpr + 'name> {
        let Expr::Name { id, span } = expression else {
            return None;
        };
        if self.locals.contains_key(id) || self.closure_read(id, *span).is_some() {
            return None;
        }
        let function = *self.program.function_ids.get(id)?;
        if self.program.generic(function).is_some() {
            return None;
        }
        // Only a function with a default, `*args`, `**kwargs` or a keyword-only parameter
        // takes another number of arguments than it names; any other keeps its own type.
        let signature = &self.program.signatures[function.0 as usize];
        let flexible = signature.parameters.iter().any(|parameter| {
            parameter.default.is_some()
                || !matches!(
                    parameter.kind,
                    ParameterKind::PositionalOnly | ParameterKind::Normal
                )
        });
        let exact = signature.parameters.len() == parameters.len()
            && signature.parameters.iter().all(|parameter| {
                matches!(
                    parameter.kind,
                    ParameterKind::PositionalOnly | ParameterKind::Normal
                )
            });
        if !flexible || exact {
            return None;
        }
        let qualified = self.program.functions[function.0 as usize]
            .as_ref()
            .map_or_else(|| id.clone(), |typed| typed.qualified_name.clone());
        let span = *span;
        Some(
            move |this: &mut Self, parameters: &[Type], returns: Option<&Type>| {
                let called = Called::Adapter {
                    name: id,
                    qualified: &qualified,
                    function,
                };
                this.builtin_value(called, span, parameters, returns)
            },
        )
    }

    /// Returns the module's function `id` names when it takes `*args` or `**kwargs`, which
    /// has no `Callable` type of its own, so it takes the one its context expects (FUN-13).
    pub(super) fn variadic_function(&self, id: &str, span: Span) -> Option<FunctionId> {
        if self.locals.contains_key(id) || self.closure_read(id, span).is_some() {
            return None;
        }
        let function = *self.program.function_ids.get(id)?;
        self.program.signatures[function.0 as usize]
            .parameters
            .iter()
            .any(|parameter| parameter.kind.is_variadic())
            .then_some(function)
    }

    /// Returns E0203 for the module's function `id`, which takes `*args` or `**kwargs`, read
    /// where no `Callable` is expected (FUN-13, ADR-0080).
    pub(super) fn variadic_without_context(&mut self, id: &str, span: Span) -> TypedExpr {
        self.error(
            Diagnostic::new(
                DiagnosticCode::error(203),
                span,
                format!(
                    "`{id}` takes `*args` or `**kwargs`, so as a value it needs the `Callable` \
                     type its context expects"
                ),
            )
            .with_rule(RuleId::FUN_13)
            .with_help(format!(
                "annotate the variable with the types you call it with, such as \
                 `combine: Callable[[int, int], int] = {id}`"
            )),
            span,
        )
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

    /// Returns the module and the name of the function of `math` or `sys` that `expression`
    /// names: `math.sqrt` after any `import math`, or `sqrt` after a `from math import sqrt`
    /// of the module; `None` for any other expression, a `ctypes` function among them.
    pub(super) fn module_function_named(
        &self,
        expression: &Expr,
    ) -> Option<(String, &'static str)> {
        let function = match expression {
            Expr::Name { id, span } => {
                if self.locals.contains_key(id) || self.closure_read(id, *span).is_some() {
                    return None;
                }
                *self.program.module_functions.get(id)?
            }
            Expr::Attribute {
                value, attribute, ..
            } => {
                let module = self.module_of(value)?;
                match self
                    .program
                    .builtin_lookup(&module, &attribute.name, attribute.span)
                {
                    Ok(BuiltinName::Function(function)) => function,
                    _ => return None,
                }
            }
            _ => return None,
        };
        let (module, name) = match function {
            ModuleFunction::Math(function) => ("math", function.name()),
            ModuleFunction::Exit => ("sys", "exit"),
            ModuleFunction::GetRecursionLimit => ("sys", "getrecursionlimit"),
            ModuleFunction::SetRecursionLimit => ("sys", "setrecursionlimit"),
            ModuleFunction::LoadLibrary
            | ModuleFunction::CtypesType(_)
            | ModuleFunction::Ctypes(_) => return None,
        };
        Some((module.to_owned(), name))
    }

    /// Returns true when `expression` names a built-in function, a method of a built-in type
    /// read through the type, or a function of `math` or `sys`, that the program does not
    /// bind otherwise.
    pub(super) fn names_builtin(&self, expression: &Expr) -> bool {
        if self.module_function_named(expression).is_some() {
            return true;
        }
        match expression {
            Expr::Name { id, span } => {
                self.unbound_builtin_function(id, *span).is_some()
                    || Builtin::named(id)
                        .is_some_and(|builtin| builtin.is_callable_type() && !self.binds(id, *span))
                    || self.variadic_function(id, *span).is_some()
            }
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

    /// Returns E0203 for a function of `math` or `sys`, `written` as the program names it,
    /// read where no `Callable` is expected, since its arguments may be of several types.
    pub(super) fn module_function_without_context(
        &mut self,
        written: &str,
        span: Span,
    ) -> TypedExpr {
        self.error(
            Diagnostic::new(
                DiagnosticCode::error(203),
                span,
                format!("`{written}` as a value needs the `Callable` type its context expects"),
            )
            .with_rule(RuleId::FUN_08)
            .with_help(format!(
                "annotate the variable with the types you call it with, such as \
                 `change: Callable[[float], float] = {written}`"
            )),
            span,
        )
    }

    /// Returns the method `attribute` of the built-in value `object`, such as `words.append`,
    /// as a value of the `Callable` the context expects of the read at `span`: a bound method
    /// of the hidden function that calls the method on its first argument; E0203 where none
    /// is expected.
    pub(super) fn bound_builtin_method(
        &mut self,
        object: TypedExpr,
        attribute: &Identifier,
        span: Span,
    ) -> TypedExpr {
        let class = TypeObjectOf::of(&object.ty);
        let expected = self.expected_method.take().filter(|(at, ..)| *at == span);
        let (Some(class), Some((_, parameters, returns))) = (class, expected) else {
            let message = format!(
                "`{}` of {} as a value needs the `Callable` type its context expects",
                attribute.name,
                object.ty.with_article()
            );
            return self.error(
                Diagnostic::new(DiagnosticCode::error(203), attribute.span, message)
                    .with_rule(RuleId::FUN_08)
                    .with_help("annotate the variable with the types you call it with"),
                span,
            );
        };
        let called = Called::Method {
            class: class.name(),
            method: &attribute.name,
        };
        let all: Vec<Type> = std::iter::once(object.ty.clone())
            .chain(parameters.iter().cloned())
            .collect();
        let value = self.builtin_value(called, span, &all, returns.as_ref());
        let TypedExprKind::FunctionValue(function) = value.kind else {
            return value;
        };
        let result = self.program.signatures[function.0 as usize].returns.clone();
        let ty = Type::Callable {
            parameters,
            returns: Box::new(result),
        };
        // A hidden function binds no class; the one entry is the method the value binds.
        Self::typed(
            TypedExprKind::BoundMethod {
                object: Box::new(object),
                methods: vec![(crate::typed::ClassId(0), function)],
                binds_class: false,
            },
            ty,
            span,
        )
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
        let name = called.key();
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
        let statements = hidden_body(called, &names, span);
        let qualified_name = called.qualified_name();
        let definition = Definition {
            name: called.short_name().to_owned(),
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
            .add_builtin_value(&called.key(), parameters.to_vec(), function, called.kind());
        Some(function)
    }
}

/// Returns the body of the hidden function of `called` whose parameters are `names`: the
/// call's `return`, after an import of the module for a function of a module, so it reads
/// the module wherever the program's own import stands.
fn hidden_body(called: Called<'_>, names: &[String], span: Span) -> Vec<Stmt> {
    let read = |each: &String| Expr::Name {
        id: each.clone(),
        span,
    };
    let attribute = |value: Expr, name: &str| Expr::Attribute {
        value: Box::new(value),
        attribute: Identifier {
            name: name.to_owned(),
            span,
        },
        span,
    };
    let (target, arguments): (Expr, Vec<Expr>) = match called {
        Called::Function(name) | Called::Type(name) | Called::Adapter { name, .. } => (
            Expr::Name {
                id: name.to_owned(),
                span,
            },
            names.iter().map(read).collect(),
        ),
        Called::Method { method, .. } => (
            attribute(read(&names[0]), method),
            names[1..].iter().map(read).collect(),
        ),
        Called::Module { module, name } => (
            attribute(
                Expr::Name {
                    id: module.to_owned(),
                    span,
                },
                name,
            ),
            names.iter().map(read).collect(),
        ),
    };
    let import = match called {
        Called::Module { module, .. } => Some(Stmt::Import {
            names: vec![Alias {
                name: Identifier {
                    name: module.to_owned(),
                    span,
                },
                as_name: None,
            }],
            is_lazy: false,
            span,
        }),
        Called::Function(_) | Called::Type(_) | Called::Adapter { .. } | Called::Method { .. } => {
            None
        }
    };
    import
        .into_iter()
        .chain([Stmt::Return {
            value: Some(Expr::Call {
                function: Box::new(target),
                arguments,
                keywords: Vec::new(),
                span,
            }),
            span,
        }])
        .collect()
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
