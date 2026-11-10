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
            Expr::Dict { keys, values, span } => self.dict_display(keys, values, expected, *span),
            Expr::ListComp { .. } | Expr::SetComp { .. } | Expr::DictComp { .. } => {
                self.comprehension(expression, expected)
            }
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
            Expr::Call {
                arguments,
                keywords,
                span,
                ..
            } if arguments.is_empty()
                && self
                    .called_builtin(expression)
                    .is_some_and(|(builtin, ..)| builtin == Builtin::Dict) =>
            {
                self.call_dict(arguments, keywords, expected, *span)
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
        let Some(expected) = expected else {
            return value;
        };
        match expected {
            Type::Optional(inner) if !matches!(value.ty, Type::Optional(_) | Type::Error) => {
                if value.ty != Type::NoneType && !value.ty.is_assignable_to(inner) {
                    return value;
                }
                // `None` and a value of the base become an optional value (ADR-0011).
                let value = if value.ty == Type::NoneType {
                    value
                } else {
                    Self::converted(value, Some(inner))
                };
                let span = value.span;
                Self::typed(TypedExprKind::Wrap(Box::new(value)), expected.clone(), span)
            }
            Type::TupleOf(item) => {
                // A display of the items becomes a tuple of them first, item by item.
                let value = match &value.kind {
                    TypedExprKind::Tuple(items) if !value.ty.fits_tuple_of(item) => {
                        let wanted = Type::Tuple(vec![item.as_ref().clone(); items.len()]);
                        Self::converted(value, Some(&wanted))
                    }
                    _ => value,
                };
                if !value.ty.fits_tuple_of(item) {
                    return value;
                }
                let (ty, span) = (Type::TupleOf(item.clone()), value.span);
                Self::typed(TypedExprKind::TupleOf(Box::new(value)), ty, span)
            }
            Type::Tuple(wanted) => match value.kind {
                TypedExprKind::Tuple(items) if items.len() == wanted.len() => {
                    let items: Vec<TypedExpr> = items
                        .into_iter()
                        .zip(wanted)
                        .map(|(item, wanted)| Self::converted(item, Some(wanted)))
                        .collect();
                    let ty = Type::Tuple(items.iter().map(|item| item.ty.clone()).collect());
                    Self::typed(TypedExprKind::Tuple(items), ty, value.span)
                }
                kind => TypedExpr { kind, ..value },
            },
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
        let item_type = match (joined_type(&items), expected_item) {
            (_, Some(expected)) => expected,
            (Some(joined), None) => joined,
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
        let items = converted_all(items, &item_type);
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
        if matches!(expression, Expr::Dict { keys, .. } if keys.is_empty()) {
            return Some(Empty::Dict);
        }
        let (builtin, arguments, keywords) = self.called_builtin(expression)?;
        if !arguments.is_empty() || !keywords.is_empty() {
            return None;
        }
        match builtin {
            Builtin::Set => Some(Empty::Set),
            Builtin::Dict => Some(Empty::Dict),
            _ => None,
        }
    }

    /// Returns whether an expression is `{}` or `dict()`, an empty dict that takes its type
    /// from where it is used.
    pub(super) fn is_empty_dict_call(&self, expression: &Expr) -> bool {
        self.empty_collection(expression) == Some(Empty::Dict)
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
            .map(|element| {
                self.empty_collection(element)
                    .is_none()
                    .then(|| self.expression(element))
            })
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
    /// `{}` or `dict()`.
    Dict,
}

impl Empty {
    /// Returns whether `ty` is a collection of this kind.
    fn holds(self, ty: &Type) -> bool {
        matches!(
            (self, ty),
            (Self::List, Type::List(_)) | (Self::Set, Type::Set(_)) | (Self::Dict, Type::Dict(..))
        )
    }

    /// Returns the collection of this kind that `in` finds `item` in; a dict finds its keys,
    /// and its value type is not known, so `x in {}` is not given one.
    fn of(self, item: Type) -> Option<Type> {
        match self {
            Self::List => Some(Type::List(Box::new(item))),
            Self::Set => Some(Type::Set(Box::new(item))),
            Self::Dict => None,
        }
    }
}

/// Returns the type the items of a display join to: the first item's type, made optional when
/// another item is `None` or optional, as Python's typing joins them (TYP-06).
pub(super) fn joined_type(items: &[TypedExpr]) -> Option<Type> {
    let (first, rest) = items.split_first()?;
    Some(
        rest.iter()
            .fold(first.ty.clone(), |joined, item| match (&joined, &item.ty) {
                (_, other) if other.is_assignable_to(&joined) => joined,
                (current, _) if current.is_assignable_to(&item.ty) => item.ty.clone(),
                (Type::NoneType, other) | (other, Type::NoneType) => Type::optional(other.clone()),
                // Type objects of different types join as `type` (CLS-12).
                (Type::TypeObject(_), Type::TypeObject(_)) => Type::TypeObject(None),
                _ => joined,
            }),
    )
}

/// Converts each item of a display to the type the display holds.
pub(super) fn converted_all(items: Vec<TypedExpr>, item_type: &Type) -> Vec<TypedExpr> {
    items
        .into_iter()
        .map(|item| BodyChecker::converted(item, Some(item_type)))
        .collect()
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
            return kind.of(neighbour.ty.clone());
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
            Type::Dict(key, _) => Some(key.as_ref().clone()),
            _ => None,
        };
    }
    kind.holds(&neighbour.ty).then(|| neighbour.ty.clone())
}
