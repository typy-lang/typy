//! `typing.cast(T, value)`, checked when the program runs (TYP-24, ADR-0091): a value the
//! checker proves is a `T` is given as one, and any other is tested as `isinstance` tests it
//! and raises `TypeError` when it is not one. TyPy reads a value by the layout its type gives,
//! so it cannot trust a cast as CPython does.

use typy_diagnostics::{Diagnostic, DiagnosticCode, RuleId, Span};
use typy_source::syntax::{Expr, Keyword};

use super::BodyChecker;
use crate::typed::{CastCheck, IsInstanceTest, MemberTest, TypedExpr, TypedExprKind};
use crate::types::{Type, TypeObjectOf};

impl BodyChecker<'_> {
    /// Returns `cast(target, value)`, or an error for a call of another shape or a type no
    /// test decides.
    pub(super) fn cast_call(
        &mut self,
        arguments: &[Expr],
        keywords: &[Keyword],
        span: Span,
    ) -> TypedExpr {
        let [annotation, value] = arguments else {
            for argument in arguments {
                self.expression(argument);
            }
            return self.cast_error(
                span,
                "`cast` takes a type and a value, by position".to_owned(),
            );
        };
        if let Some(keyword) = keywords.first() {
            return self.cast_error(
                keyword.span,
                "`cast` takes its type and its value by position".to_owned(),
            );
        }
        let target = self.annotation(annotation);
        let value = self.expression(value);
        if target.has_error() || value.ty.has_error() {
            return Self::typed(
                TypedExprKind::Constant(crate::typed::TypedConstant::None),
                Type::Error,
                span,
            );
        }
        // A value the checker proves is a `target` is given as one, with no test.
        if value.ty.is_assignable_to(&target) {
            return Self::converted(value, Some(&target));
        }
        let Some(checks) = member_checks(&target) else {
            return self.cast_error(
                annotation.span(),
                format!(
                    "a cast to {target} cannot be checked when the program runs, since a test \
                     reads only a value's class, not the types of its items"
                ),
            );
        };
        if !target.is_assignable_to(&value.ty) && !may_hold(&value.ty, &target) {
            return self.cast_error(
                span,
                format!(
                    "{} is never {}, so this cast always raises `TypeError` in TyPy",
                    value.ty.with_article(),
                    target.with_article()
                ),
            );
        }
        // A union, also inside an optional, is tested member by member.
        let inner = match &value.ty {
            Type::Optional(inner) => inner.as_ref().clone(),
            other => other.clone(),
        };
        let members = match &inner {
            Type::Union(members) => members
                .iter()
                .map(|member| member_outcome(member, &target, &checks))
                .collect(),
            _ => Vec::new(),
        };
        let test = IsInstanceTest {
            none_matches: target_holds_none(&target),
            always: false,
            members,
            checks,
            narrowed: None,
            otherwise: None,
            held: Vec::new(),
            value,
        };
        Self::typed(
            TypedExprKind::Cast(Box::new(CastCheck {
                test,
                target: target.to_string(),
            })),
            target,
            span,
        )
    }

    fn cast_error(&mut self, span: Span, message: String) -> TypedExpr {
        self.error(
            Diagnostic::new(DiagnosticCode::error(404), span, message).with_rule(RuleId::TYP_24),
            span,
        )
    }
}

/// Returns the tests that find a value of `target` among objects: the class of each member,
/// `int` itself rather than any instance of it, since a `bool` is no `int` in TyPy (TYP-03);
/// `None` for a member no test decides, such as `list[int]`.
fn member_checks(target: &Type) -> Option<Vec<MemberTest>> {
    let members: Vec<Type> = match target {
        Type::Optional(inner) => inner.members().to_vec(),
        other => other.members().to_vec(),
    };
    members
        .iter()
        .filter(|member| **member != Type::NoneType)
        .map(|member| match member {
            Type::Int => Some(MemberTest::Exact(TypeObjectOf::Int)),
            Type::Float
            | Type::Bool
            | Type::Str
            | Type::Bytes
            | Type::Range
            | Type::Slice
            | Type::Instance(_)
            | Type::Exception(_)
            | Type::Object => crate::body::each_member::member_test(member),
            _ => None,
        })
        .collect()
}

/// Returns true when `target` takes `None`.
fn target_holds_none(target: &Type) -> bool {
    matches!(target, Type::Optional(_) | Type::NoneType | Type::Object)
}

/// Returns true when a value of `source` may be a `target`: an `object`, an instance of a
/// class `target` derives from, or a union with a member that may be.
fn may_hold(source: &Type, target: &Type) -> bool {
    match source {
        Type::Object => true,
        Type::Optional(inner) => may_hold(inner, target),
        Type::Union(members) => members.iter().any(|member| may_hold(member, target)),
        Type::Instance(base) => target
            .members()
            .iter()
            .any(|member| matches!(member, Type::Instance(class) if class.derives_from(base))),
        Type::Exception(base) => target
            .members()
            .iter()
            .any(|member| matches!(member, Type::Exception(class) if class.derives_from(base))),
        member => member.is_assignable_to(target),
    }
}

/// Returns, for a member of the value's union, whether every value of it is a `target`, and
/// the tests that decide it otherwise: none for a member no value of which can be one.
fn member_outcome(member: &Type, target: &Type, checks: &[MemberTest]) -> (bool, Vec<MemberTest>) {
    if member.is_assignable_to(target) {
        return (true, Vec::new());
    }
    if may_hold(member, target) {
        return (false, checks.to_vec());
    }
    (false, Vec::new())
}
