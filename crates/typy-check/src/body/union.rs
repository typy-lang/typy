//! Values of a union of objects: their attributes and methods through the class every member
//! derives from (TYP-14, ADR-0027).

use typy_diagnostics::{Diagnostic, DiagnosticCode, RuleId};

use super::BodyChecker;
use crate::typed::{TypedExpr, TypedExprKind};
use crate::types::Type;

impl BodyChecker<'_> {
    /// Returns a value of a union of instances that share a base class as an instance of that
    /// class, of a union of exceptions as an exception of their nearest common class, and of
    /// a bounded type parameter as its bound, whose attributes and methods every value has;
    /// any other value as it is.
    pub(super) fn through_union_base(&self, value: TypedExpr) -> TypedExpr {
        let ty = if let Some(base) = value.ty.union_base() {
            self.program.instance_type(base.id)
        } else if let Type::Parameter(parameter) = &value.ty
            && let Some(bound) = &parameter.bound
        {
            // A bounded type parameter offers what its bound offers (GEN-02).
            self.through_union_base(Self::typed(
                TypedExprKind::Upcast(Box::new(value.clone())),
                bound.clone(),
                value.span,
            ))
            .ty
        } else if let Some(classes) = exception_members(&value.ty) {
            Type::Exception(crate::exception::common_base(&classes))
        } else {
            return value;
        };
        let span = value.span;
        Self::typed(TypedExprKind::Upcast(Box::new(value)), ty, span)
    }

    /// Reports a truth test of a union that TyPy does not compile yet, and returns true when it
    /// reports one. A union held as its members' base class is tested as that class, so a
    /// member that tests itself otherwise needs the object's class; any other union is tested
    /// by each object's type, which calls `__bool__` but not yet `__len__` (CLS-07, TYP-14).
    pub(super) fn union_truth_planned(&mut self, value: &TypedExpr) -> bool {
        let ty = match &value.ty {
            Type::Optional(inner) => inner.as_ref(),
            other => other,
        };
        let Type::Union(members) = ty else {
            return false;
        };
        let planned = match ty.union_base() {
            Some(base) => {
                self.program.method(base.id, "__bool__").is_none()
                    && self.program.method(base.id, "__len__").is_none()
                    && members.iter().any(|member| {
                        member.class_id().is_some_and(|class| {
                            self.program.method(class, "__bool__").is_some()
                                || self.program.method(class, "__len__").is_some()
                                || self.subclass_decides_truth(class)
                        })
                    })
            }
            None => members.iter().any(|member| {
                member
                    .class_id()
                    .is_some_and(|class| self.length_decides_truth(class))
            }),
        };
        if planned {
            self.diagnostics.push(
                Diagnostic::new(
                    DiagnosticCode::error(201),
                    value.span,
                    format!(
                        "a truth test of {} whose member classes test themselves with \
                         `__len__`, or differently from their base, is planned for TyPy 0.5 \
                         (CLS-24)",
                        value.ty.with_article()
                    ),
                )
                .with_rule(RuleId::CLS_24)
                .with_help("define `__bool__` in each member class"),
            );
        }
        planned
    }

    /// Returns true when the class, or a class derived from it, has `__len__` and no
    /// `__bool__`, so its truth comes from its length.
    fn length_decides_truth(&self, class: crate::typed::ClassId) -> bool {
        let by_length = |id| {
            self.program.method(id, "__bool__").is_none()
                && self.program.method(id, "__len__").is_some()
        };
        by_length(class)
            || self.program.classes.iter().any(|other| {
                by_length(other.id)
                    && matches!(self.program.instance_type(other.id), Type::Instance(derived)
                        if derived.ancestors.iter().any(|(id, _)| *id == class))
            })
    }
}

/// Returns the classes of a union of exceptions.
fn exception_members(ty: &Type) -> Option<Vec<crate::exception::ExceptionClass>> {
    let Type::Union(members) = ty else {
        return None;
    };
    members
        .iter()
        .map(|member| match member {
            Type::Exception(class) => Some(class.clone()),
            _ => None,
        })
        .collect()
}
