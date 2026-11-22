//! Tests of `NotImplemented`.

#[test]
fn not_implemented_is_written_by_its_name_and_its_type_named_as_cpython_names_it() {
    // SAFETY: the object is the runtime's static.
    unsafe {
        let object = super::typy_rt_not_implemented().cast_mut();
        let text = super::not_implemented_repr(object).value;
        let written: String = (0..crate::string::length(text))
            .map(|index| char::from_u32(crate::string::code_point(text, index)).unwrap())
            .collect();
        assert_eq!(written, "NotImplemented");
        crate::refcount::typy_rt_decrement(text.cast());
        let ty = crate::type_object::typy_rt_type_text(&raw const super::NOT_IMPLEMENTED_TYPE);
        let named: String = (0..crate::string::length(ty))
            .map(|index| char::from_u32(crate::string::code_point(ty, index)).unwrap())
            .collect();
        assert_eq!(named, "<class 'NotImplementedType'>");
        crate::refcount::typy_rt_decrement(ty.cast());
    }
}

/// Returns the text of `str` of an exception, and releases it.
fn shown(exception: *mut crate::ExceptionObject) -> String {
    // SAFETY: the exception is live and owned here.
    unsafe {
        let text = crate::exception::classes::BASE_EXCEPTION.str.unwrap()(exception.cast());
        let shown = (0..crate::string::length(text.value))
            .map(|index| char::from_u32(crate::string::code_point(text.value, index)).unwrap())
            .collect();
        crate::refcount::typy_rt_decrement(text.value.cast());
        crate::refcount::typy_rt_decrement(exception.cast());
        shown
    }
}

#[test]
fn operands_no_method_takes_raise_type_error_as_cpython_words_it() {
    use crate::type_object::{FLOAT, INT};
    let plus = crate::string::from_str("+");
    let less = crate::string::from_str("<=");
    // SAFETY: the strings are live, and the type objects are the runtime's statics.
    unsafe {
        assert_eq!(
            shown(super::typy_rt_operands_unsupported(
                plus,
                &raw const INT,
                &raw const FLOAT
            )),
            "unsupported operand type(s) for +: 'int' and 'float'"
        );
        assert_eq!(
            shown(super::typy_rt_ordering_unsupported(
                less,
                &raw const FLOAT,
                &raw const INT
            )),
            "'<=' not supported between instances of 'float' and 'int'"
        );
        crate::refcount::typy_rt_decrement(plus.cast());
        crate::refcount::typy_rt_decrement(less.cast());
    }
}

#[test]
fn typ_24_a_wrong_cast_raises_type_error_naming_the_type_and_the_class() {
    use crate::type_object::STR;
    let target = crate::string::from_str("int | None");
    // SAFETY: the string is live, and the type object is the runtime's static.
    unsafe {
        assert_eq!(
            shown(super::typy_rt_cast_failed(target, &raw const STR)),
            "cast to int | None found str"
        );
        crate::refcount::typy_rt_decrement(target.cast());
    }
}
