//! Checks calls of the functions of `math` that take iterables of numbers: `fsum`, `dist`,
//! `prod` and `sumprod` (LIB-12). Each iterable becomes a list of its items, which must be
//! `int`, `bool` or `float`, and the result type follows CPython's: `prod` of `float`s with
//! the default start, and `sumprod` with a `float`, give the `int` start or 0 for no items,
//! so their type is `int | float`.

use typy_diagnostics::{Diagnostic, DiagnosticCode, RuleId, Span};
use typy_source::syntax::{Expr, Keyword};

use super::BodyChecker;
use crate::math::{MathFunction, MathShape};
use crate::typed::{TypedConstant, TypedExpr, TypedExprKind};
use crate::types::Type;

impl BodyChecker<'_> {
    /// Checks `math.function(arguments)` for a function of iterables.
    pub(super) fn iterable_math_call(
        &mut self,
        function: MathFunction,
        arguments: &[Expr],
        keywords: &[Keyword],
        span: Span,
    ) -> TypedExpr {
        let name = format!("math.{}", function.name());
        let shape = function.shape();
        let wanted = if matches!(shape, MathShape::Dist | MathShape::Sumprod) {
            2
        } else {
            1
        };
        let start = match Self::start_keyword(&name, shape, keywords) {
            Ok(start) => start,
            Err(diagnostic) => return self.refused(*diagnostic, arguments, span),
        };
        if arguments.len() != wanted {
            let message = format!(
                "`{name}` takes {wanted} positional argument{}, and {} {} given",
                if wanted == 1 { "" } else { "s" },
                arguments.len(),
                if arguments.len() == 1 { "is" } else { "are" }
            );
            let diagnostic = Diagnostic::new(DiagnosticCode::error(404), span, message)
                .with_rule(RuleId::LIB_05);
            return self.refused(diagnostic, arguments, span);
        }
        let mut lists = Vec::new();
        for argument in arguments {
            match self.numbers(&name, argument) {
                Some(list) => lists.push(list),
                None => return Self::invalid_call(span),
            }
        }
        let start = match start {
            Some(expression) => match self.start_value(&name, expression) {
                Some(value) => value,
                None => return Self::invalid_call(span),
            },
            None => Self::typed(
                TypedExprKind::Constant(TypedConstant::None),
                Type::NoneType,
                span,
            ),
        };
        let result = result_type(shape, &lists, &start.ty);
        if shape == MathShape::Prod {
            lists.push(start);
        }
        Self::typed(
            TypedExprKind::MathCall {
                function,
                arguments: lists,
            },
            result,
            span,
        )
    }

    /// Returns the expression of `start`, the one keyword `prod` takes, or an error for any
    /// other keyword.
    fn start_keyword<'call>(
        name: &str,
        shape: MathShape,
        keywords: &'call [Keyword],
    ) -> Result<Option<&'call Expr>, Box<Diagnostic>> {
        let mut start = None;
        for keyword in keywords {
            let named = keyword.name.as_ref().map(|name| name.name.as_str());
            if shape == MathShape::Prod && named == Some("start") && start.is_none() {
                start = Some(&keyword.value);
                continue;
            }
            let message = match named {
                Some(other) if shape == MathShape::Prod => {
                    format!("`{name}` has no keyword argument `{other}`")
                }
                _ => format!("`{name}` takes no keyword arguments"),
            };
            return Err(Box::new(
                Diagnostic::new(DiagnosticCode::error(404), keyword.span, message)
                    .with_rule(RuleId::LIB_05),
            ));
        }
        Ok(start)
    }

    /// Reports `diagnostic` after checking the arguments, and returns an error.
    fn refused(&mut self, diagnostic: Diagnostic, arguments: &[Expr], span: Span) -> TypedExpr {
        for argument in arguments {
            self.expression(argument);
        }
        self.error(diagnostic, span)
    }

    /// Returns the list of an iterable argument's items, or `None` after reporting items that
    /// are not `int`, `bool` or `float`.
    fn numbers(&mut self, name: &str, argument: &Expr) -> Option<TypedExpr> {
        let (source, item) = self.loop_source(argument, &format!("`{name}`"));
        if item.has_error() {
            return None;
        }
        if !is_number(&item) {
            let planned = matches!(&item, Type::Union(members) if members.iter().all(is_number));
            let diagnostic = if planned {
                Diagnostic::new(
                    DiagnosticCode::error(201),
                    argument.span(),
                    format!("`{name}` of items of type {item} is planned for TyPy 0.5 (LIB-15)"),
                )
                .with_rule(RuleId::LIB_15)
            } else {
                Diagnostic::new(
                    DiagnosticCode::error(404),
                    argument.span(),
                    format!(
                        "`{name}` takes numbers, and these items are {}",
                        item.with_article()
                    ),
                )
                .with_rule(RuleId::LIB_12)
            };
            self.diagnostics.push(diagnostic);
            return None;
        }
        Some(Self::typed(
            TypedExprKind::ListOf(Box::new(source)),
            Type::List(Box::new(item)),
            argument.span(),
        ))
    }

    /// Returns the `start` of `prod`, an `int` or a `float`, or `None` after reporting another
    /// type.
    fn start_value(&mut self, name: &str, expression: &Expr) -> Option<TypedExpr> {
        let value = self.expression(expression);
        match value.ty {
            Type::Int | Type::Float => Some(value),
            Type::Error => None,
            // `prod([], start=True)` gives `True`, and with items an `int`.
            Type::Bool => {
                self.diagnostics.push(
                    Diagnostic::new(
                        DiagnosticCode::error(201),
                        value.span,
                        format!("`{name}` with a `bool` start is planned for TyPy 0.5 (LIB-15)"),
                    )
                    .with_rule(RuleId::LIB_15),
                );
                None
            }
            other => {
                self.diagnostics.push(
                    Diagnostic::new(
                        DiagnosticCode::error(201),
                        value.span,
                        format!(
                            "`{name}` with a start of type {other} is planned for TyPy 0.5 \
                             (LIB-15)"
                        ),
                    )
                    .with_rule(RuleId::LIB_15),
                );
                None
            }
        }
    }
}

/// Returns true for the types of numbers these functions take.
fn is_number(ty: &Type) -> bool {
    matches!(ty, Type::Int | Type::Bool | Type::Float)
}

/// Returns the type of the result.
fn result_type(shape: MathShape, lists: &[TypedExpr], start: &Type) -> Type {
    let whole = lists.iter().all(
        |list| matches!(&list.ty, Type::List(item) if matches!(**item, Type::Int | Type::Bool)),
    );
    match shape {
        MathShape::Prod if *start == Type::Float => Type::Float,
        MathShape::Prod | MathShape::Sumprod if whole => Type::Int,
        MathShape::Prod | MathShape::Sumprod => Type::union(vec![Type::Int, Type::Float]),
        _ => Type::Float,
    }
}
