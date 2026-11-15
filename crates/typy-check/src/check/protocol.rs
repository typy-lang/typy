//! Protocols in signatures: a parameter whose annotation names a protocol takes a type
//! parameter of its own, bounded by the protocol, so the function compiles to one copy for
//! each class it is called with (GEN-04, ADR-0023). A protocol anywhere else in a signature
//! is planned (GEN-10).

use typy_diagnostics::{Diagnostic, DiagnosticCode, Diagnostics, RuleId, Span};

use super::Definition;
use super::class::ClassInfo;
use super::definition::{FunctionParts, MethodKind};
use crate::class_type::InstanceClass;
use crate::generic::TypeParameter;
use crate::types::Type;

/// Gives each parameter of `definition` that names a protocol a type parameter bounded by
/// it, and reports a protocol in the return type or in a protocol's own method.
pub(super) fn protocol_parameters(definition: &mut Definition<'_>, diagnostics: &mut Diagnostics) {
    let in_protocol = definition.takes_instance
        && definition
            .signature
            .parameters
            .first()
            .is_some_and(|instance| !protocols(&instance.ty).is_empty());
    let first = usize::from(definition.takes_instance);
    for position in first..definition.signature.parameters.len() {
        let ty = definition.signature.parameters[position].ty.clone();
        for protocol in protocols(&ty) {
            // A refused type is an error, so it causes no further report.
            if in_protocol {
                diagnostics.push(elsewhere(definition.span, &protocol.name));
                definition.signature.parameters[position].ty = Type::Error;
                break;
            }
            let parameter = TypeParameter {
                owner: definition.span,
                index: definition.type_parameters.len(),
                // A name no source can write, so no annotation in the body names it.
                name: format!(
                    "{} of {}",
                    protocol.name, definition.signature.parameters[position].name
                ),
                bound: Some(Type::instance(protocol.clone())),
                constraints: Vec::new(),
            };
            let opaque = Type::parameter(parameter.clone());
            let held = &mut definition.signature.parameters[position].ty;
            *held = held.substituted_with(definition.span, &[], &mut |class| {
                (class.id == protocol.id).then(|| opaque.clone())
            });
            definition.type_parameters.push(parameter);
        }
    }
    if let Some(protocol) = protocols(&definition.signature.returns).first() {
        diagnostics.push(elsewhere(definition.span, &protocol.name));
        definition.signature.returns = Type::Error;
    }
}

/// Returns the protocols a type names, each once.
pub(crate) fn protocols(ty: &Type) -> Vec<InstanceClass> {
    let mut found: Vec<InstanceClass> = Vec::new();
    ty.visit(&mut |each| {
        if let Type::Instance(class) = each
            && class.protocol
            && found.iter().all(|known| known.id != class.id)
        {
            found.push(class.as_ref().clone());
        }
    });
    found
}

/// Returns the type an annotation outside a signature gives, once it is known to name no
/// protocol; one that names a protocol is reported, and is an error (GEN-10).
pub(crate) fn outside_signature(ty: Type, span: Span, diagnostics: &mut Diagnostics) -> Type {
    match protocols(&ty).first() {
        Some(protocol) => {
            diagnostics.push(elsewhere(span, &protocol.name));
            Type::Error
        }
        None => ty,
    }
}

/// Reports a protocol as a type somewhere other than a parameter (GEN-10).
fn elsewhere(span: Span, protocol: &str) -> Diagnostic {
    Diagnostic::new(
        DiagnosticCode::error(201),
        span,
        format!(
            "the protocol `{protocol}` as a type other than a parameter's is planned for TyPy \
             0.5 (GEN-10)"
        ),
    )
    .with_rule(RuleId::GEN_10)
    .with_help(format!(
        "a parameter may name `{protocol}`, and stands for each class that satisfies it"
    ))
}

/// Returns the error for a property, a static method or a class method of a protocol, which
/// TyPy 0.5 accepts (GEN-10).
pub(super) fn refused_member(
    instance: &Type,
    kind: &MethodKind,
    parts: &FunctionParts<'_>,
) -> Option<Diagnostic> {
    (*kind != MethodKind::Instance && is_protocol(instance)).then(|| {
        planned_member(
            parts.name.span,
            "properties, static methods and class methods",
        )
    })
}

/// Reports class variables and attribute defaults of a protocol, which TyPy 0.5 accepts
/// (GEN-10).
pub(super) fn check_values(
    instance: &Type,
    info: &ClassInfo,
    span: Span,
    diagnostics: &mut Diagnostics,
) {
    if is_protocol(instance) && (!info.class_variables.is_empty() || !info.defaults.is_empty()) {
        diagnostics.push(planned_member(span, "class variables and defaults"));
    }
}

/// Returns true when a class's instance type is that of a protocol.
fn is_protocol(instance: &Type) -> bool {
    matches!(instance, Type::Instance(class) if class.protocol)
}

/// Reports what a protocol's body holds that TyPy 0.5 accepts: a member other than an
/// instance method or an attribute declared by its annotation (GEN-10).
fn planned_member(span: Span, what: &str) -> Diagnostic {
    Diagnostic::new(
        DiagnosticCode::error(201),
        span,
        format!("{what} in a protocol are planned for TyPy 0.5 (GEN-10)"),
    )
    .with_rule(RuleId::GEN_10)
}
