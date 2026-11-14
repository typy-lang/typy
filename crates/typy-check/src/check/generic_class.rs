//! Generic classes as the checker collects them: their type parameters, and annotations that
//! name them, each of which stands for a copy (GEN-01, ADR-0028).

use std::cell::RefCell;
use std::collections::BTreeMap;

use typy_diagnostics::{Diagnostic, DiagnosticCode, Diagnostics, RuleId, Span};
use typy_source::syntax::Expr;

use super::annotation::{AnnotationScope, annotation_type};
use super::class::ClassStatement;
use crate::body::{ClassCopies, GenericClass};
use crate::class_type::InstanceClass;
use crate::typed::ClassId;
use crate::types::Type;

/// Notes each generic class of the module, with its type parameters, before any annotation
/// may name one; a copy asked for afterwards takes an ID after the module's classes.
pub(super) fn note_generic_classes(
    classes: &[ClassStatement<'_>],
    scope: &AnnotationScope<'_>,
    copies: &RefCell<ClassCopies>,
    diagnostics: &mut Diagnostics,
) {
    for class in classes
        .iter()
        .filter(|class| !class.type_parameters.is_empty())
    {
        let parameters = super::definition::resolve_type_parameters(
            class.type_parameters,
            class.span,
            scope,
            diagnostics,
        );
        let instance = match scope.instances.get(&class.id) {
            Some(Type::Instance(instance)) => instance.as_ref().clone(),
            _ => InstanceClass::root(class.id, &class.name.name),
        };
        copies.borrow_mut().roots.insert(
            class.id,
            GenericClass {
                parameters,
                instance,
            },
        );
    }
}

/// Returns the type `name[index]` names, for the generic class `class`: a copy for the type
/// arguments, after checking each against its parameter's bound or constraints.
pub(super) fn applied_annotation(
    class: ClassId,
    name: &str,
    index: &Expr,
    scope: &AnnotationScope<'_>,
    diagnostics: &mut Diagnostics,
) -> Type {
    let given = match index {
        Expr::Tuple { elements, .. } => elements.iter().collect(),
        single => vec![single],
    };
    let arguments: Vec<Type> = given
        .iter()
        .map(|each| annotation_type(each, scope, diagnostics))
        .collect();
    let parameters = scope.copies.borrow().roots[&class].parameters.clone();
    if arguments.len() != parameters.len() {
        diagnostics.push(
            Diagnostic::new(
                DiagnosticCode::error(408),
                index.span(),
                format!(
                    "`{name}` takes {} type arguments, and {} are given",
                    parameters.len(),
                    arguments.len()
                ),
            )
            .with_rule(RuleId::GEN_01),
        );
        return Type::Error;
    }
    if arguments.iter().any(Type::has_error) {
        return Type::Error;
    }
    for ((argument, parameter), at) in arguments.iter().zip(&parameters).zip(&given) {
        let fits = if parameter.constraints.is_empty() {
            parameter
                .bound
                .as_ref()
                .is_none_or(|bound| argument.has_parameter() || argument.is_assignable_to(bound))
        } else {
            argument.has_parameter()
                || parameter
                    .constraints
                    .iter()
                    .any(|constraint| argument.matches(constraint))
        };
        if !fits {
            diagnostics.push(
                Diagnostic::new(
                    DiagnosticCode::error(408),
                    at.span(),
                    format!(
                        "the type parameter `{}` of `{name}` does not take {}",
                        parameter.name,
                        argument.with_article()
                    ),
                )
                .with_rule(RuleId::GEN_02),
            );
            return Type::Error;
        }
    }
    scope.copies.borrow_mut().applied(class, arguments)
}

/// Returns the type the bare name of a generic class names: its own instances inside its own
/// body, where its parameters are in scope; elsewhere it needs its type arguments.
pub(super) fn bare_annotation(
    class: ClassId,
    name: &str,
    span: Span,
    scope: &AnnotationScope<'_>,
    diagnostics: &mut Diagnostics,
) -> Type {
    let parameters = scope.copies.borrow().roots[&class].parameters.clone();
    let inside = parameters.first().is_some_and(|first| {
        scope
            .parameters
            .values()
            .any(|each| matches!(each, Type::Parameter(given) if given.owner == first.owner))
    });
    if inside {
        let own = parameters.into_iter().map(Type::parameter).collect();
        return scope.copies.borrow_mut().applied(class, own);
    }
    diagnostics.push(
        Diagnostic::new(
            DiagnosticCode::error(408),
            span,
            format!(
                "the generic class `{name}` is named with its type arguments, such as `{name}[int]`"
            ),
        )
        .with_rule(RuleId::GEN_01),
    );
    Type::Error
}

/// Returns the type parameters a generic class's body may name, each as its opaque type; an
/// empty table for any other class.
pub(super) fn parameters_in_scope(
    copies: &RefCell<ClassCopies>,
    class: ClassId,
) -> BTreeMap<String, Type> {
    copies
        .borrow()
        .roots
        .get(&class)
        .map(|generic| {
            generic
                .parameters
                .iter()
                .map(|parameter| (parameter.name.clone(), Type::parameter(parameter.clone())))
                .collect()
        })
        .unwrap_or_default()
}

/// Makes the methods of a generic class generic over its type parameters, so each is checked
/// once with them opaque and copied for each copy of the class, and reports what a generic
/// class does not have yet: class variables and defaults (GEN-08).
pub(super) fn finish_generic_class(
    info: &super::ClassInfo,
    methods: &mut [super::Definition<'_>],
    copies: &RefCell<ClassCopies>,
    span: Span,
    diagnostics: &mut Diagnostics,
) {
    let Some(generic) = copies.borrow().roots.get(&info.id).cloned() else {
        return;
    };
    for method in methods {
        if !method.type_parameters.is_empty() {
            diagnostics.push(
                Diagnostic::new(
                    DiagnosticCode::error(201),
                    method.span,
                    format!(
                        "type parameters of `{}`, a method of a generic class, are planned for \
                         TyPy 0.5 (GEN-09)",
                        method.qualified_name
                    ),
                )
                .with_rule(RuleId::GEN_09),
            );
        }
        method.type_parameters.clone_from(&generic.parameters);
    }
    if !info.class_variables.is_empty() || !info.defaults.is_empty() {
        diagnostics.push(
            Diagnostic::new(
                DiagnosticCode::error(201),
                span,
                format!(
                    "class variables and defaults of a generic class, such as those of `{}`, \
                     are planned for TyPy 0.5 (GEN-08)",
                    info.name
                ),
            )
            .with_rule(RuleId::GEN_08),
        );
    }
}
