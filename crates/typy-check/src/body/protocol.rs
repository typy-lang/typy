//! Protocols: a class satisfies a protocol when it has each of the protocol's methods with a
//! signature that takes every call the protocol's takes, and each of its attributes with the
//! same type, checked when a type argument is given for a parameter the protocol bounds
//! (GEN-04, ADR-0023).

use typy_diagnostics::{Diagnostic, DiagnosticCode, RuleId};

use super::BodyChecker;
use crate::typed::{ClassId, FunctionSignature, TypedExpr};
use crate::types::Type;

impl BodyChecker<'_> {
    /// Refuses a call of a protocol, which has no instances; CPython raises `TypeError`.
    pub(super) fn protocol_called(
        &mut self,
        class: ClassId,
        arguments: &[typy_source::syntax::Expr],
        span: typy_diagnostics::Span,
    ) -> Option<TypedExpr> {
        if !self.program.is_protocol(class) {
            return None;
        }
        for argument in arguments {
            self.expression(argument);
        }
        let name = &self.program.classes[class.0 as usize].name;
        let diagnostic = Diagnostic::new(
            DiagnosticCode::error(401),
            span,
            format!("`{name}` is a protocol, which has no instances; CPython raises `TypeError`"),
        )
        .with_rule(RuleId::GEN_04)
        .with_help("make an instance of a class that satisfies it");
        Some(self.error(diagnostic, span))
    }

    /// Returns the protocol a bound names, if it names one.
    pub(super) fn bound_protocol(&self, bound: &Type) -> Option<ClassId> {
        bound
            .class_id()
            .filter(|class| self.program.is_protocol(*class))
    }

    /// Returns why a value of type `ty` does not satisfy `protocol`, or `None` when it does.
    pub(super) fn unsatisfied(&self, ty: &Type, protocol: ClassId) -> Option<String> {
        match ty {
            Type::Error => None,
            Type::Instance(class) => self.class_unsatisfied(class.id, protocol),
            Type::Union(members) => members
                .iter()
                .find_map(|member| self.unsatisfied(member, protocol)),
            // A type parameter stands for any type its bound takes.
            Type::Parameter(parameter) => match &parameter.bound {
                Some(bound) => self.unsatisfied(bound, protocol),
                None => Some(format!(
                    "a value of type {} may be of any type",
                    parameter.name
                )),
            },
            other => Some(format!(
                "{} is not an instance of a class",
                other.with_article()
            )),
        }
    }

    /// Returns the first member of `protocol` that `class` lacks or has with another type.
    fn class_unsatisfied(&self, class: ClassId, protocol: ClassId) -> Option<String> {
        if class == protocol {
            return None;
        }
        let wanted = &self.program.classes[protocol.0 as usize];
        let name = &self.program.classes[class.0 as usize].name;
        for (method, wanted_id) in &wanted.methods {
            let Some(found) = self
                .program
                .method(class, method)
                .filter(|_| !self.is_static_method(class, method))
            else {
                return Some(format!("`{name}` has no method `{method}`"));
            };
            let signature = &self.program.signatures[found.0 as usize];
            let protocol_signature = &self.program.signatures[wanted_id.0 as usize];
            if let Some(reason) = incompatible(signature, protocol_signature) {
                return Some(format!("its method `{name}.{method}` {reason}"));
            }
        }
        let info = &self.program.classes[class.0 as usize];
        for attribute in &wanted.attributes {
            // A class holds the attributes of its bases as its own.
            let Some(index) = info.attribute(&attribute.name) else {
                return Some(format!("`{name}` has no attribute `{}`", attribute.name));
            };
            match (self.program.attribute_ty(class, index), &attribute.ty) {
                (Some(found), Some(wanted)) if !found.matches(wanted) => {
                    return Some(format!(
                        "its attribute `{name}.{}` is {}, and the protocol's is {}",
                        attribute.name,
                        found.with_article(),
                        wanted.with_article()
                    ));
                }
                _ => {}
            }
        }
        None
    }
}

/// Returns why a method with `own` cannot take every call that a protocol's method with
/// `wanted` takes, if it cannot: the same parameters after the instance, by name, each
/// accepting the protocol's type and with a default where the protocol's has one, then only
/// parameters with a default, and a result the protocol's accepts. A copy binds each call to
/// the class's own method, so a default the protocol's method lacks fills its parameter.
fn incompatible(own: &FunctionSignature, wanted: &FunctionSignature) -> Option<String> {
    if own.parameters.len() < wanted.parameters.len() {
        return Some(format!(
            "takes {} parameters, and the protocol's takes {}",
            own.parameters.len(),
            wanted.parameters.len()
        ));
    }
    if let Some(extra) = own.parameters[wanted.parameters.len()..]
        .iter()
        .find(|parameter| parameter.default.is_none())
    {
        return Some(format!(
            "takes `{}`, which the protocol's does not, without a default",
            extra.name
        ));
    }
    for (own, wanted) in own.parameters.iter().zip(&wanted.parameters).skip(1) {
        if own.name != wanted.name {
            return Some(format!(
                "names a parameter `{}` where the protocol's names `{}`",
                own.name, wanted.name
            ));
        }
        if !wanted.ty.is_assignable_to(&own.ty) {
            return Some(format!(
                "takes {} for `{}`, which does not accept the protocol's {}",
                own.ty, own.name, wanted.ty
            ));
        }
        if wanted.default.is_some() && own.default.is_none() {
            return Some(format!(
                "has no default for `{}`, and the protocol's has one",
                own.name
            ));
        }
    }
    if !own.returns.is_assignable_to(&wanted.returns) {
        return Some(format!(
            "returns {}, which the protocol's {} does not accept",
            own.returns, wanted.returns
        ));
    }
    None
}
