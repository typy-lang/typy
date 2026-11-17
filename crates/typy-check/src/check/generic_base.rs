//! Bases that name a generic class: a copy, `Box[int]` or `Box[T]`, which the class derives
//! from, read once the module's generic classes are noted; and the bare name of a generic
//! class, which needs its type arguments (GEN-08, ADR-0040).

use std::collections::{BTreeMap, BTreeSet};

use typy_diagnostics::{Diagnostic, DiagnosticCode, Diagnostics, RuleId};
use typy_source::syntax::Expr;

use super::annotation::{AnnotationScope, annotation_type};
use super::class::{ClassStatement, method_names, unsupported_base};
use crate::class_type::{InstanceClass, Operators};
use crate::typed::ClassId;
use crate::types::Type;

/// Gives each class whose base is a copy of a generic class the type of its instances, and
/// each class derived from one of those, in source order, so a base is final before a class
/// that derives from it. `scope` names the module's classes, with `instances` as the class
/// types gave them; a generic class gets its new type as its copies' source too.
pub(super) fn derive_from_copies(
    classes: &[ClassStatement<'_>],
    scope: &AnnotationScope<'_>,
    instances: &mut BTreeMap<ClassId, Type>,
    diagnostics: &mut Diagnostics,
) {
    let mut changed: BTreeSet<ClassId> = BTreeSet::new();
    for class in classes {
        let Some(base) = class.base else {
            continue;
        };
        let parent = match base {
            Expr::Subscript { value, .. } => {
                if !names_generic_class(value, scope) {
                    diagnostics.push(unsupported_base(base.span()));
                    continue;
                }
                let parameters = super::generic_class::parameters_in_scope(scope.copies, class.id);
                let inner = AnnotationScope {
                    instances,
                    parameters: &parameters,
                    bound: class.id.0,
                    ..*scope
                };
                match annotation_type(base, &inner, diagnostics) {
                    Type::Instance(parent) => parent.as_ref().clone(),
                    _ => continue,
                }
            }
            named if names_generic_class(named, scope) => {
                diagnostics.push(bare_generic_base(named));
                continue;
            }
            named => match base_class(named, scope).filter(|id| changed.contains(id)) {
                Some(id) => match instances.get(&id) {
                    Some(Type::Instance(parent)) => parent.as_ref().clone(),
                    _ => continue,
                },
                None => continue,
            },
        };
        let mut ancestors = vec![(parent.id, parent.name.clone())];
        ancestors.extend(parent.ancestors.iter().cloned());
        let instance = InstanceClass {
            id: class.id,
            name: class.name.name.clone(),
            ancestors,
            operators: Operators::of(method_names(class.body), parent.operators),
            generic: None,
            protocol: false,
        };
        if let Some(generic) = scope.copies.borrow_mut().roots.get_mut(&class.id) {
            generic.instance = instance.clone();
        }
        instances.insert(class.id, Type::instance(instance));
        changed.insert(class.id);
    }
}

/// Returns the class a base names by its name or through its module, if it names one.
fn base_class(base: &Expr, scope: &AnnotationScope<'_>) -> Option<ClassId> {
    match base {
        Expr::Name { id, .. } => scope.classes.get(id).copied(),
        Expr::Attribute {
            value, attribute, ..
        } => match value.as_ref() {
            Expr::Name { id, .. } => scope.modules.get(id)?.get(&attribute.name).copied(),
            _ => None,
        },
        _ => None,
    }
}

/// Returns true when an expression names a generic class, by its name or through its module.
fn names_generic_class(named: &Expr, scope: &AnnotationScope<'_>) -> bool {
    base_class(named, scope).is_some_and(|class| scope.copies.borrow().roots.contains_key(&class))
}

/// Returns the error for a generic class named as a base without its type arguments, whose
/// copy the class would derive from.
fn bare_generic_base(named: &Expr) -> Diagnostic {
    let written = match named {
        Expr::Attribute {
            value, attribute, ..
        } => match value.as_ref() {
            Expr::Name { id, .. } => format!("{id}.{}", attribute.name),
            _ => attribute.name.clone(),
        },
        Expr::Name { id, .. } => id.clone(),
        _ => String::new(),
    };
    Diagnostic::new(
        DiagnosticCode::error(408),
        named.span(),
        format!(
            "the generic class `{written}` is named with its type arguments as a base, such as \
             `{written}[int]`"
        ),
    )
    .with_rule(RuleId::GEN_08)
}
