//! Tests of the built-in exception classes and their bases.

use super::{
    ARITHMETIC_ERROR, BASE_EXCEPTION, BROKEN_PIPE_ERROR, CONNECTION_ERROR, EVERY_CLASS, EXCEPTION,
    FILE_NOT_FOUND_ERROR, KEY_ERROR, LOOKUP_ERROR, OBJECT, OS_ERROR, TYPE_ERROR,
    UNICODE_DECODE_ERROR, UNSUPPORTED_OPERATION, VALUE_ERROR, ZERO_DIVISION_ERROR, derives_from,
};

#[test]
fn every_exception_class_derives_from_base_exception_and_object() {
    for class in EVERY_CLASS {
        assert!(derives_from(class, &BASE_EXCEPTION), "{}", class.name());
        assert!(derives_from(class, &OBJECT), "{}", class.name());
    }
}

#[test]
fn a_class_derives_from_its_bases_and_not_from_its_siblings() {
    let cases = [
        (&ZERO_DIVISION_ERROR, &ARITHMETIC_ERROR, true),
        (&ZERO_DIVISION_ERROR, &EXCEPTION, true),
        (&KEY_ERROR, &LOOKUP_ERROR, true),
        (&UNICODE_DECODE_ERROR, &VALUE_ERROR, true),
        (&KEY_ERROR, &KEY_ERROR, true),
        (&KEY_ERROR, &TYPE_ERROR, false),
        (&EXCEPTION, &KEY_ERROR, false),
        (&OBJECT, &BASE_EXCEPTION, false),
        (&FILE_NOT_FOUND_ERROR, &OS_ERROR, true),
        (&BROKEN_PIPE_ERROR, &CONNECTION_ERROR, true),
        (&BROKEN_PIPE_ERROR, &OS_ERROR, true),
        (&FILE_NOT_FOUND_ERROR, &VALUE_ERROR, false),
        (&UNSUPPORTED_OPERATION, &OS_ERROR, true),
        (&UNSUPPORTED_OPERATION, &VALUE_ERROR, true),
        (&UNSUPPORTED_OPERATION, &EXCEPTION, true),
        (&UNSUPPORTED_OPERATION, &KEY_ERROR, false),
        (&UNSUPPORTED_OPERATION, &FILE_NOT_FOUND_ERROR, false),
    ];
    for (class, base, expected) in cases {
        assert_eq!(
            derives_from(class, base),
            expected,
            "{} {}",
            class.name(),
            base.name()
        );
    }
}
