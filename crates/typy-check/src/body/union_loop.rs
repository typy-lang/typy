//! A loop over a variable, or `args` of an exception, that holds a union of fixed tuples (TYP-14, ADR-0034): each member
//! gives a list of its items, each held as the union of every member's item types, and the
//! loop goes over the list the value's member gives.

use typy_source::syntax::Expr;

use super::BodyChecker;
use crate::typed::{EachMember, TypedExpr, TypedExprKind};
use crate::types::{Type, TypeObjectOf};

impl BodyChecker<'_> {
    /// Returns the items of a variable that holds a union of fixed tuples, as a list of the
    /// union of their types; `None` for any other expression.
    pub(super) fn union_tuple_items(&mut self, expression: &Expr) -> Option<TypedExpr> {
        let (split, members) = self.union_operand(expression)?;
        let shapes = members
            .iter()
            .map(|member| match member {
                Type::Tuple(items) => Some(items.clone()),
                _ => None,
            })
            .collect::<Option<Vec<Vec<Type>>>>()?;
        let item = Type::union(shapes.iter().flatten().cloned().collect());
        let list = Type::List(Box::new(item.clone()));
        let span = expression.span();
        let value = self.expression(expression);
        let before = self.assigned.clone();
        let mut arms = Vec::new();
        for (member, items) in members.into_iter().zip(&shapes) {
            // The variable reads as the member while its items are taken.
            self.splitting.push((split, member));
            let read = self.expression(expression);
            self.splitting.pop();
            self.assigned.clone_from(&before);
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
            arms.push((
                crate::typed::MemberTest::Class(TypeObjectOf::Tuple),
                Self::typed(TypedExprKind::List(elements), list.clone(), span),
            ));
        }
        Some(Self::typed(
            TypedExprKind::EachMember(Box::new(EachMember { value, arms })),
            list,
            span,
        ))
    }
}
