//! Checks calls of program functions and classes: arity, keywords, defaults and argument
//! types.

use typy_diagnostics::{Diagnostic, DiagnosticCode, RuleId, Span};
use typy_source::syntax::{Expr, Keyword};

use super::BodyChecker;
use super::binding::{Parameters, Slot};
use super::unpacked_arguments::{Reach, Tail};
use crate::builtin::Builtin;
use crate::exception::BuiltinException;
use crate::typed::{Argument, FunctionId, Passed, TypedConstant, TypedExpr, TypedExprKind};
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
        // A nested function in reach is called directly by its name (FUN-04).
        if let Some(call) = self.nested_call(name, name_span, arguments, keywords, span) {
            return call;
        }
        if let Some(symbol) = self.locals.get(name).copied() {
            return self.call_local(symbol, (name, name_span), arguments, keywords, span);
        }
        // A captured variable that holds a function value is called through it.
        if let Some(callee) = self.closure_read(name, name_span) {
            if let Type::Callable { .. } = callee.ty {
                return self.call_value(callee, arguments, keywords, span);
            }
            for argument in arguments {
                self.expression(argument);
            }
            return self.error(
                Diagnostic::new(
                    DiagnosticCode::error(401),
                    name_span,
                    format!(
                        "`{name}` is {} and cannot be called",
                        callee.ty.with_article()
                    ),
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
            if let Type::Callable { .. } = read.ty {
                return self.call_value(read, arguments, keywords, span);
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

    /// Returns the call of the local variable `symbol`, named `name`: of the class it holds,
    /// or through the function value it holds, which a test may have narrowed it to from an
    /// optional value; E0401 for any other value.
    fn call_local(
        &mut self,
        symbol: crate::typed::SymbolId,
        (name, name_span): (&str, Span),
        arguments: &[Expr],
        keywords: &[Keyword],
        span: Span,
    ) -> TypedExpr {
        // A variable called before an assignment gives it a type is read before it is
        // assigned, which the read reports (TYP-12).
        let Some(ty) = self.types.get(&symbol).cloned() else {
            let read = self.name(name, name_span);
            for argument in arguments {
                self.expression(argument);
            }
            return TypedExpr { span, ..read };
        };
        if let Some(made) = self.call_class_value(&ty, arguments, keywords, span) {
            return made;
        }
        if matches!(
            ty,
            Type::Callable { .. } | Type::Optional(_) | Type::Union(_)
        ) {
            let callee = self.name(name, name_span);
            if let Type::Callable { .. } = callee.ty {
                return self.call_value(callee, arguments, keywords, span);
            }
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
        self.error(
            Diagnostic::new(
                DiagnosticCode::error(401),
                name_span,
                format!("`{name}` is {} and cannot be called", ty.with_article()),
            )
            .with_rule(RuleId::EXP_02),
            span,
        )
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

    /// Binds the arguments of a call to the parameters of `signature` after the first `skip`:
    /// each positional argument to the next parameter that takes one by position, then to
    /// `*args`, and each keyword argument to the parameter it names that is not
    /// positional-only, then to `**kwargs`, as CPython binds them (FUN-03, EXP-17). The
    /// items of a fixed tuple `*` unpacks bind as positional arguments; from the first
    /// `*iterable` of unknown length on, the positional values, and the entries of each
    /// `**dict`, bind when the call runs (ADR-0073).
    pub(super) fn bind_to(
        &mut self,
        signature: &crate::typed::FunctionSignature,
        skip: usize,
        name: &str,
        arguments: &[Expr],
        keywords: &[Keyword],
        span: Span,
    ) -> Result<Vec<Argument>, TypedExpr> {
        // The call takes what it unpacks, so its own errors stand (ADR-0073).
        if super::call_expression::unpacking(arguments, keywords).is_some() {
            self.unpacking_calls.push(span);
        }
        let parameters = &signature.parameters[skip.min(signature.parameters.len())..];
        let shape = Parameters::of(parameters);
        let mut bound: Vec<Argument> = Vec::new();
        let mut filled = vec![false; parameters.len()];
        let mut position = 0;
        let mut tail: Option<Tail> = None;
        for argument in arguments {
            if let Some(tail) = &mut tail {
                self.add_to_tail(tail, argument);
                continue;
            }
            if let Expr::Starred { value, .. } = argument {
                let value = Self::version_tuple(self.expression(value));
                if let Type::Tuple(items) = &value.ty {
                    let count = items.len();
                    let shapes = (&shape, parameters);
                    let argument =
                        self.bind_items(shapes, (value, position), (&mut filled, skip), name)?;
                    bound.push(argument);
                    position += count;
                } else {
                    tail = Some(self.start_tail(parameters, position, value));
                }
                continue;
            }
            let index = position;
            position += 1;
            let slot = shape.of_position(index);
            let value = self.expression_expecting(argument, slot.and_then(Slot::expected));
            let Some(slot) = slot else {
                let (positional, keyword_only) = (shape.positional(), shape.has_keyword_only());
                return Err(self.too_many_positional(name, positional, keyword_only, &value, span));
            };
            if let Slot::Named(index, _) = slot {
                filled[index] = true;
            }
            bound.push(self.bound_argument(slot, None, value, (parameters, skip)));
        }
        let mut reach = Reach {
            tail_from: tail.as_ref().map(Tail::start),
            entries: false,
        };
        if let Some(tail) = tail {
            bound.push(self.finish_tail(tail, parameters, skip));
        }
        let mut entries = None;
        for keyword in keywords {
            let Some(keyword_name) = &keyword.name else {
                bound.extend(self.bind_entries(&keyword.value, &mut entries));
                reach.entries = true;
                continue;
            };
            let slot = shape.of_keyword(&keyword_name.name);
            let value = self.expression_expecting(&keyword.value, slot.and_then(Slot::expected));
            let message = match slot {
                None => format!("`{name}` has no parameter named `{}`", keyword_name.name),
                Some(Slot::PositionalOnly(_)) => {
                    let message = format!(
                        "`{name}` takes `{}` by position only, not by keyword",
                        keyword_name.name
                    );
                    return Err(self.error(
                        Diagnostic::new(DiagnosticCode::error(404), keyword_name.span, message)
                            .with_rule(RuleId::FUN_03),
                        span,
                    ));
                }
                Some(Slot::Named(index, _)) if filled[index] => {
                    format!("argument `{}` is given more than once", keyword_name.name)
                }
                Some(slot) => {
                    if let Slot::Named(index, _) = slot {
                        filled[index] = true;
                    }
                    let keyword = Some(keyword_name.name.as_str());
                    bound.push(self.bound_argument(slot, keyword, value, (parameters, skip)));
                    continue;
                }
            };
            return Err(self.error(
                Diagnostic::new(DiagnosticCode::error(404), keyword_name.span, message)
                    .with_rule(RuleId::TYP_01),
                span,
            ));
        }
        if let Some(item) = &entries {
            self.check_entries_reach(item, parameters, &filled, span);
        }
        self.bind_defaults((parameters, &filled, reach), skip, name, span, &mut bound)?;
        Ok(bound)
    }

    /// Returns the argument `value` bound to `slot`, after checking its type: an argument
    /// `*args` or `**kwargs` takes is held as the parameter's items are, and keeps its
    /// keyword.
    fn bound_argument(
        &mut self,
        slot: Slot<'_>,
        keyword: Option<&str>,
        value: TypedExpr,
        (parameters, skip): (&[crate::typed::SignatureParameter], usize),
    ) -> Argument {
        match slot {
            Slot::Named(index, ty) => {
                self.expect(
                    &value,
                    ty.clone(),
                    &format!("argument `{}`", parameters[index].name),
                );
                Argument {
                    parameter: index + skip,
                    passed: if keyword.is_some() {
                        Passed::Name
                    } else {
                        Passed::Position
                    },
                    value,
                }
            }
            Slot::Packed(index, item) => {
                let value = Self::converted(value, Some(item));
                let what = keyword.map_or_else(
                    || format!("an argument for `*{}`", parameters[index].name),
                    |keyword| format!("argument `{keyword}`"),
                );
                self.expect(&value, item.clone(), &what);
                Argument {
                    parameter: index + skip,
                    passed: keyword.map_or(Passed::Position, |keyword| {
                        Passed::Keyword(keyword.to_owned())
                    }),
                    value,
                }
            }
            Slot::PositionalOnly(_) => {
                unreachable!("a keyword for a positional-only parameter is reported")
            }
        }
    }

    /// Adds to `bound` the argument of each parameter no argument filled whose default a
    /// call gives each time, and reports a parameter that has no default and that no
    /// unpacked argument may fill when the call runs; `*args` and `**kwargs` take what is
    /// left, or nothing.
    fn bind_defaults(
        &mut self,
        (parameters, filled, reach): (&[crate::typed::SignatureParameter], &[bool], Reach),
        skip: usize,
        name: &str,
        span: Span,
        bound: &mut Vec<Argument>,
    ) -> Result<(), TypedExpr> {
        for (index, (parameter, is_filled)) in parameters.iter().zip(filled).enumerate() {
            if *is_filled || parameter.kind.is_variadic() {
                continue;
            }
            if let Some(factory) = &parameter.default_call {
                let value = self.default_call(factory, &parameter.ty, span);
                bound.push(Argument {
                    parameter: index + skip,
                    passed: Passed::Default,
                    value,
                });
                continue;
            }
            if parameter.default.is_none() && !reach.reaches(index, parameter.kind) {
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
        Ok(())
    }

    /// Returns the error of a positional argument, `value`, past the `positional` parameters
    /// of `name` that take one: those of a function whose other parameters are keyword-only
    /// (FUN-03), or all of them.
    fn too_many_positional(
        &mut self,
        name: &str,
        positional: usize,
        keyword_only: bool,
        value: &TypedExpr,
        span: Span,
    ) -> TypedExpr {
        let plural = if positional == 1 { "" } else { "s" };
        let (message, rule) = if keyword_only {
            (
                format!(
                    "`{name}` takes {positional} positional argument{plural}, and more are \
                     given; the rest are keyword-only"
                ),
                RuleId::FUN_03,
            )
        } else {
            (
                format!("`{name}` takes {positional} argument{plural}, and more are given"),
                RuleId::TYP_01,
            )
        };
        self.error(
            Diagnostic::new(DiagnosticCode::error(404), value.span, message).with_rule(rule),
            span,
        )
    }
}
