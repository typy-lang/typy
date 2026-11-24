//! Tests of `subset::pattern_key`.

use super::{Key, float_repr, key_repr, same_key};

#[test]
fn one_true_and_one_point_zero_are_one_key() {
    let one = Key::Int("1".to_owned());
    assert!(same_key(&one, &Key::Bool(true)));
    assert!(same_key(&Key::Float(1.0), &one));
    assert!(same_key(&Key::Int("-0".to_owned()), &Key::Bool(false)));
    assert!(!same_key(&Key::Float(1.5), &one));
    assert!(!same_key(&Key::Str("1".to_owned()), &one));
    assert!(same_key(&Key::None, &Key::None));
}

#[test]
fn keys_are_written_as_repr_writes_them() {
    let cases = [
        (Key::Str("a".to_owned()), "'a'"),
        (Key::Str("it's".to_owned()), "\"it's\""),
        (Key::Str("'\"\n".to_owned()), "'\\'\"\\n'"),
        (Key::Bytes(vec![b'a', 0xff, b'\\']), "b'a\\xff\\\\'"),
        (
            Key::Str("\u{1}\u{7f}\u{e9}".to_owned()),
            "'\\x01\\x7f\u{e9}'",
        ),
        (Key::Int("007".to_owned()), "7"),
        (Key::Bool(true), "True"),
        (Key::None, "None"),
    ];
    for (key, expected) in cases {
        assert_eq!(key_repr(&key), expected);
    }
}

#[test]
fn floats_are_written_as_repr_writes_them() {
    let cases = [
        (1.0, "1.0"),
        (0.0, "0.0"),
        (-0.0, "-0.0"),
        (0.1, "0.1"),
        (-2.5, "-2.5"),
        (1e16, "1e+16"),
        (1.5e-5, "1.5e-05"),
        (f64::INFINITY, "inf"),
        (f64::NEG_INFINITY, "-inf"),
        (f64::NAN, "nan"),
    ];
    for (value, expected) in cases {
        assert_eq!(float_repr(value), expected);
    }
}
