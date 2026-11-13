//! Operations on a variable that holds a union of objects, checked once for each member with
//! the variable narrowed to it; the member the object is runs (TYP-14, ADR-0022).

use typy_diagnostics::{Diagnostic, DiagnosticCode, RuleId};
use typy_source::syntax::{CompareOperator, Expr, UnaryOperator};

use super::BodyChecker;
use crate::builtin::Builtin;
use crate::typed::{EachMember, SymbolId, TypedExpr, TypedExprKind};
use crate::types::{Type, TypeObjectOf};

impl BodyChecker<'_> {
    /// Returns `expression` checked for each member of the union its operand holds, when the
    /// operation needs the member: an attribute, a method, an operator, an item, or a builtin
    /// other than those that take any object. `None` leaves the expression to its own check.
    pub(super) fn each_member(&mut self, expression: &Expr) -> Option<TypedExpr> {
        let operand = self.member_operand(expression)?;
        let (symbol, members) = self.union_operand(operand)?;
        let span = expression.span();
        let Some(tests) = members.iter().map(member_test).collect::<Option<Vec<_>>>() else {
            return Some(
                self.error(
                    Diagnostic::new(
                        DiagnosticCode::error(201),
                        span,
                        format!(
                            "an operation on {} whose member has no class to test, such as an \
                         iterator, is planned for TyPy 0.5 (TYP-14)",
                            Type::Union(members).with_article()
                        ),
                    )
                    .with_rule(RuleId::TYP_14),
                    span,
                ),
            );
        };
        let value = self.expression(operand);
        let before = self.assigned.clone();
        let mut arms = Vec::new();
        for (member, test) in members.into_iter().zip(tests) {
            // The variable reads as the member while the operation is checked for it, also in
            // code no path reaches, whose flow state holds nothing.
            self.splitting.push((symbol, member));
            let body = self.expression(expression);
            self.splitting.pop();
            self.assigned.clone_from(&before);
            arms.push((test, body));
        }
        let ty = self.each_member_type(&arms, span);
        let arms = arms
            .into_iter()
            .map(|(test, body)| (test, Self::converted(body, Some(&ty))))
            .collect();
        Some(Self::typed(
            TypedExprKind::EachMember(Box::new(EachMember { value, arms })),
            ty,
            span,
        ))
    }

    /// Returns the type of the result: the members' results joined into one type, which is
    /// held as each of them when they are objects, or one type they all give.
    fn each_member_type(
        &mut self,
        arms: &[(TypeObjectOf, TypedExpr)],
        span: typy_diagnostics::Span,
    ) -> Type {
        let results: Vec<Type> = arms.iter().map(|(_, body)| body.ty.clone()).collect();
        if results.iter().any(Type::has_error) {
            return Type::Error;
        }
        let joined = Type::union(results);
        let held = match &joined {
            Type::Optional(inner) => inner.members().iter().all(Type::is_object),
            other => other.members().len() == 1 || other.members().iter().all(Type::is_object),
        };
        if held {
            return joined;
        }
        self.diagnostics.push(
            Diagnostic::new(
                DiagnosticCode::error(201),
                span,
                format!(
                    "an operation whose result is {} for the members of a union, held by \
                     value, is planned for TyPy 0.4 (TYP-14)",
                    joined.with_article()
                ),
            )
            .with_rule(RuleId::TYP_14),
        );
        Type::Error
    }

    /// Returns the operand whose member an operation needs, if the expression is one.
    fn member_operand<'expression>(
        &self,
        expression: &'expression Expr,
    ) -> Option<&'expression Expr> {
        let is_union = |operand: &Expr| self.union_operand(operand).is_some();
        let first =
            |operands: &[&'expression Expr]| operands.iter().copied().find(|each| is_union(each));
        match expression {
            Expr::Attribute { value, .. } | Expr::Subscript { value, .. } => first(&[value]),
            Expr::Call {
                function,
                arguments,
                ..
            } => match function.as_ref() {
                Expr::Attribute { value, .. } => first(&[value]),
                Expr::Name { .. } => {
                    let (builtin, ..) = self.called_builtin(expression)?;
                    if takes_any_object(builtin) {
                        return None;
                    }
                    arguments.iter().find(|each| is_union(each))
                }
                _ => None,
            },
            Expr::BinOp { left, right, .. } => first(&[left, right]),
            Expr::UnaryOp {
                operator, operand, ..
            } if *operator != UnaryOperator::Not => first(&[operand]),
            Expr::Compare {
                left,
                operators,
                comparators,
                ..
            } => match (operators.as_slice(), comparators.as_slice()) {
                ([CompareOperator::In | CompareOperator::NotIn], [container]) => {
                    first(&[container])
                }
                ([CompareOperator::Is | CompareOperator::IsNot], _) => None,
                ([_], [right]) => first(&[left, right]),
                _ => None,
            },
            _ => None,
        }
    }

    /// Returns the variable an operand reads, with the members of the union it holds, when it
    /// is a variable that `union_variable` splits on.
    fn union_operand(&self, operand: &Expr) -> Option<(SymbolId, Vec<Type>)> {
        match operand {
            Expr::Name { id, .. } => self.union_variable(id),
            _ => None,
        }
    }

    /// Returns the variable `id` with the members of the union it holds here, when it holds
    /// one whose members share no class, and no operation is already split on it. A variable
    /// a function declares `global` is not narrowed, so it is not split either.
    fn union_variable(&self, id: &str) -> Option<(SymbolId, Vec<Type>)> {
        let symbol = *self.locals.get(id)?;
        let split = self.splitting.iter().any(|(each, _)| *each == symbol);
        if split || self.program.assigned_in_functions.contains(id) {
            return None;
        }
        let ty = self
            .narrowed_type(symbol, id)
            .or_else(|| self.types.get(&symbol).cloned())?;
        let Type::Union(members) = &ty else {
            return None;
        };
        let exceptions = members
            .iter()
            .all(|member| matches!(member, Type::Exception(_)));
        if ty.union_base().is_some() || exceptions {
            return None;
        }
        Some((symbol, members.clone()))
    }
}

/// Returns true for a builtin that takes any object, as a union is.
const fn takes_any_object(builtin: Builtin) -> bool {
    matches!(
        builtin,
        Builtin::Print
            | Builtin::Str
            | Builtin::Repr
            | Builtin::Type
            | Builtin::IsInstance
            | Builtin::Bool
    )
}

/// Returns the class the runtime tests an object against to find it is of `member`, when
/// the member has one.
fn member_test(member: &Type) -> Option<TypeObjectOf> {
    match member {
        Type::Exception(class) => Some(TypeObjectOf::Exception(class.clone())),
        Type::Iterator(_)
        | Type::Iterable(_)
        | Type::Module(_)
        | Type::AnyModule
        | Type::TextFile
        | Type::Traceback => None,
        other => TypeObjectOf::of(other),
    }
}
