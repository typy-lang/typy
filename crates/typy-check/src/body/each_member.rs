//! Operations on a variable that holds a union of objects, checked once for each member with
//! the variable narrowed to it; the member the object is runs (TYP-14, ADR-0027).

use typy_diagnostics::{Diagnostic, DiagnosticCode, RuleId, Span};
use typy_source::syntax::{CompareOperator, Constant, Expr, UnaryOperator};

use super::BodyChecker;
use super::expression::literal_position;
use crate::builtin::Builtin;
use crate::exception::{BuiltinException, ExceptionClass};
use crate::typed::{
    EachMember, MemberTest, RuntimeKind, SymbolId, TypedConstant, TypedExpr, TypedExprKind,
};
use crate::types::{Type, TypeObjectOf};

impl BodyChecker<'_> {
    /// Returns `expression` checked for each member of the union its operand holds, when the
    /// operation needs the member: an attribute, a method, an operator, an item, or a builtin
    /// other than those that take any object. `None` leaves the expression to its own check.
    pub(super) fn each_member(&mut self, expression: &Expr) -> Option<TypedExpr> {
        let operand = self.member_operand(expression)?;
        let (split, members) = self.union_operand(operand)?;
        let span = expression.span();
        // The runtime tells the members apart by their classes, or by the kind of the
        // runtime's own objects; members it cannot tell apart are planned (TYP-19).
        let tests: Option<Vec<MemberTest>> = members.iter().map(member_test).collect();
        let Some(tests) = tests.filter(|tests| {
            let kinds: Vec<RuntimeKind> = tests
                .iter()
                .filter_map(|test| match test {
                    MemberTest::Kind(kind) => Some(*kind),
                    MemberTest::Class(_) | MemberTest::Exact(_) => None,
                })
                .collect();
            kinds
                .iter()
                .enumerate()
                .all(|(position, kind)| !kinds[position + 1..].contains(kind))
        }) else {
            return Some(
                self.error(
                    Diagnostic::new(
                        DiagnosticCode::error(201),
                        span,
                        format!(
                            "an operation on {} whose members the program cannot tell apart, \
                             such as two iterators or an iterable, is planned for TyPy 1.0 \
                             (TYP-19)",
                            Type::Union(members).with_article()
                        ),
                    )
                    .with_rule(RuleId::TYP_19),
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
            self.splitting.push((split, member));
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
    fn each_member_type(arms: &[(MemberTest, TypedExpr)]) -> Type {
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

    /// Returns what an operand reads, with the members of the union it holds, when it is a
    /// variable that `union_variable` splits on, or `args` of the exception a variable holds,
    /// which `union_arguments` splits on.
    pub(super) fn union_operand(&self, operand: &Expr) -> Option<(Split, Vec<Type>)> {
        match operand {
            Expr::Name { id, .. } => self.union_variable(id),
            Expr::Attribute { .. } => self.union_arguments(operand),
            _ => None,
        }
    }

    /// Returns the variable `id` with the members of the union it holds here, when it holds
    /// one whose members share no class, and no operation is already split on it. A variable
    /// a call may assign is not narrowed, so it is not split either.
    fn union_variable(&self, id: &str) -> Option<(Split, Vec<Type>)> {
        let symbol = *self.locals.get(id)?;
        let split = self
            .splitting
            .iter()
            .any(|(each, _)| *each == Split::variable(symbol));
        if split || self.changed_by_calls(id) {
            return None;
        }
        let ty = self
            .narrowed_type(symbol, id)
            .or_else(|| self.types.get(&symbol).cloned())?;
        // A variable narrowed to literal types holds values of their base types, so the
        // members are those bases: `Literal[0, 1]` is one `int`, with nothing to split on.
        let ty = ty.without_literal();
        let Type::Union(members) = &ty else {
            return None;
        };
        let exceptions = members
            .iter()
            .all(|member| matches!(member, Type::Exception(_)));
        if ty.union_base().is_some() || exceptions {
            return None;
        }
        Some((Split::variable(symbol), members.clone()))
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

/// What an operation checked for each member splits on: a variable, or `args` of the
/// exception a variable holds.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct Split {
    pub(super) symbol: SymbolId,
    pub(super) arguments: bool,
}

impl Split {
    /// Returns the split on a variable.
    pub(super) const fn variable(symbol: SymbolId) -> Self {
        Self {
            symbol,
            arguments: false,
        }
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

/// Returns how the runtime finds an object is of `member`: by its class, or by the kind of
/// the runtime's own objects; `None` for an iterable, which any object may be (TYP-14).
#[must_use]
pub fn member_test(member: &Type) -> Option<MemberTest> {
    Some(match member {
        Type::Iterator(_) => MemberTest::Kind(RuntimeKind::Iterator),
        Type::Traceback => MemberTest::Kind(RuntimeKind::Traceback),
        Type::Module(_) | Type::AnyModule => MemberTest::Kind(RuntimeKind::Module),
        Type::TextFile | Type::BinaryFile(_) | Type::DescriptorFile(_) => {
            MemberTest::Kind(RuntimeKind::File)
        }
        Type::TypeAlias(_) => MemberTest::Kind(RuntimeKind::TypeAlias),
        Type::VersionInfo => MemberTest::Kind(RuntimeKind::VersionInfo),
        Type::NotImplemented => MemberTest::Kind(RuntimeKind::NotImplemented),
        other => MemberTest::Class(member_class(other)?),
    })
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
        | Type::TypeAlias(_)
        | Type::VersionInfo
        | Type::NotImplemented
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
