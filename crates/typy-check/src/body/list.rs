//! Checks lists: displays, empty displays that take an expected type, items, and methods
//! (DAT-07, DAT-11, TYP-05, LIB-03).

use typy_diagnostics::{Diagnostic, DiagnosticCode, RuleId, Span};
use typy_source::syntax::{Expr, Identifier, Keyword};

use super::BodyChecker;
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
        let items: Vec<TypedExpr> = elements
            .iter()
            .map(|element| self.expression_expecting(element, expected_item.as_ref()))
            .collect();
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
        if !item_type.is_list_item() {
            return self.error(
                Diagnostic::new(
                    DiagnosticCode::error(201),
                    span,
                    format!("lists of {item_type} are planned for TyPy 0.2 (DAT-07)"),
                )
                .with_rule(RuleId::DAT_07),
                span,
            );
        }
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
        if let Some(keyword) = keywords.first() {
            return self.error(
                Diagnostic::new(
                    DiagnosticCode::error(404),
                    keyword.span,
                    format!("`{}` takes no keyword arguments", method.name()),
                )
                .with_rule(RuleId::LIB_03),
                span,
            );
        }
        let (parameters, result) = match (method, &receiver.ty) {
            (Method::ListAppend, Type::List(item)) => (vec![item.as_ref().clone()], Type::NoneType),
            (Method::ListAppend, _) => unreachable!("`append` belongs to lists"),
        };
        if arguments.len() != parameters.len() {
            return self.error(
                Diagnostic::new(
                    DiagnosticCode::error(404),
                    span,
                    format!(
                        "`{}` takes {} arguments, and {} are given",
                        method.name(),
                        parameters.len(),
                        arguments.len()
                    ),
                )
                .with_rule(RuleId::LIB_03),
                span,
            );
        }
        let arguments: Vec<TypedExpr> = arguments
            .iter()
            .zip(&parameters)
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
            result,
            span,
        )
    }
}
