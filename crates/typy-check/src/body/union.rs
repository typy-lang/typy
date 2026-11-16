//! Values of a union of objects: their attributes and methods through the class every member
//! derives from (TYP-14, ADR-0027).

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
