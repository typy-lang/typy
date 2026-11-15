//! Checks a method that overrides a base's method against it, by the rules of PEP 484 type
//! checkers, so a call through the base's type is valid for the override (CLS-04, ADR-0021).

use typy_diagnostics::{Diagnostic, DiagnosticCode, RuleId, Span};

use crate::typed::FunctionSignature;
use crate::types::Type;

/// A method and the base method it overrides, each by its qualified name.
pub(super) struct Override<'signatures> {
    pub(super) name: &'signatures str,
    pub(super) signature: &'signatures FunctionSignature,
    pub(super) base_name: &'signatures str,
    pub(super) base: &'signatures FunctionSignature,
    pub(super) span: Span,
}

impl Override<'_> {
    /// Returns why the override does not fit its base method, if it does not: it takes the
    /// base's parameters in the same order with the same names, each of a type that accepts
    /// the base's and with a default where the base has one, and returns a type the base's
    /// return type accepts. The instance, the first parameter, is not compared.
    pub(super) fn check(&self) -> Option<Diagnostic> {
        let (own, base) = (&self.signature.parameters, &self.base.parameters);
        if own.len() != base.len() {
            return Some(self.mismatch(format!(
                "`{}` takes {} parameters, but `{}`, which it overrides, takes {}",
                self.name,
                own.len(),
                self.base_name,
                base.len()
            )));
        }
        for (own, base) in own.iter().zip(base).skip(1) {
            if own.name != base.name {
                return Some(self.mismatch(format!(
                    "`{}` names a parameter `{}` where `{}`, which it overrides, names it `{}`",
                    self.name, own.name, self.base_name, base.name
                )));
            }
            if !base.ty.is_assignable_to(&own.ty) {
                return Some(self.mismatch(format!(
                    "`{}` takes {} for `{}`, which does not accept the {} `{}` takes",
                    self.name, own.ty, own.name, base.ty, self.base_name
                )));
            }
            if base.default.is_some() && own.default.is_none() {
                return Some(self.mismatch(format!(
                    "`{}` has no default for `{}`, but `{}`, which it overrides, has one",
                    self.name, own.name, self.base_name
                )));
            }
            // A call through the base fills a missing argument with the base's default, so an
            // override that gives another would run with the wrong value.
            if base.default.is_some() && own.default != base.default {
                return Some(
                    Diagnostic::new(
                        DiagnosticCode::error(201),
                        self.span,
                        format!(
                            "an override whose default for `{}` differs from that of `{}` is \
                             planned for TyPy 0.5 (CLS-04)",
                            own.name, self.base_name
                        ),
                    )
                    .with_rule(RuleId::CLS_04)
                    .with_help("give the parameter the base's default"),
                );
            }
            if !same_representation(&base.ty, &own.ty) {
                return Some(self.adapter_planned(&own.ty, &base.ty));
            }
        }
        let (returns, base_returns) = (&self.signature.returns, &self.base.returns);
        if !returns.is_assignable_to(base_returns) {
            return Some(self.mismatch(format!(
                "`{}` returns {returns}, which the {base_returns} of `{}` does not accept",
                self.name, self.base_name
            )));
        }
        if !same_representation(returns, base_returns) {
            return Some(self.adapter_planned(returns, base_returns));
        }
        None
    }

    fn mismatch(&self, message: String) -> Diagnostic {
        Diagnostic::new(DiagnosticCode::error(404), self.span, message).with_rule(RuleId::CLS_04)
    }

    /// Reports an override whose call would need its arguments or result converted, such as
    /// `int | None` where the base takes an `int`.
    fn adapter_planned(&self, own: &Type, base: &Type) -> Diagnostic {
        Diagnostic::new(
            DiagnosticCode::error(201),
            self.span,
            format!(
                "an override with {own} where `{}` has {base} is planned for TyPy 0.5 (CLS-04)",
                self.base_name
            ),
        )
        .with_rule(RuleId::CLS_04)
        .with_help(format!(
            "declare it as {base}, as `{}` does",
            self.base_name
        ))
    }
}

/// Returns true when a value of one type is held as one of the other: the same type, or two
/// instances or exceptions, optional or not, which are each an address.
fn same_representation(one: &Type, other: &Type) -> bool {
    one == other || (is_address(one) && is_address(other))
}

fn is_address(ty: &Type) -> bool {
    match ty {
        Type::Optional(inner) => matches!(inner.as_ref(), Type::Instance(_) | Type::Exception(_)),
        Type::Instance(_) | Type::Exception(_) => true,
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::Override;
    use crate::class_type::InstanceClass;
    use crate::typed::{ClassId, FunctionSignature, SignatureParameter, TypedConstant};
    use crate::types::Type;
    use typy_diagnostics::{FileId, Span, TextOffset};

    fn parameter(name: &str, ty: Type) -> SignatureParameter {
        SignatureParameter {
            name: name.to_owned(),
            ty,
            default: None,
        }
    }

    fn signature(parameters: Vec<SignatureParameter>, returns: Type) -> FunctionSignature {
        let mut all = vec![parameter("self", Type::Error)];
        all.extend(parameters);
        FunctionSignature {
            parameters: all,
            returns,
        }
    }

    fn message(own: &FunctionSignature, base: &FunctionSignature) -> Option<String> {
        Override {
            name: "Circle.area",
            signature: own,
            base_name: "Shape.area",
            base,
            span: Span::empty(FileId(0), TextOffset(0)),
        }
        .check()
        .map(|diagnostic| format!("{} {}", diagnostic.code(), diagnostic.message()))
    }

    #[test]
    fn an_override_widens_parameters_and_narrows_its_result() {
        let shape = Type::instance(InstanceClass::root(ClassId(0), "Shape"));
        let circle = Type::instance(InstanceClass {
            id: ClassId(1),
            name: "Circle".to_owned(),
            ancestors: vec![(ClassId(0), "Shape".to_owned())],
            operators: crate::class_type::Operators::default(),
            generic: None,
            protocol: false,
        });
        let base = signature(vec![parameter("other", circle.clone())], shape.clone());
        let own = signature(
            vec![parameter("other", Type::optional(shape.clone()))],
            circle.clone(),
        );
        assert_eq!(message(&own, &base), None);
        let narrower = signature(vec![parameter("other", circle.clone())], shape.clone());
        let wider_base = signature(vec![parameter("other", shape.clone())], shape.clone());
        assert!(
            message(&narrower, &wider_base).is_some_and(|text| text.contains(
                "takes Circle for `other`, which does not accept the Shape `Shape.area` takes"
            ))
        );
        let wider_result = signature(vec![parameter("other", circle.clone())], shape);
        let narrow_base = signature(vec![parameter("other", circle.clone())], circle);
        assert!(
            message(&wider_result, &narrow_base)
                .is_some_and(|text| text.contains("returns Shape, which the Circle"))
        );
    }

    #[test]
    fn an_override_keeps_the_parameters_names_count_and_defaults() {
        let base = signature(vec![parameter("scale", Type::Int)], Type::Int);
        let fewer = signature(Vec::new(), Type::Int);
        assert!(message(&fewer, &base).is_some_and(|text| text.contains("takes 1 parameters")));
        let renamed = signature(vec![parameter("factor", Type::Int)], Type::Int);
        assert!(
            message(&renamed, &base)
                .is_some_and(|text| text.contains("names a parameter `factor`"))
        );
        let mut defaulted = base.clone();
        defaulted.parameters[1].default = Some(TypedConstant::Int("1".to_owned()));
        assert!(message(&base, &defaulted).is_some_and(|text| text.contains("has no default")));
        assert_eq!(message(&defaulted, &base), None);
        assert_eq!(message(&defaulted, &defaulted), None);
        let mut other = defaulted.clone();
        other.parameters[1].default = Some(TypedConstant::Int("2".to_owned()));
        let text = message(&other, &defaulted).unwrap_or_default();
        assert!(text.starts_with("E0201"), "{text}");
        assert!(text.contains("default for `scale`"), "{text}");
    }

    #[test]
    fn an_override_may_return_an_exception_of_a_derived_class() {
        use crate::exception::{BuiltinException, ExceptionClass};
        let base_error = Type::Exception(ExceptionClass::Builtin(BuiltinException::Exception));
        let value_error = Type::Exception(ExceptionClass::Builtin(BuiltinException::ValueError));
        let base = signature(Vec::new(), Type::optional(base_error));
        let own = signature(Vec::new(), value_error);
        assert_eq!(message(&own, &base), None);
    }

    #[test]
    fn an_override_held_differently_from_its_base_is_planned() {
        let base = signature(vec![parameter("scale", Type::Int)], Type::Int);
        let optional = signature(
            vec![parameter("scale", Type::optional(Type::Int))],
            Type::Int,
        );
        let text = message(&optional, &base).unwrap_or_default();
        assert!(text.starts_with("E0201"), "{text}");
        assert!(text.contains("TyPy 0.5"), "{text}");
        let base_optional = signature(Vec::new(), Type::optional(Type::Int));
        let plain = signature(Vec::new(), Type::Int);
        assert!(message(&plain, &base_optional).is_some_and(|text| text.starts_with("E0201")));
    }
}
