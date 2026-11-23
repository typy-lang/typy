//! Tests of `check::overload_fit`.

use typy_source::syntax::ParameterKind;

use super::{body_signature, fit};
use crate::typed::{FunctionSignature, SignatureParameter, TypedConstant};
use crate::types::Type;

fn parameter(name: &str, ty: Type, default: Option<TypedConstant>) -> SignatureParameter {
    SignatureParameter {
        name: name.to_owned(),
        ty,
        default,
        default_call: None,
        kind: ParameterKind::Normal,
    }
}

fn signature(parameters: Vec<SignatureParameter>, returns: Type) -> FunctionSignature {
    FunctionSignature {
        parameters,
        returns,
    }
}

fn implementation() -> FunctionSignature {
    signature(
        vec![
            parameter("key", Type::union(vec![Type::Int, Type::Str]), None),
            parameter(
                "default",
                Type::optional(Type::Str),
                Some(TypedConstant::None),
            ),
        ],
        Type::optional(Type::Str),
    )
}

#[test]
fn an_overload_reads_the_implementations_positions_and_defaults() {
    let overload = signature(
        vec![
            parameter("key", Type::Int, None),
            parameter("default", Type::Str, Some(TypedConstant::None)),
        ],
        Type::Str,
    );
    let view = fit(&overload, &implementation()).expect("the overload fits");
    assert_eq!(view.positions, [0, 1]);
    assert_eq!(
        view.signature.parameters[1].default,
        Some(TypedConstant::None)
    );
    let body = body_signature(&view, &implementation());
    assert_eq!(body.parameters[0].ty, Type::Int);
    assert_eq!(body.parameters[1].ty, Type::Str);
    assert_eq!(body.returns, Type::Str);
    // An overload that leaves out a parameter with a default fits, at its own position.
    let shorter = signature(
        vec![parameter("key", Type::Str, None)],
        Type::optional(Type::Str),
    );
    let view = fit(&shorter, &implementation()).expect("the overload fits");
    assert_eq!(view.positions, [0]);
    let body = body_signature(&view, &implementation());
    assert_eq!(body.parameters[1].ty, Type::optional(Type::Str));
}

#[test]
fn an_overload_the_implementation_does_not_take_says_why() {
    let cases = [
        (
            signature(vec![parameter("other", Type::Int, None)], Type::Str),
            "the implementation has no parameter `other` in that place",
        ),
        (
            signature(
                vec![
                    parameter("default", Type::Str, None),
                    parameter("key", Type::Int, None),
                ],
                Type::Str,
            ),
            "it leaves out `key`, which has no default in the implementation",
        ),
        (
            signature(vec![parameter("key", Type::Float, None)], Type::Str),
            "its `key: float` is not held by the implementation's `key: int | str`",
        ),
        (
            signature(vec![parameter("key", Type::Int, None)], Type::Int),
            "its result, int, is not one the implementation's str | None holds",
        ),
    ];
    for (overload, reason) in cases {
        assert_eq!(
            fit(&overload, &implementation()).err().as_deref(),
            Some(reason)
        );
    }
}
