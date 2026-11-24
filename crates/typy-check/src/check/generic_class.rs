//! Generic classes as the checker collects them: their type parameters, and annotations that
//! name them, each of which stands for a copy (GEN-01, ADR-0028).

use std::cell::RefCell;
use std::collections::BTreeMap;

use typy_diagnostics::{Diagnostic, DiagnosticCode, Diagnostics, RuleId, Span};
use typy_source::syntax::{Expr, Stmt};

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
/// once with them opaque and copied for each copy of the class. A static method that names
/// none of them, in its signature or its body, is the same for every copy, so it is compiled
/// once and called as it is.
pub(super) fn finish_generic_class(
    info: &super::ClassInfo,
    methods: &mut [super::Definition<'_>],
    copies: &RefCell<ClassCopies>,
) {
    let Some(generic) = copies.borrow().roots.get(&info.id).cloned() else {
        return;
    };
    let names: Vec<&str> = generic
        .parameters
        .iter()
        .map(|parameter| parameter.name.as_str())
        .collect();
    for method in methods {
        // A static method that names none of the class's type parameters is the same for every
        // copy: generic over its own, if it has some, and otherwise not generic at all.
        let is_static = !method.takes_instance && info.static_methods.contains(&method.name);
        if is_static && !names_type_parameter(method, &names) {
            continue;
        }
        // A method with type parameters of its own is generic over the class's first, whose
        // arguments a call takes from the copy that holds it (GEN-09).
        let own = std::mem::take(&mut method.type_parameters);
        method.type_parameters.clone_from(&generic.parameters);
        method.type_parameters.extend(own);
    }
}

/// Returns true when a method names one of `names`: its signature holds a type parameter, or
/// an expression or a variable's annotation in its body names one.
fn names_type_parameter(method: &super::Definition<'_>, names: &[&str]) -> bool {
    let signature = &method.signature;
    // A parameter of the method's own is not one of the class's.
    let own: Vec<Span> = method
        .type_parameters
        .iter()
        .map(|parameter| parameter.owner)
        .collect();
    let mut in_signature = false;
    for ty in signature
        .parameters
        .iter()
        .map(|parameter| &parameter.ty)
        .chain(std::iter::once(&signature.returns))
    {
        ty.visit(&mut |each| {
            in_signature |=
                matches!(each, Type::Parameter(parameter) if !own.contains(&parameter.owner));
        });
    }
    if in_signature {
        return true;
    }
    let mut found = false;
    let mut visit = |expression: &Expr| {
        if let Expr::Name { id, .. } = expression {
            found |= names.contains(&id.as_str());
        }
    };
    for statement in method.body {
        super::visit_statement(statement, &mut visit);
    }
    let mut pending: Vec<&Stmt> = method.body.iter().collect();
    while let Some(statement) = pending.pop() {
        if let Stmt::AnnAssign { annotation, .. } = statement {
            found |= annotation_names(annotation, names);
        }
        pending.extend(super::visit::nested_blocks(statement).into_iter().flatten());
    }
    found
}

/// Returns true when an annotation names one of `names`, also inside a string.
pub(super) fn annotation_names(annotation: &Expr, names: &[&str]) -> bool {
    let mut found = false;
    let mut visit = |expression: &Expr| match expression {
        Expr::Name { id, .. } => found |= names.contains(&id.as_str()),
        Expr::Quoted { annotation, .. } => found |= annotation_names(annotation, names),
        _ => {}
    };
    super::visit::visit_expression(annotation, &mut visit);
    found
}
