//! Calls of `map` and `filter` (LIB-23, ADR-0089): an instance of a copy of the class the
//! module was given for the call's form, for the item types of the iterables and the type the
//! function gives. The iterables are typed first, so the function, a `lambda` or a built-in
//! among them, is typed against their items, as the `key` of `sorted` is; the function is
//! still evaluated first, as CPython evaluates it.

use typy_diagnostics::{Diagnostic, DiagnosticCode, RuleId, Span};
use typy_source::syntax::{Constant, Expr, Keyword};

use super::BodyChecker;
use crate::check::builtin_iterators::{filter_class, map_class};
use crate::typed::{Argument, ClassId, Passed, TypedExpr, TypedExprKind};
use crate::types::Type;

impl BodyChecker<'_> {
    /// Checks a call of `map` or `filter` when the module was given the class of its form;
    /// `None` for another name, or a form the module has no class for.
    pub(super) fn map_filter_call(
        &mut self,
        name: &str,
        (arguments, keywords): (&[Expr], &[Keyword]),
        span: Span,
    ) -> Option<TypedExpr> {
        let (class, function, iterables) = match (name, arguments) {
            ("map", [function, iterables @ ..]) if !iterables.is_empty() => {
                (map_class(iterables.len()), Some(function), iterables)
            }
            (
                "filter",
                [
                    Expr::Constant {
                        value: Constant::None,
                        ..
                    },
                    iterable,
                ],
            ) => (filter_class(false), None, std::slice::from_ref(iterable)),
            ("filter", [function, iterable]) => (
                filter_class(true),
                Some(function),
                std::slice::from_ref(iterable),
            ),
            ("map" | "filter", _) => return Some(self.map_filter_arity(name, arguments, span)),
            _ => return None,
        };
        if let Some(unpacked) = arguments
            .iter()
            .find(|argument| matches!(argument, Expr::Starred { .. }))
        {
            // The refusal names the later release, so it stands for the general one.
            self.unpacking_calls.push(span);
            self.refused_arguments(arguments);
            return Some(
                self.error(
                    Diagnostic::new(
                        DiagnosticCode::error(201),
                        unpacked.span(),
                        format!("`*` in a call of `{name}` is planned for TyPy 1.0 (LIB-25)"),
                    )
                    .with_rule(RuleId::LIB_25),
                    span,
                ),
            );
        }
        let class = self.program.class_ids.get(&class).copied()?;
        let strict = match self.strict_keyword(name, keywords, arguments, span) {
            Ok(strict) => strict,
            Err(refused) => return Some(refused),
        };
        let made = self.map_filter_object(class, name, (function, iterables, strict), span);
        Some(made.unwrap_or_else(|| Self::invalid_call(span)))
    }

    /// Returns what `map` is given for `strict=`, `False` without it, and nothing for
    /// `filter`; a keyword CPython refuses for the program's floor is reported.
    fn strict_keyword(
        &mut self,
        name: &str,
        keywords: &[Keyword],
        arguments: &[Expr],
        span: Span,
    ) -> Result<Option<TypedExpr>, TypedExpr> {
        let takes_strict = name == "map" && self.program.floor >= 14;
        let refused = keywords.iter().find(|keyword| {
            !takes_strict
                || keyword
                    .name
                    .as_ref()
                    .is_none_or(|given| given.name != "strict")
        });
        if let Some(keyword) = refused {
            return Err(self.map_filter_keyword(name, keyword, arguments, span));
        }
        let strict = keywords.first().map(|keyword| {
            let value = self.expression_expecting(&keyword.value, Some(&Type::Bool));
            self.expect(&value, Type::Bool, "`strict`");
            value
        });
        if name == "filter" {
            return Ok(None);
        }
        Ok(Some(strict.unwrap_or_else(|| {
            Self::typed(
                TypedExprKind::Constant(crate::typed::TypedConstant::Bool(false)),
                Type::Bool,
                span,
            )
        })))
    }

    /// Returns the new object of `class` for the call, or `None` after a reported error.
    fn map_filter_object(
        &mut self,
        class: ClassId,
        name: &str,
        (function, iterables, strict): (Option<&Expr>, &[Expr], Option<TypedExpr>),
        span: Span,
    ) -> Option<TypedExpr> {
        let mut sources = Vec::new();
        for iterable in iterables {
            sources.push(self.item_iterator(iterable, name));
        }
        let sources: Vec<(TypedExpr, Type)> = sources.into_iter().collect::<Option<_>>()?;
        let items: Vec<Type> = sources.iter().map(|(_, item)| item.clone()).collect();
        let mut types = items.clone();
        let mut passed = Vec::new();
        if let Some(function) = function {
            let (value, result) = self.mapped_function(function, name, &items)?;
            types.push(result);
            passed.push(value);
        }
        passed.extend(sources.into_iter().map(|(source, _)| source));
        passed.extend(strict);
        Some(self.new_iterator(class, types, passed, span))
    }

    /// Returns the iterator of `iterable` and the type of its items, as `iter()` makes it;
    /// `None` after a reported error.
    fn item_iterator(&mut self, iterable: &Expr, name: &str) -> Option<(TypedExpr, Type)> {
        let source = self.iterator_of(iterable, iterable.span());
        if source.ty.has_error() {
            return None;
        }
        if let Type::Iterator(item) = &source.ty {
            let item = item.as_ref().clone();
            return Some((source, item));
        }
        // An iterator class's object passes as the iterator it is, of what `__next__` gives
        // (TYP-20).
        let Some(next) = source
            .ty
            .class_id()
            .and_then(|class| self.program.method(class, "__next__"))
        else {
            self.iterated_item(&source, &format!("`{name}`"));
            return None;
        };
        let item = self.program.signatures[next.0 as usize].returns.clone();
        let wanted = Type::Iterator(Box::new(item.clone()));
        Some((Self::converted(source, Some(&wanted)), item))
    }

    /// Returns the function of `map` or `filter` typed against the item types `items`, and
    /// the type of its result; `None` after reporting a value that takes no such items, or
    /// for `filter` a result whose truth cannot be tested.
    fn mapped_function(
        &mut self,
        function: &Expr,
        name: &str,
        items: &[Type],
    ) -> Option<(TypedExpr, Type)> {
        let value = self.lambda(
            function,
            super::lambda::Expected {
                parameters: Some(items),
                returns: None,
            },
        );
        if value.ty.has_error() {
            return None;
        }
        let result = value
            .ty
            .callable_parts()
            .map(|(_, returns)| returns.clone());
        let wanted = result.as_ref().map(|result| Type::Callable {
            parameters: items.to_vec(),
            returns: Box::new(result.clone()),
        });
        let (Some(result), Some(wanted)) = (result, wanted) else {
            self.function_error(&value, name, items);
            return None;
        };
        if !value.ty.is_assignable_to(&wanted) {
            self.function_error(&value, name, items);
            return None;
        }
        if name == "filter" && !result.is_truth_testable() {
            self.check_truth(&Self::typed(
                TypedExprKind::Constant(crate::typed::TypedConstant::None),
                result.clone(),
                value.span,
            ));
            return None;
        }
        Some((Self::converted(value, Some(&wanted)), result))
    }

    /// Reports a function of `map` or `filter` that does not take an item of each iterable.
    fn function_error(&mut self, value: &TypedExpr, name: &str, items: &[Type]) {
        let listed: Vec<String> = items.iter().map(ToString::to_string).collect();
        self.diagnostics.push(
            Diagnostic::new(
                DiagnosticCode::error(404),
                value.span,
                format!(
                    "the function of `{name}` is called with an item of each iterable, of type \
                     {}, and this is {}",
                    listed.join(", "),
                    value.ty.with_article()
                ),
            )
            .with_rule(RuleId::LIB_23)
            .with_help(format!(
                "give a function of {} parameter{} of type {}",
                items.len(),
                if items.len() == 1 { "" } else { "s" },
                listed.join(", ")
            )),
        );
    }

    /// Returns a new object of the copy of `class` for `types`, made with `passed`.
    fn new_iterator(
        &mut self,
        class: ClassId,
        types: Vec<Type>,
        passed: Vec<TypedExpr>,
        span: Span,
    ) -> TypedExpr {
        let instance = self.program.class_copies.borrow_mut().applied(class, types);
        self.program.make_class_copies();
        let copy = instance.class_id().unwrap_or(class);
        let init = self.program.method(copy, "__init__");
        let parameters = init
            .map(|init| self.program.signatures[init.0 as usize].parameters.clone())
            .unwrap_or_default();
        let arguments = passed
            .into_iter()
            .enumerate()
            .map(|(position, value)| Argument {
                passed: Passed::Position,
                parameter: position + 1,
                value: Self::converted(
                    value,
                    parameters.get(position + 1).map(|parameter| &parameter.ty),
                ),
            })
            .collect();
        Self::typed(
            TypedExprKind::New {
                class: copy,
                init,
                arguments,
            },
            instance,
            span,
        )
    }

    /// Checks the arguments of a refused call for their own errors; a `lambda` or a built-in
    /// takes its type from the call, so it is left alone.
    fn refused_arguments(&mut self, arguments: &[Expr]) {
        for argument in arguments {
            let value = match argument {
                Expr::Starred { value, .. } => value.as_ref(),
                other => other,
            };
            if !matches!(value, Expr::Lambda { .. }) && !self.names_builtin(value) {
                self.expression(value);
            }
        }
    }

    /// Reports a call of `map` or `filter` with too few or too many arguments, with
    /// CPython's text.
    fn map_filter_arity(&mut self, name: &str, arguments: &[Expr], span: Span) -> TypedExpr {
        self.refused_arguments(arguments);
        let text = if name == "map" {
            "map() must have at least two arguments".to_owned()
        } else {
            format!("filter expected 2 arguments, got {}", arguments.len())
        };
        self.error(
            Diagnostic::new(
                DiagnosticCode::error(404),
                span,
                format!("{text}, as CPython raises `TypeError`"),
            )
            .with_rule(RuleId::LIB_23),
            span,
        )
    }

    /// Reports a keyword argument of `map` or `filter` CPython refuses, with its text for the
    /// program's Python floor: from 3.14 `map` takes `strict=` once.
    fn map_filter_keyword(
        &mut self,
        name: &str,
        keyword: &Keyword,
        arguments: &[Expr],
        span: Span,
    ) -> TypedExpr {
        self.refused_arguments(arguments);
        let given = keyword.name.as_ref().map(|name| name.name.as_str());
        let text = match given {
            Some(other) if name == "map" && self.program.floor >= 14 => {
                format!("map() got an unexpected keyword argument '{other}'")
            }
            _ => format!("{name}() takes no keyword arguments"),
        };
        self.error(
            Diagnostic::new(
                DiagnosticCode::error(404),
                keyword.span,
                format!("{text}, as CPython raises `TypeError`"),
            )
            .with_rule(RuleId::LIB_23),
            span,
        )
    }
}
