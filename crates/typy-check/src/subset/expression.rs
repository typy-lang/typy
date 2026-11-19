//! Checks expressions and annotations against the feature table.

use typy_diagnostics::{Diagnostic, DiagnosticCode, RuleId, Span};
use typy_source::syntax::{BinaryOperator, Comprehension, Constant, Expr, NodeKind};

use super::Walker;
use super::feature::{Feature, feature, planned};

impl Walker {
    /// Checks the fields of an f-string and of its format specifications.
    fn fstring(&mut self, parts: &[typy_source::syntax::FStringPart]) {
        for part in parts {
            if let typy_source::syntax::FStringPart::Field(field) = part {
                self.expression(&field.value);
                if let Some(spec) = &field.spec {
                    self.fstring(spec);
                }
            }
        }
    }

    /// Checks a comprehension: each clause's target, source and conditions, and what it
    /// makes; an `async for` clause is refused as `async` is. The names its targets bind are
    /// in scope for what it makes and for its conditions, and a source is walked as one,
    /// where CPython refuses an assignment expression (EXP-13).
    fn comprehension(&mut self, made: &[&Expr], generators: &[Comprehension], kind: &'static str) {
        self.comprehension_kinds.push(kind);
        let mut names = Vec::new();
        for generator in generators {
            collect_names(&generator.target, &mut names);
        }
        for generator in generators {
            if generator.is_async {
                self.report(generator.span, feature(NodeKind::AsyncFor));
            }
            self.name_target(&generator.target);
            self.comprehension_sources += 1;
            self.expression(&generator.iter);
            self.comprehension_sources -= 1;
        }
        self.comprehension_names.push(names);
        for generator in generators {
            for condition in &generator.ifs {
                self.expression(condition);
            }
        }
        for expression in made {
            self.expression(expression);
        }
        self.comprehension_names.pop();
        self.comprehension_kinds.pop();
    }

    /// Checks a `yield` that is a statement, as SMT-11 accepts it: `yield value`, `yield from
    /// iterable`, or an assignment of either. Outside a function it is CPython's
    /// `SyntaxError`, E0103.
    pub(super) fn yield_statement(&mut self, expression: &Expr) {
        if self.scope.is_none() {
            self.syntax_error(expression.span(), "'yield' outside function");
        }
        self.yield_value(expression);
    }

    /// Checks a `yield` inside a larger expression: outside a function, or in a
    /// comprehension, CPython's `SyntaxError`; in a `lambda` or anywhere else in a function,
    /// planned for 1.0 (FUN-12).
    fn nested_yield(&mut self, expression: &Expr, span: Span) {
        if self.scope.is_none() {
            self.syntax_error(span, "'yield' outside function");
        } else if let Some(kind) = self.comprehension_kinds.last() {
            let message = format!("'yield' inside {kind}");
            self.syntax_error(span, &message);
        } else if self.lambda_depth > 0 {
            self.report(
                span,
                planned("generator `lambda` expressions", "1.0", RuleId::FUN_12),
            );
        } else {
            self.report(
                span,
                planned(
                    "`yield` and `yield from` inside a larger expression",
                    "1.0",
                    RuleId::FUN_12,
                ),
            );
        }
        self.yield_value(expression);
    }

    /// Walks the value a `yield` or a `yield from` gives.
    fn yield_value(&mut self, expression: &Expr) {
        match expression {
            Expr::Yield {
                value: Some(value), ..
            }
            | Expr::YieldFrom { value, .. } => self.expression(value),
            _ => {}
        }
    }

    /// Checks `target := value` with CPython's rules: the target may not be `__debug__` nor
    /// a name an enclosing comprehension binds, and the expression may not be in a
    /// comprehension's source (EXP-13).
    fn named(
        &mut self,
        target: &typy_source::syntax::Identifier,
        value: &Expr,
        span: typy_diagnostics::Span,
    ) {
        if target.name == "__debug__" {
            self.syntax_error(target.span, "cannot assign to __debug__");
        } else if self
            .comprehension_names
            .iter()
            .any(|names| names.contains(&target.name))
        {
            let message = format!(
                "assignment expression cannot rebind comprehension iteration variable '{}'",
                target.name
            );
            self.syntax_error(span, &message);
        } else if self.comprehension_sources > 0 {
            self.syntax_error(
                span,
                "assignment expression cannot be used in a comprehension iterable expression",
            );
        } else {
            self.assigned_name(&target.name);
        }
        self.expression(value);
    }

    /// Annotations name `int`, `float`, `bool`, `str`, `None`, `tuple[...]`, `list[...]`,
    /// `set[...]` or `dict[...]`; the checker reports any other name.
    /// Checks the item types of `tuple[...]`, `list[T]`, `set[T]`, `dict[K, V]` or `type[T]`;
    /// `tuple[int, ...]` is checked, and reported, by the checker.
    fn builtin_annotation(&mut self, index: &Expr) {
        match index {
            Expr::Tuple { elements, .. } => {
                for element in elements {
                    if !matches!(
                        element,
                        Expr::Constant {
                            value: Constant::Ellipsis,
                            ..
                        }
                    ) {
                        self.annotation(element);
                    }
                }
            }
            Expr::Constant {
                value: Constant::Ellipsis,
                ..
            } => {}
            single => self.annotation(single),
        }
    }

    /// Checks the types of `Callable[[A, ...], R]`: those of the list, then the result. The
    /// checker reports `...` for the list, and any other form (FUN-05).
    fn callable_annotation(&mut self, index: &Expr) {
        let items = match index {
            Expr::Tuple { elements, .. } => elements.iter().collect(),
            single => vec![single],
        };
        for item in items {
            match item {
                Expr::List { elements, .. } => {
                    for element in elements {
                        self.annotation(element);
                    }
                }
                Expr::Constant {
                    value: Constant::Ellipsis,
                    ..
                } => {}
                other => self.annotation(other),
            }
        }
    }

    pub(super) fn annotation(&mut self, annotation: &Expr) {
        match annotation {
            Expr::Subscript { value, index, .. } if matches!(value.as_ref(), Expr::Name { id, .. } if matches!(id.as_str(), "tuple" | "list" | "set" | "dict" | "type")) =>
            {
                self.builtin_annotation(index);
            }
            // `Iterator[T]` and `Iterable[T]`, by name or through `typing` or
            // `collections.abc`, `ClassVar[T]` and `Final[T]`; the checker resolves where the
            // name comes from (TYP-15, CLS-11, TYP-08).
            Expr::Subscript { value, index, .. }
                if matches!(
                    value.as_ref(),
                    Expr::Name { id, .. } | Expr::Attribute { attribute: typy_source::syntax::Identifier { name: id, .. }, .. }
                        if matches!(id.as_str(), "Iterator" | "Iterable" | "ClassVar" | "Final")
                ) =>
            {
                self.annotation(index);
            }
            // `Optional`, `Union`, `List`, `Dict`, `Set`, `Tuple` and `Type` of `typing`, by
            // name or through the module, which the checker resolves (LIB-17).
            Expr::Subscript { value, index, .. }
                if matches!(
                    value.as_ref(),
                    Expr::Name { id, .. } | Expr::Attribute { attribute: typy_source::syntax::Identifier { name: id, .. }, .. }
                        if matches!(id.as_str(), "Optional" | "Union" | "List" | "Dict" | "Set" | "Tuple" | "Type")
                ) =>
            {
                let items = match index.as_ref() {
                    Expr::Tuple { elements, .. } => elements.iter().collect(),
                    single => vec![single],
                };
                for item in items {
                    if !matches!(
                        item,
                        Expr::Constant {
                            value: Constant::Ellipsis,
                            ..
                        }
                    ) {
                        self.annotation(item);
                    }
                }
            }
            // `Callable[[A, ...], R]`, by name or through `typing` or `collections.abc`.
            Expr::Subscript { value, index, .. } if names_callable(value) => {
                self.callable_annotation(index);
            }
            // A generic class or type alias of the module, given its type arguments (GEN-01),
            // or a name of another module, which the checker reads (SMT-12).
            Expr::Subscript { value, index, .. }
                if matches!(value.as_ref(), Expr::Name { id, .. } if self.generic_classes.contains(id) || self.generic_aliases.contains(id) || self.imported.contains(id))
                    || matches!(value.as_ref(), Expr::Attribute { value: module, .. } if matches!(module.as_ref(), Expr::Name { id, .. } if self.imported_modules.contains(id))) =>
            {
                match index.as_ref() {
                    Expr::Tuple { elements, .. } => {
                        for element in elements {
                            self.annotation(element);
                        }
                    }
                    single => self.annotation(single),
                }
            }
            // Only the generic types of TYP-06, CLS-12 and TYP-15 are provided.
            Expr::Subscript { span, .. } => {
                self.report(
                    *span,
                    Feature::Excluded {
                        what: "generic annotations other than `list`, `dict`, `set`, `tuple`, `type`, `Iterator`, `Iterable` and their `typing` names, such as `frozenset[int]`,",
                    },
                );
            }
            // `T | None` is an optional type (TYP-06), and any other union a union, whose
            // members the checker reads (TYP-14).
            Expr::BinOp {
                operator: BinaryOperator::BitOr,
                left,
                right,
                ..
            } => {
                self.annotation(left);
                self.annotation(right);
            }
            Expr::Constant {
                value: Constant::Str(_),
                span,
            } => {
                self.report(
                    *span,
                    Feature::Excluded {
                        what: "annotations written as strings",
                    },
                );
            }
            other => self.expression(other),
        }
    }

    #[allow(clippy::too_many_lines, reason = "one arm per expression kind")]
    pub(super) fn expression(&mut self, expression: &Expr) {
        if let Expr::Name { id, .. } = expression {
            self.used_name(id);
        }
        match expression {
            Expr::BoolOp { values, .. } => {
                for value in values {
                    self.expression(value);
                }
            }
            Expr::BinOp {
                left,
                operator,
                right,
                span,
            } => {
                if *operator == BinaryOperator::MatMult {
                    self.report(
                        *span,
                        Feature::Excluded {
                            what: "the `@` operator",
                        },
                    );
                }
                self.expression(left);
                self.expression(right);
            }
            Expr::UnaryOp { operand, .. } => self.expression(operand),
            Expr::IfExp {
                test, body, orelse, ..
            } => {
                self.expression(test);
                self.expression(body);
                self.expression(orelse);
            }
            Expr::Compare {
                left, comparators, ..
            } => {
                self.expression(left);
                for comparator in comparators {
                    self.expression(comparator);
                }
            }
            Expr::Call {
                function,
                arguments,
                keywords,
                ..
            } => {
                match function.as_ref() {
                    Expr::Name { .. } => {}
                    // A method call: the checker decides whether the value has the method.
                    Expr::Attribute { value, .. } => self.expression(value),
                    // A generic class given its type arguments, such as `Box[int](1)`.
                    Expr::Subscript { value, index, .. } if matches!(value.as_ref(), Expr::Name { id, .. } if self.generic_classes.contains(id)) =>
                    {
                        self.annotation(index);
                    }
                    // A `ctypes` class made where it is called, such as `POINTER(T)()` or
                    // `(T * 3)(1, 2, 3)`, or a function value (FUN-05): the checker decides
                    // which it is.
                    other => self.expression(other),
                }
                for argument in arguments {
                    self.expression(argument);
                }
                // CPython refuses a call that names one keyword twice when it compiles it.
                for (index, keyword) in keywords.iter().enumerate() {
                    if let Some(name) = &keyword.name
                        && keywords[..index].iter().any(|before| {
                            before
                                .name
                                .as_ref()
                                .is_some_and(|earlier| earlier.name == name.name)
                        })
                    {
                        self.syntax_error(
                            keyword.span,
                            &format!("keyword argument repeated: {}", name.name),
                        );
                    }
                }
                for keyword in keywords {
                    if keyword.name.is_none() {
                        self.report(
                            keyword.span,
                            planned("dictionary unpackings with `**`", "0.9", RuleId::EXP_17),
                        );
                    }
                    self.expression(&keyword.value);
                }
            }
            Expr::Constant { value, span } => match value {
                Constant::Complex { .. } => self.diagnostics.push(
                    Diagnostic::new(
                        DiagnosticCode::error(104),
                        *span,
                        "imaginary literals are not supported",
                    )
                    .with_rule(RuleId::LEX_08),
                ),
                Constant::Ellipsis => self.report(
                    *span,
                    Feature::Excluded {
                        what: "`...` as a value",
                    },
                ),
                Constant::None
                | Constant::Bool(_)
                | Constant::Int(_)
                | Constant::Float(_)
                | Constant::Str(_)
                | Constant::StrPoints(_)
                | Constant::Bytes(_) => {}
            },
            Expr::Subscript { value, index, .. } => {
                self.expression(value);
                self.expression(index);
            }
            Expr::Name { .. } => {}
            Expr::Slice {
                lower, upper, step, ..
            } => {
                for bound in [lower, upper, step].into_iter().flatten() {
                    self.expression(bound);
                }
            }
            Expr::Tuple { elements, .. }
            | Expr::List { elements, .. }
            | Expr::Set { elements, .. } => {
                for element in elements {
                    self.expression(element);
                }
            }
            Expr::ListComp {
                element,
                generators,
                ..
            } => self.comprehension(&[element], generators, "list comprehension"),
            Expr::SetComp {
                element,
                generators,
                ..
            } => self.comprehension(&[element], generators, "set comprehension"),
            Expr::DictComp {
                key,
                value,
                generators,
                ..
            } => self.comprehension(&[key, value], generators, "dict comprehension"),
            Expr::Yield { span, .. } | Expr::YieldFrom { span, .. } => {
                self.nested_yield(expression, *span);
            }
            Expr::Named {
                target,
                value,
                span,
            } => self.named(target, value, *span),
            Expr::Lambda {
                parameters, body, ..
            } => self.lambda(parameters, body),
            Expr::Dict { keys, values, .. } => {
                for (key, value) in keys.iter().zip(values) {
                    match key {
                        Some(key) => self.expression(key),
                        None => self.report(
                            value.span(),
                            planned("dictionary unpackings with `**`", "0.9", RuleId::EXP_17),
                        ),
                    }
                    self.expression(value);
                }
            }
            Expr::Attribute { value, .. } => self.expression(value),
            Expr::Quoted { annotation, .. } => self.annotation(annotation),
            Expr::FString { parts, .. } => self.fstring(parts),
            Expr::Other { kind, span } => self.report(*span, feature(*kind)),
        }
    }
}

/// Adds the names a comprehension target binds.
fn collect_names(target: &Expr, names: &mut Vec<String>) {
    match target {
        Expr::Name { id, .. } => names.push(id.clone()),
        Expr::Tuple { elements, .. } | Expr::List { elements, .. } => {
            for element in elements {
                collect_names(element, names);
            }
        }
        _ => {}
    }
}

/// Returns true when `value` names `Callable`, by name or through `typing` or
/// `collections.abc`, which the checker resolves (FUN-05).
fn names_callable(value: &Expr) -> bool {
    matches!(
        value,
        Expr::Name { id, .. }
            | Expr::Attribute { attribute: typy_source::syntax::Identifier { name: id, .. }, .. }
            if id == "Callable"
    )
}
