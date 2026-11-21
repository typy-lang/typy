//! Protocols in signatures: a parameter whose annotation names a protocol takes a type
//! parameter of its own, bounded by the protocol, so the function compiles to one copy for
//! each class it is called with (GEN-04, ADR-0023). Anywhere else, and in a parameter of a
//! method that cannot be copied, a protocol is the type of a value that holds an instance of a
//! class that satisfies it (GEN-10, ADR-0041).

use typy_diagnostics::{Diagnostic, DiagnosticCode, Diagnostics, RuleId, Span};

use super::Definition;
use super::class::ClassInfo;
use super::definition::{FunctionParts, MethodKind};
use crate::class_type::InstanceClass;
use crate::generic::TypeParameter;
use crate::types::Type;

/// Gives each parameter of `definition` that names a protocol a type parameter bounded by
/// it, unless `as_values`, for a method with one body, whose parameters then take protocol
/// values; and reports a protocol in the signature of a protocol's own method (GEN-17).
pub(super) fn protocol_parameters(
    definition: &mut Definition<'_>,
    as_values: bool,
    diagnostics: &mut Diagnostics,
) {
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
                diagnostics.push(in_own_method(definition.span, &protocol.name));
                definition.signature.parameters[position].ty = Type::Error;
                break;
            }
            if as_values {
                continue;
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
    if in_protocol && let Some(protocol) = protocols(&definition.signature.returns).first() {
        diagnostics.push(in_own_method(definition.span, &protocol.name));
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

/// Reports a protocol in the signature of a protocol's own method, which a class's method
/// would have to take or give as protocol values to satisfy it (GEN-17).
fn in_own_method(span: Span, protocol: &str) -> Diagnostic {
    Diagnostic::new(
        DiagnosticCode::error(201),
        span,
        format!(
            "the protocol `{protocol}` in the signature of a protocol's method is planned for \
             TyPy 0.13 (GEN-17)"
        ),
    )
    .with_rule(RuleId::GEN_17)
}

/// Returns the error for a property, a static method or a class method of a protocol, which
/// TyPy 1.0 accepts (GEN-17).
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

/// Reports class variables and attribute defaults of a protocol, which TyPy 1.0 accepts
/// (GEN-17).
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

/// Reports what a protocol's body holds that TyPy 1.0 accepts: a member other than an
/// instance method or an attribute declared by its annotation (GEN-17).
fn planned_member(span: Span, what: &str) -> Diagnostic {
    Diagnostic::new(
        DiagnosticCode::error(201),
        span,
        format!("{what} in a protocol are planned for TyPy 0.13 (GEN-17)"),
    )
    .with_rule(RuleId::GEN_17)
}
