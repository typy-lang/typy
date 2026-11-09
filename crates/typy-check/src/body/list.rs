//! Checks lists: displays, empty displays that take an expected type, items, and methods
//! (DAT-07, DAT-11, DAT-12, TYP-05, LIB-03).

use typy_diagnostics::{Diagnostic, DiagnosticCode, RuleId, Span};
use typy_source::syntax::{CompareOperator, Expr, Identifier, Keyword};

use super::BodyChecker;
use super::iterable_call::Given;
use crate::builtin::Builtin;
use crate::method::Method;
use crate::typed::{TypedConstant, TypedExpr, TypedExprKind};
use crate::types::Type;

impl BodyChecker<'_> {
    /// Checks an expression whose type the context already knows, such as the value of an
    /// annotated assignment. An empty list display takes that type, which it cannot have of its
    /// own (TYP-05); any other expression is checked as usual.
    pub(super) fn expression_expecting(
        &mut self,
        expression: &Expr,
        expected: Option<&Type>,
    ) -> TypedExpr {
        match expression {
            Expr::List { elements, span } => self.list_display(elements, expected, *span),
            Expr::Call {
                arguments,
                keywords,
                span,
                ..
            } if arguments.is_empty()
                && self
                    .called_builtin(expression)
                    .is_some_and(|(builtin, ..)| builtin == Builtin::List) =>
            {
                self.call_list(arguments, keywords, expected, *span)
            }
            other => self.expression(other),
        }
    }

    /// Checks a list display: every item has the type of the first, and an empty display has the
    /// expected type.
    pub(super) fn list_display(
        &mut self,
        elements: &[Expr],
        expected: Option<&Type>,
        span: Span,
    ) -> TypedExpr {
        let expected_item = match expected {
            Some(Type::List(item)) => Some(item.as_ref().clone()),
            // The expected type already has its error, so the display gives no second one.
            Some(expected) if expected.has_error() => Some(Type::Error),
            _ => None,
        };
        let items = self.display_items(elements, expected_item.as_ref());
        let item_type = match (items.first(), expected_item) {
            (_, Some(expected)) => expected,
            (Some(first), None) => first.ty.clone(),
            (None, None) => {
                return self.error(
                    Diagnostic::new(
                        DiagnosticCode::error(410),
                        span,
                        "an empty list needs an annotation that names its item type",
                    )
                    .with_rule(RuleId::TYP_05)
                    .with_help("annotate the variable, such as `items: list[int] = []`"),
                    span,
                );
            }
        };
        for item in &items {
            if !item.ty.is_assignable_to(&item_type) {
                self.diagnostics.push(
                    Diagnostic::new(
                        DiagnosticCode::error(409),
                        item.span,
                        format!(
                            "the items of a list have one type; this is {}, and the list holds {item_type}",
                            item.ty.with_article()
                        ),
                    )
                    .with_rule(RuleId::DAT_07),
                );
            }
        }
        Self::typed(
            TypedExprKind::List(items),
            Type::List(Box::new(item_type)),
            span,
        )
    }

    /// Returns the value given for the method's keyword parameter, after reporting any keyword
    /// the method does not take.
    fn method_keyword<'call>(
        &mut self,
        method: Method,
        accepted: Option<&'static str>,
        keywords: &'call [Keyword],
        span: Span,
    ) -> Given<'call> {
        let mut value = None;
        for keyword in keywords {
            let name = keyword.name.as_ref().map(|name| name.name.as_str());
            if name.is_some() && name == accepted && value.is_none() {
                value = Some(&keyword.value);
                continue;
            }
            let diagnostic = match name {
                Some("key") if method == Method::ListSort => Diagnostic::new(
                    DiagnosticCode::error(201),
                    keyword.span,
                    "the `key` argument of `sort` is planned for TyPy 1.0, with function values (FUN-05)",
                )
                .with_rule(RuleId::FUN_05),
                Some(name) if Some(name) == accepted => Diagnostic::new(
                    DiagnosticCode::error(404),
                    keyword.span,
                    format!("`{name}` is given twice"),
                )
                .with_rule(RuleId::LIB_03),
                _ if accepted.is_none() => Diagnostic::new(
                    DiagnosticCode::error(404),
                    keyword.span,
                    format!("`{}` takes no keyword arguments", method.name()),
                )
                .with_rule(RuleId::LIB_03),
                _ => Diagnostic::new(
                    DiagnosticCode::error(404),
                    keyword.span,
                    format!(
                        "`{}` takes only the keyword argument `{}`",
                        method.name(),
                        accepted.unwrap_or_default()
                    ),
                )
                .with_rule(RuleId::LIB_03),
            };
            self.error(diagnostic, span);
            return Given::Refused;
        }
        value.map_or(Given::Absent, Given::Value)
    }

    /// Checks the operands of a comparison chain. An empty list display takes its type from a
    /// neighbouring operand, so `xs == []` and `x in []` need no annotation (TYP-05).
    pub(super) fn comparison_operands(
        &mut self,
        left: &Expr,
        comparators: &[Expr],
        operators: &[CompareOperator],
    ) -> Vec<TypedExpr> {
        let operands: Vec<&Expr> = std::iter::once(left).chain(comparators).collect();
        let mut typed: Vec<Option<TypedExpr>> = operands
            .iter()
            .map(|operand| (!is_empty_list(operand)).then(|| self.expression(operand)))
            .collect();
        for position in 0..operands.len() {
            if typed[position].is_some() {
                continue;
            }
            let expected = empty_list_type(&typed, operators, position);
            typed[position] =
                Some(self.expression_expecting(operands[position], expected.as_ref()));
        }
        typed.into_iter().flatten().collect()
    }

    /// Checks the items of a display. Without an expected item type, an empty display among the
    /// items takes the type of the first other item, so `[[1], []]` needs no annotation.
    fn display_items(&mut self, elements: &[Expr], expected: Option<&Type>) -> Vec<TypedExpr> {
        if expected.is_some() {
            return elements
                .iter()
                .map(|element| self.expression_expecting(element, expected))
                .collect();
        }
        let mut items: Vec<Option<TypedExpr>> = elements
            .iter()
            .map(|element| (!is_empty_list(element)).then(|| self.expression(element)))
            .collect();
        let sibling = items.iter().flatten().next().map(|item| item.ty.clone());
        for (item, element) in items.iter_mut().zip(elements) {
            if item.is_none() {
                *item = Some(self.expression_expecting(element, sibling.as_ref()));
            }
        }
        items.into_iter().flatten().collect()
    }

    /// Checks `list[index]`: the index is an `int`, and the item has the list's item type.
    pub(super) fn list_item(
        &mut self,
        list: TypedExpr,
        item: Type,
        index: &Expr,
        span: Span,
    ) -> TypedExpr {
        let index = self.expression(index);
        self.expect(&index, Type::Int, "an index");
        Self::typed(
            TypedExprKind::ListItem {
                list: Box::new(list),
                index: Box::new(index),
            },
            item,
            span,
        )
    }

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
            return Self::typed(
                TypedExprKind::Constant(TypedConstant::None),
                Type::Error,
                span,
            );
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
        let signature = method.signature(&receiver.ty);
        let keyword_value = self.method_keyword(method, signature.keyword, keywords, span);
        if let Given::Refused = keyword_value {
            return Self::typed(
                TypedExprKind::Constant(TypedConstant::None),
                Type::Error,
                span,
            );
        }
        let keyword_value = keyword_value.value();
        let parameters = signature.parameters;
        let positional = parameters.len() - usize::from(signature.keyword.is_some());
        if arguments.len() < signature.required || arguments.len() > positional {
            let expected = if positional == 0 && signature.keyword.is_some() {
                "no positional arguments".to_owned()
            } else if signature.required == positional {
                counted(positional, "argument")
            } else if signature.required == 0 {
                format!("at most {}", counted(positional, "argument"))
            } else {
                format!("{} to {} arguments", signature.required, positional)
            };
            let given = if arguments.len() == 1 { "is" } else { "are" };
            return self.error(
                Diagnostic::new(
                    DiagnosticCode::error(404),
                    span,
                    format!(
                        "`{}` takes {expected}, and {} {given} given",
                        method.name(),
                        arguments.len()
                    ),
                )
                .with_rule(RuleId::LIB_03),
                span,
            );
        }
        let keyword_parameter = keyword_value.and_then(|value| Some((value, parameters.last()?)));
        let arguments: Vec<TypedExpr> = arguments
            .iter()
            .zip(&parameters)
            .chain(keyword_parameter)
            .map(|(argument, parameter)| {
                let value = self.expression_expecting(argument, Some(parameter));
                self.expect(
                    &value,
                    parameter.clone(),
                    &format!("the argument of `{}`", method.name()),
                );
                value
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
}

/// Returns `count` with the noun, plural unless the count is one: "1 argument", "2 arguments".
fn counted(count: usize, noun: &str) -> String {
    if count == 1 {
        format!("1 {noun}")
    } else {
        format!("{count} {noun}s")
    }
}

fn is_empty_list(expression: &Expr) -> bool {
    matches!(expression, Expr::List { elements, .. } if elements.is_empty())
}

/// Returns the type an empty list display at `position` of a comparison chain takes from its
/// left neighbour, or else from its right one: the neighbour's list type, or for `in` the list
/// type that holds or is held by the neighbour.
fn empty_list_type(
    typed: &[Option<TypedExpr>],
    operators: &[CompareOperator],
    position: usize,
) -> Option<Type> {
    let is_membership = |operator: CompareOperator| {
        matches!(operator, CompareOperator::In | CompareOperator::NotIn)
    };
    let left = position
        .checked_sub(1)
        .and_then(|before| Some((typed[before].as_ref()?, operators[before])));
    if let Some((neighbour, operator)) = left {
        if is_membership(operator) {
            return Some(Type::List(Box::new(neighbour.ty.clone())));
        }
        if matches!(neighbour.ty, Type::List(_)) {
            return Some(neighbour.ty.clone());
        }
    }
    let (neighbour, operator) = (typed.get(position + 1)?.as_ref()?, operators[position]);
    match &neighbour.ty {
        Type::List(item) if is_membership(operator) => Some(item.as_ref().clone()),
        Type::List(_) => Some(neighbour.ty.clone()),
        _ => None,
    }
}
