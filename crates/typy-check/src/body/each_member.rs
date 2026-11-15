//! Operations on a variable that holds a union of objects, checked once for each member with
//! the variable narrowed to it; the member the object is runs (TYP-14, ADR-0027).

use typy_diagnostics::{Diagnostic, DiagnosticCode, RuleId, Span};
use typy_source::syntax::{CompareOperator, Constant, Expr, UnaryOperator};

use super::BodyChecker;
use super::expression::literal_position;
use crate::builtin::Builtin;
use crate::exception::{BuiltinException, ExceptionClass};
use crate::typed::{EachMember, SymbolId, TypedConstant, TypedExpr, TypedExprKind};
use crate::types::{Type, TypeObjectOf};

impl BodyChecker<'_> {
    /// Returns `expression` checked for each member of the union its operand holds, when the
    /// operation needs the member: an attribute, a method, an operator, an item, or a builtin
    /// other than those that take any object. `None` leaves the expression to its own check.
    pub(super) fn each_member(&mut self, expression: &Expr) -> Option<TypedExpr> {
        let operand = self.member_operand(expression)?;
        let (symbol, members) = self.union_operand(operand)?;
        let span = expression.span();
        let Some(tests) = members.iter().map(member_class).collect::<Option<Vec<_>>>() else {
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
        let unequal = self.unequal_answer(expression, operand, &members);
        let short = short_members(expression, &members);
        if let Expr::Subscript { index, .. } = expression
            && short.iter().all(|short| *short)
        {
            return Some(
                self.error(
                    Diagnostic::new(
                        DiagnosticCode::error(307),
                        index.span(),
                        format!(
                            "position {} is outside each member of {}",
                            literal_position(index).unwrap_or_default(),
                            Type::Union(members)
                        ),
                    )
                    .with_rule(RuleId::DAT_08),
                    span,
                ),
            );
        }
        let value = self.expression(operand);
        let before = self.assigned.clone();
        let mut arms = Vec::new();
        for ((member, test), short) in members.into_iter().zip(tests).zip(short) {
            if short {
                arms.push((test, Self::index_error(span)));
                continue;
            }
            if let Some((other, answer)) = &unequal
                && !member.can_test_equality(other)
            {
                let constant = TypedExprKind::Constant(TypedConstant::Bool(*answer));
                arms.push((test, Self::typed(constant, Type::Bool, span)));
                continue;
            }
            // The variable reads as the member while the operation is checked for it, also in
            // code no path reaches, whose flow state holds nothing.
            self.splitting.push((symbol, member));
            let body = self.expression(expression);
            self.splitting.pop();
            self.assigned.clone_from(&before);
            arms.push((test, body));
        }
        let ty = Self::each_member_type(&arms);
        let arms = arms
            .into_iter()
            .map(|(test, mut body)| {
                if matches!(body.kind, TypedExprKind::Raise(_)) {
                    body.ty = ty.clone();
                    (test, body)
                } else {
                    (test, Self::converted(body, Some(&ty)))
                }
            })
            .collect();
        Some(Self::typed(
            TypedExprKind::EachMember(Box::new(EachMember { value, arms })),
            ty,
            span,
        ))
    }

    /// Returns, for `value == other` or `value != other` whose other operand is a constant or
    /// a variable, that operand's type and what the comparison gives for a member it cannot
    /// equal, as CPython's default equality does: `False` for `==` and `True` for `!=`. The
    /// other operand has no effect to skip. `None` for any other expression, or when no member
    /// can equal it, which the comparison reports as it reports two such types.
    fn unequal_answer(
        &self,
        expression: &Expr,
        operand: &Expr,
        members: &[Type],
    ) -> Option<(Type, bool)> {
        let Expr::Compare {
            left,
            operators,
            comparators,
            ..
        } = expression
        else {
            return None;
        };
        let ([operator], [right]) = (operators.as_slice(), comparators.as_slice()) else {
            return None;
        };
        let answer = match operator {
            CompareOperator::Eq => false,
            CompareOperator::NotEq => true,
            _ => return None,
        };
        let other = if std::ptr::eq(left.as_ref(), operand) {
            right
        } else {
            left.as_ref()
        };
        let ty = match other {
            Expr::Constant { value, .. } => constant_type(value)?,
            // A variable split on before has the member it is checked for now.
            Expr::Name { id, .. } => match self.locals.get(id) {
                Some(symbol) => self
                    .narrowed_type(*symbol, id)
                    .or_else(|| self.peek_type(other))?,
                None => self.peek_type(other)?,
            },
            _ => return None,
        };
        members
            .iter()
            .any(|member| member.can_test_equality(&ty))
            .then_some((ty, answer))
    }

    /// Returns `IndexError: tuple index out of range`, raised by the item of a tuple member
    /// too short for the position, as CPython raises it only for that member.
    fn index_error(span: Span) -> TypedExpr {
        let class = ExceptionClass::Builtin(BuiltinException::IndexError);
        let message = TypedConstant::Str("tuple index out of range".to_owned());
        let exception = Self::typed(
            TypedExprKind::NewException {
                class: class.clone(),
                arguments: vec![Self::typed(
                    TypedExprKind::Constant(message),
                    Type::Str,
                    span,
                )],
                init: None,
            },
            Type::Exception(class),
            span,
        );
        // The type is the result's, which `each_member` gives once the other arms are known.
        Self::typed(
            TypedExprKind::Raise(Box::new(exception)),
            Type::NoneType,
            span,
        )
    }

    /// Returns the type of the result: the members' results joined into one type. An arm
    /// that raises gives no result.
    fn each_member_type(arms: &[(TypeObjectOf, TypedExpr)]) -> Type {
        let results: Vec<Type> = arms
            .iter()
            .filter(|(_, body)| !matches!(body.kind, TypedExprKind::Raise(_)))
            .map(|(_, body)| body.ty.clone())
            .collect();
        if results.iter().any(Type::has_error) {
            return Type::Error;
        }
        Type::union(results)
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
                    let found = arguments.iter().find(|each| is_union(each))?;
                    // A builtin that reads an iterable goes over a union of fixed tuples as the
                    // list of its member's items (ADR-0034).
                    if reads_iterable(builtin) && self.holds_fixed_tuples(found) {
                        return None;
                    }
                    Some(found)
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

    /// Returns true when the operand is a variable that holds a union of fixed tuples.
    fn holds_fixed_tuples(&self, operand: &Expr) -> bool {
        self.union_operand(operand).is_some_and(|(_, members)| {
            members
                .iter()
                .all(|member| matches!(member, Type::Tuple(_)))
        })
    }

    /// Returns the variable an operand reads, with the members of the union it holds, when it
    /// is a variable that `union_variable` splits on.
    pub(super) fn union_operand(&self, operand: &Expr) -> Option<(SymbolId, Vec<Type>)> {
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

/// Returns, for each member, whether `expression` is `variable[position]` with a literal
/// position outside the member, a tuple too short for it.
fn short_members(expression: &Expr, members: &[Type]) -> Vec<bool> {
    let position = match expression {
        Expr::Subscript { index, .. } => literal_position(index),
        _ => None,
    };
    members
        .iter()
        .map(|member| match (member, position) {
            (Type::Tuple(items), Some(position)) => {
                let length = i128::try_from(items.len()).unwrap_or(i128::MAX);
                !(-length..length).contains(&position)
            }
            _ => false,
        })
        .collect()
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
/// the member has one (TYP-14).
#[must_use]
pub fn member_class(member: &Type) -> Option<TypeObjectOf> {
    match member {
        Type::Exception(class) => Some(TypeObjectOf::Exception(class.clone())),
        Type::Iterator(_)
        | Type::Iterable(_)
        | Type::Module(_)
        | Type::TypeAlias
        | Type::VersionInfo
        | Type::AnyModule
        | Type::TextFile
        | Type::BinaryFile(_)
        | Type::DescriptorFile(_)
        | Type::Traceback => None,
        other => TypeObjectOf::of(other),
    }
}

/// Returns the type of a constant, when it is one TyPy compiles.
fn constant_type(value: &Constant) -> Option<Type> {
    Some(match value {
        Constant::None => Type::NoneType,
        Constant::Bool(_) => Type::Bool,
        Constant::Int(_) => Type::Int,
        Constant::Float(_) => Type::Float,
        Constant::Str(_) | Constant::StrPoints(_) => Type::Str,
        Constant::Bytes(_) => Type::Bytes,
        Constant::Complex { .. } | Constant::Ellipsis => return None,
    })
}

/// Returns true for a builtin that reads its argument as an iterable.
const fn reads_iterable(builtin: Builtin) -> bool {
    matches!(
        builtin,
        Builtin::List
            | Builtin::Tuple
            | Builtin::Set
            | Builtin::Sorted
            | Builtin::Sum
            | Builtin::Any
            | Builtin::All
            | Builtin::Min
            | Builtin::Max
            | Builtin::Enumerate
            | Builtin::Zip
    )
}
