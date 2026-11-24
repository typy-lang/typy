//! Tests of `literal_type`.

use super::LiteralValue;
use crate::typed::TypedConstant;
use crate::types::Type;

fn string(text: &str) -> Type {
    Type::literal(LiteralValue::Str(text.to_owned()))
}

fn int(digits: &str) -> Type {
    Type::literal(LiteralValue::Int(digits.to_owned()))
}

#[test]
fn a_constant_gives_its_literal_value() {
    let cases = [
        (
            TypedConstant::Int("007".to_owned()),
            Some(LiteralValue::Int("7".to_owned())),
        ),
        (
            TypedConstant::Int("-0".to_owned()),
            Some(LiteralValue::Int("0".to_owned())),
        ),
        (TypedConstant::Bool(true), Some(LiteralValue::Bool(true))),
        (
            TypedConstant::Str("r".to_owned()),
            Some(LiteralValue::Str("r".to_owned())),
        ),
        (
            TypedConstant::Bytes(vec![1]),
            Some(LiteralValue::Bytes(vec![1])),
        ),
        (
            TypedConstant::StrPoints(vec![0xd800]),
            Some(LiteralValue::StrPoints(vec![0xd800])),
        ),
        (TypedConstant::None, None),
        (TypedConstant::Float(1.5), None),
    ];
    for (constant, value) in cases {
        assert_eq!(LiteralValue::of_constant(&constant), value, "{constant:?}");
    }
}

#[test]
fn an_int_is_negated_and_nothing_else_is() {
    let five = LiteralValue::Int("5".to_owned());
    assert_eq!(five.negated(), Some(LiteralValue::Int("-5".to_owned())));
    assert_eq!(five.negated().and_then(|value| value.negated()), Some(five));
    let zero = LiteralValue::Int("0".to_owned());
    assert_eq!(zero.negated(), Some(zero));
    assert_eq!(LiteralValue::Bool(true).negated(), None);
}

#[test]
fn true_equals_one_and_false_equals_zero() {
    let one = LiteralValue::Int("1".to_owned());
    let zero = LiteralValue::Int("0".to_owned());
    assert!(LiteralValue::Bool(true).equals(&one));
    assert!(zero.equals(&LiteralValue::Bool(false)));
    assert!(!LiteralValue::Bool(true).equals(&zero));
    assert!(!LiteralValue::Str("1".to_owned()).equals(&one));
}

#[test]
fn values_are_written_as_repr_writes_them() {
    let cases = [
        (LiteralValue::Int("-3".to_owned()), "-3"),
        (LiteralValue::Bool(false), "False"),
        (LiteralValue::Str("it's".to_owned()), "\"it's\""),
        (LiteralValue::Bytes(b"a\x00".to_vec()), "b'a\\x00'"),
        (
            LiteralValue::StrPoints(vec![0x61, 0xdc80, 0x27, 0x0a]),
            "'a\\udc80\\'\\n'",
        ),
        (
            LiteralValue::StrPoints(vec![0x5c, 0x0d, 0x09, 0x01, 0xd800]),
            "'\\\\\\r\\t\\x01\\ud800'",
        ),
    ];
    for (value, written) in cases {
        assert_eq!(value.to_string(), written);
    }
    let union = Type::union(vec![string("r"), Type::Int, string("w")]);
    assert_eq!(union.to_string(), "int | Literal['r', 'w']");
    assert_eq!(Type::optional(int("1")).to_string(), "Literal[1] | None");
}

#[test]
fn a_literal_type_takes_only_its_own_values() {
    let mode = Type::union(vec![string("r"), string("w")]);
    assert!(string("r").takes_literals_of(&mode));
    assert!(!string("x").takes_literals_of(&mode));
    assert!(!Type::Str.takes_literals_of(&mode));
    assert!(string("r").takes_literals_of(&Type::Str));
    assert!(mode.takes_literals_of(&Type::Str));
    assert!(!mode.takes_literals_of(&string("r")));
    assert!(Type::NoneType.takes_literals_of(&Type::optional(mode.clone())));
    assert!(Type::Int.takes_literals_of(&Type::union(vec![string("r"), Type::Int])));
    assert!(!LiteralValue::Bool(true).base().takes_literals_of(&int("1")));
    assert!(!Type::literal(LiteralValue::Bool(true)).takes_literals_of(&int("1")));
}

#[test]
fn a_union_keeps_the_base_type_over_its_literals() {
    assert_eq!(Type::union(vec![string("r"), Type::Str]), Type::Str);
    assert_eq!(Type::union(vec![Type::Str, string("r")]), Type::Str);
    let both = Type::union(vec![string("r"), string("w"), string("r")]);
    assert_eq!(
        both.literal_values().map(|(values, _)| values.len()),
        Some(2)
    );
}

#[test]
fn a_value_type_has_no_literal() {
    let declared = Type::List(Box::new(Type::union(vec![string("r"), int("2")])));
    assert!(declared.has_literal());
    assert_eq!(
        declared.value_type(),
        Type::List(Box::new(Type::union(vec![Type::Str, Type::Int])))
    );
    assert_eq!(
        Type::optional(string("r")).value_type(),
        Type::optional(Type::Str)
    );
    assert!(!Type::Int.has_literal());
}

#[test]
fn two_literal_narrowings_meet_in_the_values_both_allow() {
    let mode = Type::union(vec![string("r"), string("w")]);
    assert_eq!(super::meet(&mode, &string("w")), Some(string("w")));
    assert_eq!(super::meet(&string("r"), &string("w")), None);
    assert_eq!(super::meet(&mode, &Type::Str), None);
    assert_eq!(
        super::meet(&Type::optional(mode.clone()), &Type::optional(string("r"))),
        Some(Type::optional(string("r")))
    );
    assert_eq!(
        super::meet(&Type::optional(mode), &string("r")),
        Some(string("r"))
    );
}
