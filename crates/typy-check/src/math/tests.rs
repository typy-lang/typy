//! Tests of `math`.

use super::{CONSTANTS, FUNCTIONS, MathFunction, MathName, MathShape, math_name};

#[test]
fn the_names_are_sorted_and_each_function_has_one() {
    assert!(FUNCTIONS.windows(2).all(|pair| pair[0].0 < pair[1].0));
    for (name, function) in FUNCTIONS {
        assert_eq!(function.name(), name);
        assert_eq!(math_name(name), Some(MathName::Function(function)));
    }
    for (name, value) in CONSTANTS {
        match math_name(name) {
            Some(MathName::Constant(found)) => {
                assert_eq!(found.to_bits(), value.to_bits(), "{name}");
            }
            other => panic!("{name} is a constant, not {other:?}"),
        }
    }
    assert_eq!(
        math_name("fsum"),
        Some(MathName::Function(MathFunction::Fsum))
    );
    assert_eq!(math_name("square"), None);
}

#[test]
fn shapes_and_versions_follow_cpython() {
    assert_eq!(MathFunction::Sqrt.shape(), MathShape::Real(1));
    assert_eq!(MathFunction::Fma.shape(), MathShape::Real(3));
    assert_eq!(MathFunction::Gcd.shape(), MathShape::Fold);
    assert_eq!(MathFunction::Fma.since(), 13);
    assert_eq!(MathFunction::Signbit.since(), 15);
    assert_eq!(MathFunction::Sqrt.since(), 12);
}
