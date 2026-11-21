//! Checks lists: displays, empty displays that take an expected type, items, and methods
//! (DAT-07, DAT-11, DAT-12, TYP-05, LIB-03).

use typy_diagnostics::{Diagnostic, DiagnosticCode, RuleId, Span};
use typy_source::syntax::{BinaryOperator, CompareOperator, Expr};

use super::BodyChecker;
use crate::builtin::Builtin;
use crate::typed::{SpreadKind, TypedExpr, TypedExprKind};
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
        // A generator expression yields the item type an iterable expected here has (EXP-15).
        if let Expr::GeneratorExp {
            element,
            generators,
            span,
        } = expression
        {
            let made = self.generator_expression(element, generators, expected, *span);
            return Self::converted(made, expected);
        }
        if let Expr::Tuple { elements, span } = expression
            && super::expression::has_starred(elements)
        {
            let made = self.spread_display(SpreadKind::Tuple, elements, expected, *span);
            return Self::converted(made, expected);
        }
        // A display or a collection call where `T | None` is expected is built as a `T`,
        // then held as an optional, as any other value is.
        // An empty display where an iterable is expected is a list, or a set, of its items.
        let iterable_list;
        let iterable_set;
        // A display where an abstract collection is expected is built as the built-in types
        // it holds (LIB-20).
        let held = expected.map(|expected| match expected {
            Type::Abstract(collection) => collection.display_members().without_abstract(),
            other => other.without_abstract(),
        });
        let target = match (held.as_ref(), expression) {
            (Some(Type::Optional(inner)), _) => Some(inner.as_ref()),
            (Some(Type::Iterable(item)), Expr::Set { .. } | Expr::SetComp { .. }) => {
                iterable_set = Type::Set(item.clone());
                Some(&iterable_set)
            }
            (Some(Type::Iterable(item)), _) => {
                iterable_list = Type::List(item.clone());
                Some(&iterable_list)
            }
            (other, _) => other,
        };
        // A display where a union is expected is built as the member of its kind (TYP-14).
        let member = match target {
            Some(Type::Union(members)) => {
                let called = self.called_builtin(expression).map(|(builtin, ..)| builtin);
                display_member(expression, called, members)
            }
            _ => None,
        };
        let target = member.as_ref().or(target);
        let value = match (expression, target) {
            // A `lambda`, and a built-in function, takes its parameter and result types from
            // the `Callable` expected (FUN-08).
            (
                Expr::Lambda { .. } | Expr::Name { .. } | Expr::Attribute { .. },
                Some(Type::Callable {
                    parameters,
                    returns,
                }),
            ) => self.lambda(
                expression,
                super::lambda::Expected {
                    parameters: Some(parameters),
                    returns: Some(returns),
                },
            ),
            // Each branch of a conditional gets the `Callable` expected, as a `lambda` needs.
            (
                Expr::IfExp {
                    test,
                    body,
                    orelse,
                    span,
                },
                Some(callable @ Type::Callable { .. }),
            ) => self.conditional((test, body, orelse), Some(callable), *span),
            _ => self.expression_for(expression, target),
        };
        Self::converted(value, expected)
    }

    /// Returns `expression` checked for the type `target` its context gives a display or a
    /// collection call.
    fn expression_for(&mut self, expression: &Expr, target: Option<&Type>) -> TypedExpr {
        match expression {
            Expr::List { elements, span } => self.list_display(elements, target, *span),
            Expr::Set { elements, span } => self.set_display(elements, target, *span),
            Expr::Dict { keys, values, span } => self.dict_display(keys, values, target, *span),
            Expr::ListComp { .. } | Expr::SetComp { .. } | Expr::DictComp { .. } => {
                self.comprehension(expression, target)
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
                self.call_list(arguments, keywords, target, *span)
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
                self.call_tuple(arguments, keywords, target, *span)
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
                self.call_set(arguments, keywords, target, *span)
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
                self.call_dict(arguments, keywords, target, *span)
            }
            other => self.expression(other),
        }
    }

    /// Returns `value` as a `tuple[T, ...]` when that is the expected type and `value` is a
    /// fixed tuple of items of type `T`, which Python's typing accepts there (DAT-14); returns
    /// any other value as it is.
    pub(super) fn converted(value: TypedExpr, expected: Option<&Type>) -> TypedExpr {
        // An abstract collection holds a value as one of its built-in types; one of a member's
        // kind with items of a derived class is that member as it is (LIB-20, ADR-0086).
        let expected = match expected {
            None => return value,
            Some(Type::Abstract(collection)) => {
                return Self::converted_to_abstract(value, collection);
            }
            Some(expected) => expected,
        };
        let value = match Self::converted_to_object_member(value, expected) {
            Ok(held) => return held,
            Err(value) => value,
        };
        if value.ty != *expected && value.ty.is_upcast_to(expected) {
            let span = value.span;
            return Self::typed(
                TypedExprKind::Upcast(Box::new(value)),
                expected.clone(),
                span,
            );
        }
        match expected {
            // An instance of an iterator class becomes an iterator as it is (TYP-20).
            Type::Iterator(_)
                if matches!(value.ty, Type::Instance(_)) && value.ty.is_assignable_to(expected) =>
            {
                let span = value.span;
                Self::typed(
                    TypedExprKind::AsIterator(Box::new(value)),
                    expected.clone(),
                    span,
                )
            }
            // A collection or an iterator becomes an iterable as it is.
            Type::Iterable(item)
                if !matches!(value.ty, Type::Iterable(_) | Type::Error)
                    && value.ty.is_assignable_to(expected) =>
            {
                // A fixed tuple becomes the `tuple[T, ...]` object an iterable can hold.
                let value = if let Type::Tuple(_) = value.ty {
                    Self::converted(value, Some(&Type::TupleOf(item.clone())))
                } else {
                    value
                };
                let span = value.span;
                Self::typed(
                    TypedExprKind::AsIterable(Box::new(value)),
                    expected.clone(),
                    span,
                )
            }
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

    /// Returns `value` held as the union `expected` through the member that holds it as
    /// another object, an iterable, an iterator or a `tuple[T, ...]`, which takes it as it
    /// would alone: a fixed tuple becomes a `tuple[T, ...]` object first, and a collection or
    /// an iterator an iterable. The union then holds that object. Gives back the value for
    /// any other conversion.
    fn converted_to_object_member(
        value: TypedExpr,
        expected: &Type,
    ) -> Result<TypedExpr, TypedExpr> {
        let Type::Union(members) = expected else {
            return Err(value);
        };
        let member = members.iter().find(|member| {
            value.ty.is_assignable_to(member)
                || matches!(member, Type::TupleOf(item) if value.ty.fits_tuple_of(item))
        });
        match member {
            Some(member @ (Type::Iterable(_) | Type::Iterator(_) | Type::TupleOf(_)))
                if value.ty != *expected && !members.contains(&value.ty) =>
            {
                let value = Self::converted(value, Some(member));
                let span = value.span;
                Ok(Self::typed(
                    TypedExprKind::Upcast(Box::new(value)),
                    expected.clone(),
                    span,
                ))
            }
            _ => Err(value),
        }
    }

    /// Returns `value` held as the abstract collection `collection`: as the member of its
    /// kind whose items, or the part of a dict it reads, the value passes for, then as the
    /// built-in types the collection holds (LIB-20).
    fn converted_to_abstract(
        value: TypedExpr,
        collection: &crate::abstract_type::AbstractType,
    ) -> TypedExpr {
        let value = match collection.member_for(&value.ty) {
            Some(member) if !value.ty.is_assignable_to(&collection.members) => {
                let member = member.clone();
                if matches!(value.ty, Type::Tuple(_)) {
                    Self::converted(value, Some(&member))
                } else {
                    let span = value.span;
                    Self::typed(TypedExprKind::Upcast(Box::new(value)), member, span)
                }
            }
            _ => value,
        };
        Self::converted(value, Some(&collection.members))
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
        if super::expression::has_starred(elements) {
            return self.spread_display(SpreadKind::List, elements, expected, span);
        }
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
                    .with_help(
                        "annotate the variable, such as `items: list[int] = []`; the item type \
                         inferred from its first use is planned for TyPy 0.12 (TYP-27)",
                    ),
                    span,
                );
            }
        };
        let items = converted_all(items, &item_type);
        for item in &items {
            if !item.ty.is_assignable_to(&item_type) {
                let mut diagnostic = Diagnostic::new(
                    DiagnosticCode::error(409),
                    item.span,
                    format!(
                        "the items of a list have one type; this is {}, and the list holds {item_type}",
                        item.ty.with_article()
                    ),
                )
                .with_rule(RuleId::DAT_07);
                if let Some(help) = self.protocol_help(&item.ty, &item_type) {
                    diagnostic = diagnostic.with_help(help);
                }
                self.diagnostics.push(diagnostic);
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
        // A built-in function compared with a function value takes its `Callable` type, as
        // `measure is len` reads it (FUN-08).
        let builtins: Vec<bool> = operands
            .iter()
            .map(|operand| self.names_builtin(operand))
            .collect();
        let mut typed: Vec<Option<TypedExpr>> = operands
            .iter()
            .zip(&kinds)
            .zip(&builtins)
            .map(|((operand, kind), builtin)| {
                (kind.is_none() && !builtin).then(|| self.expression(operand))
            })
            .collect();
        for position in (0..operands.len()).filter(|position| builtins[*position]) {
            let neighbour = [position.checked_sub(1), Some(position + 1)]
                .into_iter()
                .flatten()
                .filter_map(|other| typed.get(other)?.as_ref())
                .find(|other| matches!(other.ty, Type::Callable { .. }))
                .map(|other| other.ty.clone());
            typed[position] =
                Some(self.expression_expecting(operands[position], neighbour.as_ref()));
        }
        for (position, kind) in kinds.iter().enumerate() {
            let Some(kind) = kind else {
                continue;
            };
            let expected = empty_type(*kind, &typed, operators, position);
            typed[position] =
                Some(self.expression_expecting(operands[position], expected.as_ref()));
        }
        // `sys.version_info` compares as the tuple of its items, and by identity with `is`.
        let identity = operators
            .iter()
            .any(|operator| matches!(operator, CompareOperator::Is | CompareOperator::IsNot));
        let mut typed: Vec<TypedExpr> = typed
            .into_iter()
            .flatten()
            .map(|operand| {
                if identity {
                    operand
                } else {
                    Self::version_tuple(operand)
                }
            })
            .collect();
        // A fixed tuple compared with a `tuple[T, ...]`, or looked for among them, becomes one
        // (DAT-14).
        for (position, operator) in operators.iter().enumerate() {
            let (left, right) = (typed[position].ty.clone(), typed[position + 1].ty.clone());
            let membership = matches!(operator, CompareOperator::In | CompareOperator::NotIn);
            let wanted = if membership {
                match &right {
                    Type::Set(item) | Type::Dict(item, _) => Some(item.as_ref().clone()),
                    _ => right.sequence_item().cloned(),
                }
            } else {
                Some(right.clone())
            };
            // An item of a union, or an optional one, is looked for as a value of the item type,
            // which the runtime compares as the member it holds (TYP-14).
            let union_item = |wanted: &Type| {
                membership
                    && matches!(wanted, Type::Union(_) | Type::Optional(_))
                    && left != *wanted
                    && left.is_assignable_to(wanted)
            };
            if let Some(wanted) =
                wanted.filter(|wanted| matches!(wanted, Type::TupleOf(_)) || union_item(wanted))
            {
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

    /// Returns `list[index]` of a checked index, an `int`.
    pub(super) fn list_item_of(
        &mut self,
        list: TypedExpr,
        item: Type,
        index: TypedExpr,
        span: Span,
    ) -> TypedExpr {
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

/// Returns the member of a union a display, a comprehension or a call of `list`, `set` or
/// `dict` builds: the member of its kind, when there is one.
fn display_member(expression: &Expr, called: Option<Builtin>, members: &[Type]) -> Option<Type> {
    let kind = |member: &Type| match (expression, member) {
        (Expr::List { .. } | Expr::ListComp { .. }, Type::List(_))
        | (Expr::Set { .. } | Expr::SetComp { .. }, Type::Set(_))
        | (Expr::Dict { .. } | Expr::DictComp { .. }, Type::Dict(..)) => true,
        (Expr::Call { .. }, Type::List(_)) => called == Some(Builtin::List),
        (Expr::Call { .. }, Type::Set(_)) => called == Some(Builtin::Set),
        (Expr::Call { .. }, Type::Dict(..)) => called == Some(Builtin::Dict),
        _ => false,
    };
    let mut found = members.iter().filter(|member| kind(member));
    let member = found.next()?;
    found.next().is_none().then(|| member.clone())
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
