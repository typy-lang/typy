//! Tests of `check::overriding`.

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
        default_call: None,
        kind: typy_source::syntax::ParameterKind::Normal,
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
    .err()
    .map(|diagnostic| format!("{} {}", diagnostic.code(), diagnostic.message()))
}

fn fit(own: &FunctionSignature, base: &FunctionSignature) -> Option<super::Fit> {
    Override {
        name: "Circle.area",
        signature: own,
        base_name: "Shape.area",
        base,
        span: Span::empty(FileId(0), TextOffset(0)),
    }
    .check()
    .ok()
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
    assert!(message(&narrower, &wider_base).is_some_and(|text| {
        text.contains(
            "takes Circle for `other`, which does not accept the Shape `Shape.area` takes",
        )
    }));
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
        message(&renamed, &base).is_some_and(|text| text.contains("names a parameter `factor`"))
    );
    let mut defaulted = base.clone();
    defaulted.parameters[1].default = Some(TypedConstant::Int("1".to_owned()));
    assert!(message(&base, &defaulted).is_some_and(|text| text.contains("has no default")));
    assert_eq!(message(&defaulted, &base), None);
    assert_eq!(message(&defaulted, &defaulted), None);
    // An override may give the parameter a default of its own (CLS-04).
    let mut other = defaulted.clone();
    other.parameters[1].default = Some(TypedConstant::Int("2".to_owned()));
    assert_eq!(message(&other, &defaulted), None);
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
fn an_override_held_differently_from_its_base_needs_an_adapter() {
    use super::Fit;
    let base = signature(vec![parameter("scale", Type::Int)], Type::Int);
    let optional = signature(
        vec![parameter("scale", Type::optional(Type::Int))],
        Type::Int,
    );
    assert_eq!(fit(&optional, &base), Some(Fit::Adapted));
    let base_optional = signature(Vec::new(), Type::optional(Type::Int));
    let plain = signature(Vec::new(), Type::Int);
    assert_eq!(fit(&plain, &base_optional), Some(Fit::Adapted));
    assert_eq!(fit(&base, &base), Some(Fit::Exact));
}
