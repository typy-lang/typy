//! A loop over a variable, or `args` of an exception, that holds a union of tuples (TYP-14,
//! ADR-0034): each member gives a list of its items, each held as the union of every member's
//! item types, and the loop goes over the list the value's member gives. A `tuple[T, ...]`
//! member, which a call of an exception class with `*` gives `args` (EXP-22), gives its items
//! through a comprehension that converts each.

use typy_diagnostics::Span;
use typy_source::syntax::Expr;

use super::BodyChecker;
use crate::typed::{AssignTarget, EachMember, Generator, LoopSource, TypedExpr, TypedExprKind};
use crate::types::{Type, TypeObjectOf};

/// The items of one member of a union of tuples.
enum Shape {
    /// A tuple of these items.
    Fixed(Vec<Type>),
    /// A `tuple[T, ...]` of items of this type.
    Variadic(Type),
}

impl BodyChecker<'_> {
    /// Returns the items of a variable that holds a union of tuples, as a list of the union
    /// of their types; `None` for any other expression.
    pub(super) fn union_tuple_items(&mut self, expression: &Expr) -> Option<TypedExpr> {
        let (split, members) = self.union_operand(expression)?;
        let shapes = members
            .iter()
            .map(|member| match member {
                Type::Tuple(items) => Some(Shape::Fixed(items.clone())),
                Type::TupleOf(item) => Some(Shape::Variadic(item.as_ref().clone())),
                _ => None,
            })
            .collect::<Option<Vec<Shape>>>()?;
        let item = Type::union(
            shapes
                .iter()
                .flat_map(|shape| match shape {
                    Shape::Fixed(items) => items.clone(),
                    Shape::Variadic(item) => vec![item.clone()],
                })
                .collect(),
        );
        let list = Type::List(Box::new(item.clone()));
        let span = expression.span();
        let value = self.expression(expression);
        let before = self.assigned.clone();
        let mut arms = Vec::new();
        for (member, shape) in members.into_iter().zip(&shapes) {
            // The variable reads as the member while its items are taken.
            self.splitting.push((split, member));
            let read = self.expression(expression);
            self.splitting.pop();
            self.assigned.clone_from(&before);
            let items = match shape {
                Shape::Fixed(items) => {
                    let elements = items
                        .iter()
                        .enumerate()
                        .map(|(index, ty)| {
                            let taken = Self::typed(
                                TypedExprKind::TupleItem {
                                    tuple: Box::new(read.clone()),
                                    index,
                                },
                                ty.clone(),
                                span,
                            );
                            Self::converted(taken, Some(&item))
                        })
                        .collect();
                    Self::typed(TypedExprKind::List(elements), list.clone(), span)
                }
                Shape::Variadic(each) => self.converted_items(read, each, &item, span),
            };
            arms.push((crate::typed::MemberTest::Class(TypeObjectOf::Tuple), items));
        }
        Some(Self::typed(
            TypedExprKind::EachMember(Box::new(EachMember { value, arms })),
            list,
            span,
        ))
    }

    /// Returns `[each for each in tuple]` of the `tuple[each, ...]` `tuple`, each item
    /// converted to `item`.
    pub(super) fn converted_items(
        &mut self,
        tuple: TypedExpr,
        each: &Type,
        item: &Type,
        span: Span,
    ) -> TypedExpr {
        let symbol = self.hidden_variable("item", each.clone(), span);
        let element = Self::typed(TypedExprKind::Local(symbol), each.clone(), span);
        Self::typed(
            TypedExprKind::Comprehension {
                element: Box::new(Self::converted(element, Some(item))),
                value: None,
                generators: vec![Generator {
                    target: AssignTarget::Name(symbol),
                    source: LoopSource::Items(tuple),
                    conditions: Vec::new(),
                }],
            },
            Type::List(Box::new(item.clone())),
            span,
        )
    }
}
