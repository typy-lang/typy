//! Checks lists: displays, empty displays that take an expected type, items, and methods
//! (DAT-07, DAT-11, DAT-12, TYP-05, LIB-03).

use typy_diagnostics::{Diagnostic, DiagnosticCode, RuleId, Span};
use typy_source::syntax::{BinaryOperator, CompareOperator, Expr};

use super::BodyChecker;
use crate::builtin::Builtin;
use crate::typed::{TypedExpr, TypedExprKind};
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
            Expr::Set { elements, span } => self.set_display(elements, expected, *span),
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
            Expr::Call {
                arguments,
                keywords,
                span,
                ..
            } if arguments.is_empty()
                && self
                    .called_builtin(expression)
                    .is_some_and(|(builtin, ..)| builtin == Builtin::Tuple) =>
            {
                self.call_tuple(arguments, keywords, expected, *span)
            }
            Expr::Call {
                arguments,
                keywords,
                span,
                ..
            } if arguments.is_empty()
                && self
                    .called_builtin(expression)
                    .is_some_and(|(builtin, ..)| builtin == Builtin::Set) =>
            {
                self.call_set(arguments, keywords, expected, *span)
            }
            other => {
                let value = self.expression(other);
                Self::converted(value, expected)
            }
        }
    }

    /// Returns `value` as a `tuple[T, ...]` when that is the expected type and `value` is a
    /// fixed tuple of items of type `T`, which Python's typing accepts there (DAT-14); returns
    /// any other value as it is.
    pub(super) fn converted(value: TypedExpr, expected: Option<&Type>) -> TypedExpr {
        match expected {
            Some(Type::TupleOf(item)) if value.ty.fits_tuple_of(item) => {
                let (ty, span) = (Type::TupleOf(item.clone()), value.span);
                Self::typed(TypedExprKind::TupleOf(Box::new(value)), ty, span)
            }
            _ => value,
        }
    }

    /// Returns the operands of `left op right` with a fixed tuple beside a `tuple[T, ...]` in
    /// `+` converted to one, as Python's typing accepts it there (DAT-14).
    pub(super) fn tuple_operands(
        operator: BinaryOperator,
        left: TypedExpr,
        right: TypedExpr,
    ) -> (TypedExpr, TypedExpr) {
        if operator != BinaryOperator::Add {
            return (left, right);
        }
        let left = match &right.ty {
            Type::TupleOf(_) => {
                let wanted = right.ty.clone();
                Self::converted(left, Some(&wanted))
            }
            _ => left,
        };
        let right = match &left.ty {
            Type::TupleOf(_) => {
                let wanted = left.ty.clone();
                Self::converted(right, Some(&wanted))
            }
            _ => right,
        };
        (left, right)
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

    /// Checks the operands of a comparison chain. An empty list display, or `set()`, takes its
    /// type from a neighbouring operand, so `xs == []`, `x in []` and `s == set()` need no
    /// annotation (TYP-05).
    pub(super) fn comparison_operands(
        &mut self,
        left: &Expr,
        comparators: &[Expr],
        operators: &[CompareOperator],
    ) -> Vec<TypedExpr> {
        let operands: Vec<&Expr> = std::iter::once(left).chain(comparators).collect();
        let kinds: Vec<Option<Empty>> = operands
            .iter()
            .map(|operand| self.empty_collection(operand))
            .collect();
        let mut typed: Vec<Option<TypedExpr>> = operands
            .iter()
            .zip(&kinds)
            .map(|(operand, kind)| kind.is_none().then(|| self.expression(operand)))
            .collect();
        for (position, kind) in kinds.iter().enumerate() {
            let Some(kind) = kind else {
                continue;
            };
            let expected = empty_type(*kind, &typed, operators, position);
            typed[position] =
                Some(self.expression_expecting(operands[position], expected.as_ref()));
        }
        let mut typed: Vec<TypedExpr> = typed.into_iter().flatten().collect();
        // A fixed tuple compared with a `tuple[T, ...]`, or looked for among them, becomes one
        // (DAT-14).
        for (position, operator) in operators.iter().enumerate() {
            let (left, right) = (typed[position].ty.clone(), typed[position + 1].ty.clone());
            let membership = matches!(operator, CompareOperator::In | CompareOperator::NotIn);
            let wanted = if membership {
                match &right {
                    Type::Set(item) => Some(item.as_ref().clone()),
                    _ => right.sequence_item().cloned(),
                }
            } else {
                Some(right.clone())
            };
            if let Some(wanted) = wanted.filter(|wanted| matches!(wanted, Type::TupleOf(_))) {
                let value = typed[position].clone();
                typed[position] = Self::converted(value, Some(&wanted));
            }
            if !membership && matches!(left, Type::TupleOf(_)) {
                let value = typed[position + 1].clone();
                typed[position + 1] = Self::converted(value, Some(&left));
            }
        }
        typed
    }

    /// Returns which empty collection an expression makes without an annotation: an empty list
    /// display, or `set()`.
    fn empty_collection(&self, expression: &Expr) -> Option<Empty> {
        if is_empty_list(expression) {
            return Some(Empty::List);
        }
        let (builtin, arguments, keywords) = self.called_builtin(expression)?;
        (builtin == Builtin::Set && arguments.is_empty() && keywords.is_empty())
            .then_some(Empty::Set)
    }

    /// Returns whether an expression is `set()`, an empty set that takes its type from where it
    /// is used.
    pub(super) fn is_empty_set_call(&self, expression: &Expr) -> bool {
        self.empty_collection(expression) == Some(Empty::Set)
    }

    /// Checks the items of a display. Without an expected item type, an empty display among the
    /// items takes the type of the first other item, so `[[1], []]` needs no annotation.
    pub(super) fn display_items(
        &mut self,
        elements: &[Expr],
        expected: Option<&Type>,
    ) -> Vec<TypedExpr> {
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
}

/// An empty collection that takes its type from where it is used.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Empty {
    /// `[]`.
    List,
    /// `set()`.
    Set,
}

impl Empty {
    /// Returns whether `ty` is a collection of this kind.
    fn holds(self, ty: &Type) -> bool {
        matches!(
            (self, ty),
            (Self::List, Type::List(_)) | (Self::Set, Type::Set(_))
        )
    }

    /// Returns the collection of this kind with items of type `item`.
    fn of(self, item: Type) -> Type {
        match self {
            Self::List => Type::List(Box::new(item)),
            Self::Set => Type::Set(Box::new(item)),
        }
    }
}

fn is_empty_list(expression: &Expr) -> bool {
    matches!(expression, Expr::List { elements, .. } if elements.is_empty())
}

/// Returns the type an empty collection at `position` of a comparison chain takes from its
/// left neighbour, or else from its right one: the neighbour's type when it is a collection of
/// the same kind, or for `in` the collection that holds the neighbour, or the item type of the
/// collection that holds this one.
fn empty_type(
    kind: Empty,
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
            return Some(kind.of(neighbour.ty.clone()));
        }
        if kind.holds(&neighbour.ty) {
            return Some(neighbour.ty.clone());
        }
    }
    let (neighbour, operator) = (typed.get(position + 1)?.as_ref()?, operators[position]);
    if is_membership(operator) {
        // The empty collection is the item looked for, so it has the item type.
        return match &neighbour.ty {
            Type::List(item) | Type::Set(item) => Some(item.as_ref().clone()),
            _ => None,
        };
    }
    kind.holds(&neighbour.ty).then(|| neighbour.ty.clone())
}
