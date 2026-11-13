//! Tests of exception objects: their text, matching, chaining and the handled exception.

use super::classes::{
    ARITHMETIC_ERROR, EXCEPTION, KEY_ERROR, LOOKUP_ERROR, VALUE_ERROR, ZERO_DIVISION_ERROR,
};
use super::text::{typy_rt_exception_repr, typy_rt_exception_str};
use super::{
    ExceptionKind, ExceptionObject, raise, typy_rt_exception_enter, typy_rt_exception_handled,
    typy_rt_exception_leave, typy_rt_exception_matches, typy_rt_exception_new,
    typy_rt_exception_raise, typy_rt_exception_set_cause, typy_rt_traceback_add,
};
use crate::Raising;
use crate::list::ItemKind;
use crate::refcount::{typy_rt_decrement, typy_rt_increment};
use crate::string::{StrObject, code_point, from_str, length};
use crate::tuple::{Field, TupleType};
use crate::type_object::TypeObject;

/// Returns the text of a `str` and releases it.
fn text(result: Raising<*mut StrObject>) -> String {
    assert!(result.exception.is_null());
    // SAFETY: the string is live and owned here.
    unsafe {
        let value = (0..length(result.value))
            .map(|index| char::from_u32(code_point(result.value, index)).unwrap())
            .collect();
        typy_rt_decrement(result.value.cast());
        value
    }
}

/// Returns `str` and `repr` of an exception.
fn texts(exception: *mut ExceptionObject) -> (String, String) {
    // SAFETY: the exception is live.
    unsafe {
        (
            text(typy_rt_exception_str(exception.cast())),
            text(typy_rt_exception_repr(exception.cast())),
        )
    }
}

/// Returns the reference count of an exception.
fn count(exception: *mut ExceptionObject) -> u32 {
    // SAFETY: the exception is live.
    unsafe { (*exception).header.reference_count }
}

/// Releases exceptions.
fn release(exceptions: &[*mut ExceptionObject]) {
    for exception in exceptions {
        // SAFETY: each exception is live, and this is a reference the test owns.
        unsafe { typy_rt_decrement(exception.cast()) };
    }
}

static NO_FIELDS: [Field; 0] = [];
static NOTHING: TupleType = TupleType {
    size: 8,
    length: 0,
    fields: NO_FIELDS.as_ptr(),
};
static ONE_OBJECT_FIELD: [Field; 1] = [Field::plain(ItemKind::Object, 0)];
static ONE_OBJECT: TupleType = TupleType {
    size: 8,
    length: 1,
    fields: ONE_OBJECT_FIELD.as_ptr(),
};
static ONE_INT_FIELD: [Field; 1] = [Field::plain(ItemKind::Int, 0)];
static ONE_INT: TupleType = TupleType {
    size: 8,
    length: 1,
    fields: ONE_INT_FIELD.as_ptr(),
};
static TEXT_AND_INT_FIELDS: [Field; 2] = [
    Field::plain(ItemKind::Object, 0),
    Field::plain(ItemKind::Int, 8),
];
static TEXT_AND_INT: TupleType = TupleType {
    size: 16,
    length: 2,
    fields: TEXT_AND_INT_FIELDS.as_ptr(),
};

/// Returns an exception of `class` built from `words` laid out as `layout`, whose references
/// pass to the exception.
fn built(class: &TypeObject, layout: &TupleType, words: &[u64]) -> *mut ExceptionObject {
    // SAFETY: the words are live items of the layout; the exception retains its copy, and the
    // test's references are released after.
    unsafe {
        let exception = typy_rt_exception_new(class, layout, words.as_ptr().cast());
        if layout.length > 0 {
            crate::tuple::release(layout, words.as_ptr().expose_provenance() as u64);
        }
        exception
    }
}

/// Returns the item of a new `str`.
fn word_of(value: &str) -> u64 {
    from_str(value).expose_provenance() as u64
}

/// Returns the item of an `int`.
fn int_word(value: i64) -> u64 {
    crate::word::inline(value).unwrap().cast_unsigned()
}

#[test]
fn an_exception_keeps_its_kind_message_and_frames() {
    let exception = raise(ExceptionKind::ValueError, "bad");
    // SAFETY: the exception is live, and the names are static and null-terminated.
    unsafe {
        typy_rt_traceback_add(
            exception,
            c"f".as_ptr().cast(),
            c"app.py".as_ptr().cast(),
            3,
        );
        typy_rt_traceback_add(
            exception,
            c"<module>".as_ptr().cast(),
            c"app.py".as_ptr().cast(),
            7,
        );
        assert_eq!((*exception).kind(), ExceptionKind::ValueError);
        assert_eq!((*exception).traceback.len(), 2);
    }
    release(&[exception]);
}

#[test]
fn freeing_an_exception_releases_its_cause() {
    let cause = raise(ExceptionKind::ZeroDivisionError, "division by zero");
    let exception = raise(ExceptionKind::ValueError, "bad");
    // SAFETY: both exceptions are live, and the outer one takes a reference to its cause.
    unsafe { typy_rt_exception_set_cause(exception, cause) };
    release(&[exception]);
    assert_eq!(count(cause), 1);
    release(&[cause]);
}

#[test]
fn the_runtime_text_is_the_str_and_stands_for_the_one_argument_of_the_repr() {
    let cases = [
        (ExceptionKind::ValueError, "bad", "ValueError('bad')"),
        (ExceptionKind::KeyError, "'k'", "KeyError('k')"),
        (ExceptionKind::IndexError, "", "IndexError('')"),
    ];
    for (kind, message, repr) in cases {
        let exception = raise(kind, message);
        assert_eq!(texts(exception), (message.to_owned(), repr.to_owned()));
        release(&[exception]);
    }
}

#[test]
fn the_text_of_a_built_exception_follows_its_arguments() {
    let cases: [(&TypeObject, &TupleType, Vec<u64>, &str, &str); 6] = [
        (&VALUE_ERROR, &NOTHING, vec![], "", "ValueError()"),
        (
            &VALUE_ERROR,
            &ONE_OBJECT,
            vec![word_of("bad")],
            "bad",
            "ValueError('bad')",
        ),
        (
            &VALUE_ERROR,
            &ONE_INT,
            vec![int_word(7)],
            "7",
            "ValueError(7)",
        ),
        (
            &KEY_ERROR,
            &ONE_OBJECT,
            vec![word_of("k")],
            "'k'",
            "KeyError('k')",
        ),
        (
            &EXCEPTION,
            &TEXT_AND_INT,
            vec![word_of("a"), int_word(2)],
            "('a', 2)",
            "Exception('a', 2)",
        ),
        (
            &LOOKUP_ERROR,
            &ONE_INT,
            vec![int_word(-3)],
            "-3",
            "LookupError(-3)",
        ),
    ];
    for (class, layout, words, str, repr) in cases {
        let exception = built(class, layout, &words);
        assert_eq!(
            texts(exception),
            (str.to_owned(), repr.to_owned()),
            "{repr}"
        );
        release(&[exception]);
    }
}

#[test]
fn a_built_exception_releases_its_arguments() {
    let argument = from_str("held");
    // SAFETY: the string is live; the exception takes the second reference.
    unsafe { typy_rt_increment(argument.cast()) };
    let exception = built(
        &VALUE_ERROR,
        &ONE_OBJECT,
        &[argument.expose_provenance() as u64],
    );
    release(&[exception]);
    // SAFETY: the string is live, and the test owns its last reference.
    unsafe {
        assert_eq!((*argument).header.reference_count, 1);
        typy_rt_decrement(argument.cast());
    }
}

#[test]
fn an_exception_matches_its_class_and_its_bases_only() {
    let exception = raise(ExceptionKind::ZeroDivisionError, "division by zero");
    let cases: [(&TypeObject, bool); 4] = [
        (&ZERO_DIVISION_ERROR, true),
        (&ARITHMETIC_ERROR, true),
        (&EXCEPTION, true),
        (&LOOKUP_ERROR, false),
    ];
    for (class, expected) in cases {
        // SAFETY: the exception and the class are live.
        let matched = unsafe { typy_rt_exception_matches(exception, class) };
        assert_eq!(matched, expected, "{}", class.name());
    }
    release(&[exception]);
}

#[test]
fn an_exception_raised_while_one_is_handled_takes_it_as_its_context() {
    let handled = raise(ExceptionKind::KeyError, "'k'");
    // SAFETY: the handled exception outlives the clause.
    let previous = unsafe { typy_rt_exception_enter(handled) };
    assert!(previous.is_null());
    let inner = raise(ExceptionKind::ValueError, "inner");
    // SAFETY: both exceptions are live.
    unsafe {
        assert_eq!((*inner).context, handled);
        assert_eq!(count(handled), 2);
        typy_rt_exception_leave(previous);
    }
    let after = raise(ExceptionKind::ValueError, "after");
    // SAFETY: the exception is live.
    assert!(unsafe { (*after).context }.is_null());
    release(&[inner, after]);
    assert_eq!(count(handled), 1);
    release(&[handled]);
}

#[test]
fn raising_the_handled_exception_again_gives_it_no_context() {
    let handled = raise(ExceptionKind::KeyError, "'k'");
    // SAFETY: the handled exception outlives the clause.
    unsafe {
        let previous = typy_rt_exception_enter(handled);
        typy_rt_exception_raise(handled);
        assert!((*handled).context.is_null());
        typy_rt_exception_leave(previous);
    }
    release(&[handled]);
}

#[test]
fn a_context_that_would_form_a_cycle_is_cut() {
    let older = raise(ExceptionKind::ValueError, "older");
    let handled = raise(ExceptionKind::KeyError, "'k'");
    // SAFETY: both exceptions are live, and the handled one outlives the clause.
    unsafe {
        typy_rt_increment(older.cast());
        (*handled).context = older;
        let previous = typy_rt_exception_enter(handled);
        typy_rt_exception_raise(older);
        typy_rt_exception_leave(previous);
        assert!((*handled).context.is_null());
        assert_eq!((*older).context, handled);
        (*older).context = core::ptr::null_mut();
    }
    release(&[handled, older]);
}

#[test]
fn a_new_context_replaces_the_old_one() {
    let first = raise(ExceptionKind::KeyError, "'a'");
    let second = raise(ExceptionKind::KeyError, "'b'");
    let raised = raise(ExceptionKind::ValueError, "v");
    // SAFETY: the exceptions are live, and each handled one outlives its clause.
    unsafe {
        for handled in [first, second] {
            let previous = typy_rt_exception_enter(handled);
            typy_rt_exception_raise(raised);
            typy_rt_exception_leave(previous);
        }
        assert_eq!((*raised).context, second);
    }
    release(&[raised]);
    assert_eq!((count(first), count(second)), (1, 1));
    release(&[first, second]);
}

#[test]
fn a_bare_raise_gets_a_new_reference_to_the_handled_exception() {
    let handled = raise(ExceptionKind::KeyError, "'k'");
    // SAFETY: the handled exception outlives the clause.
    unsafe {
        let previous = typy_rt_exception_enter(handled);
        let again = typy_rt_exception_handled();
        assert_eq!(again, handled);
        assert_eq!(count(handled), 2);
        typy_rt_exception_leave(previous);
    }
    release(&[handled, handled]);
}

#[test]
fn from_none_leaves_no_cause_and_suppresses_the_context() {
    let exception = raise(ExceptionKind::ValueError, "v");
    // SAFETY: the exception is live.
    unsafe {
        typy_rt_exception_set_cause(exception, core::ptr::null_mut());
        assert!((*exception).cause.is_null());
        assert_eq!((*exception).suppress_context, 1);
    }
    release(&[exception]);
}

#[test]
fn a_new_cause_replaces_the_old_one() {
    let first = raise(ExceptionKind::KeyError, "'a'");
    let second = raise(ExceptionKind::KeyError, "'b'");
    let exception = raise(ExceptionKind::ValueError, "v");
    // SAFETY: the exceptions are live, and `exception` takes a reference to each cause.
    unsafe {
        typy_rt_exception_set_cause(exception, first);
        typy_rt_exception_set_cause(exception, second);
        assert_eq!((*exception).cause, second);
    }
    release(&[exception]);
    assert_eq!((count(first), count(second)), (1, 1));
    release(&[first, second]);
}

#[test]
fn a_unicode_error_shows_its_five_arguments_in_its_repr() {
    let data = crate::bytes::new_bytes(1);
    // SAFETY: the bytes are live and hold one byte; codec 0 is UTF-8 and handler 0 `strict`.
    let result = unsafe {
        crate::string::set_code_point(data, 0, 0xff);
        crate::codec::typy_rt_bytes_decode(
            data,
            int_word(0).cast_signed(),
            int_word(0).cast_signed(),
        )
    };
    let error = result.exception;
    assert!(!error.is_null());
    assert_eq!(
        texts(error),
        (
            "'utf-8' codec can't decode byte 0xff in position 0: invalid start byte".to_owned(),
            "UnicodeDecodeError('utf-8', b'\\xff', 0, 1, 'invalid start byte')".to_owned(),
        )
    );
    release(&[error]);
    // SAFETY: the bytes are live, and the test owns their last reference.
    unsafe { typy_rt_decrement(data.cast()) };
}

static OPTIONAL_OBJECT_FIELD: [Field; 1] = [Field::plain(ItemKind::OptionalObject, 0)];
static OPTIONAL_INT_FIELD: [Field; 1] = [Field::plain(ItemKind::OptionalInt, 0)];
static ONE_OPTIONAL_OBJECT: TupleType = TupleType {
    size: 8,
    length: 1,
    fields: OPTIONAL_OBJECT_FIELD.as_ptr(),
};
static ONE_OPTIONAL_INT: TupleType = TupleType {
    size: 8,
    length: 1,
    fields: OPTIONAL_INT_FIELD.as_ptr(),
};

#[test]
fn the_str_of_an_argument_follows_its_type() {
    let inner = raise(ExceptionKind::KeyError, "'k'");
    let ty: *const TypeObject = &raw const VALUE_ERROR;
    let cases: [(&TupleType, u64, &str, &str); 5] = [
        (&ONE_OPTIONAL_OBJECT, 0, "None", "ValueError(None)"),
        (&ONE_OPTIONAL_OBJECT, word_of("x"), "x", "ValueError('x')"),
        (&ONE_OPTIONAL_INT, int_word(5), "5", "ValueError(5)"),
        (
            &ONE_OBJECT,
            inner.expose_provenance() as u64,
            "'k'",
            "ValueError(KeyError('k'))",
        ),
        (
            &ONE_OBJECT,
            ty.expose_provenance() as u64,
            "<class 'ValueError'>",
            "ValueError(<class 'ValueError'>)",
        ),
    ];
    // SAFETY: the exception argument is live; the exception that holds it takes a reference.
    unsafe { typy_rt_increment(inner.cast()) };
    for (layout, word, str, repr) in cases {
        let exception = built(&VALUE_ERROR, layout, &[word]);
        assert_eq!(
            texts(exception),
            (str.to_owned(), repr.to_owned()),
            "{repr}"
        );
        release(&[exception]);
    }
    release(&[inner]);
}

unsafe extern "C" fn failing_repr(_: *mut crate::header::ObjectHeader) -> Raising<*mut StrObject> {
    Raising::error(
        core::ptr::null_mut(),
        raise(ExceptionKind::RuntimeError, "no text"),
    )
}

/// A class whose `repr` raises.
static SILENT_ERROR: TypeObject = TypeObject {
    repr: Some(failing_repr),
    ..TypeObject::new("SilentError", &raw const VALUE_ERROR)
};

#[test]
fn a_repr_that_raises_gives_its_exception() {
    let silent = built(&SILENT_ERROR, &NOTHING, &[0]);
    let exception = built(
        &VALUE_ERROR,
        &ONE_OBJECT,
        &[silent.expose_provenance() as u64],
    );
    // SAFETY: the exception is live.
    let result = unsafe { typy_rt_exception_repr(exception.cast()) };
    assert!(!result.exception.is_null());
    release(&[result.exception, exception]);
}

static ATTRIBUTE_FIELDS: [Field; 1] = [Field::plain(ItemKind::Object, 0)];
static ATTRIBUTES: TupleType = TupleType {
    size: 8,
    length: 1,
    fields: ATTRIBUTE_FIELDS.as_ptr(),
};

/// A class with one attribute, a `str`.
static CODED_ERROR: TypeObject = TypeObject {
    fields: &raw const ATTRIBUTES,
    ..TypeObject::new("CodedError", &raw const VALUE_ERROR)
};

#[test]
fn a_class_s_attributes_follow_the_exception_and_are_released_with_it() {
    let exception = built(&CODED_ERROR, &NOTHING, &[0]);
    let attribute = from_str("code");
    // SAFETY: the exception has one attribute slot after its fixed part; it takes the string.
    unsafe {
        #[allow(
            clippy::cast_ptr_alignment,
            reason = "the fixed part is a multiple of eight bytes from an aligned allocation"
        )]
        let slot = exception.cast::<u8>().add(super::PREFIX).cast::<u64>();
        assert_eq!(*slot, 0);
        typy_rt_increment(attribute.cast());
        *slot = attribute.expose_provenance() as u64;
        assert!(core::ptr::eq(
            crate::type_object::type_of(exception.cast()),
            &raw const CODED_ERROR
        ));
    }
    release(&[exception]);
    // SAFETY: the string is live, and the test owns its last reference.
    unsafe {
        assert_eq!((*attribute).header.reference_count, 1);
        typy_rt_decrement(attribute.cast());
    }
}

#[test]
fn a_cycle_deep_in_the_context_chain_is_cut() {
    let raised = raise(ExceptionKind::ValueError, "raised");
    let middle = raise(ExceptionKind::KeyError, "'m'");
    let handled = raise(ExceptionKind::KeyError, "'h'");
    // SAFETY: the exceptions are live, and the handled one outlives the clause.
    unsafe {
        typy_rt_increment(middle.cast());
        (*handled).context = middle;
        typy_rt_increment(raised.cast());
        (*middle).context = raised;
        let previous = typy_rt_exception_enter(handled);
        typy_rt_exception_raise(raised);
        typy_rt_exception_leave(previous);
        assert!((*middle).context.is_null());
        assert_eq!((*raised).context, handled);
        (*raised).context = core::ptr::null_mut();
    }
    release(&[handled, middle, raised]);
}

#[test]
fn fields_are_built_at_run_time_as_at_compile_time() {
    let field = Field::plain(ItemKind::Int, 16);
    assert_eq!((field.kind, field.offset), (0, 16));
    assert!(field.tuple.is_null());
    let class = super::classes::class_for_test("Probe");
    assert!(class.str.is_some() && class.repr.is_some());
}

#[test]
fn throwing_gives_a_new_reference_with_the_handled_context() {
    let handled = raise(ExceptionKind::KeyError, "'k'");
    let exception = built(&VALUE_ERROR, &NOTHING, &[0]);
    // SAFETY: both exceptions are live, and the handled one outlives the clause.
    unsafe {
        let previous = typy_rt_exception_enter(handled);
        let thrown = super::typy_rt_exception_throw(exception);
        typy_rt_exception_leave(previous);
        assert_eq!(thrown, exception);
        assert_eq!(count(exception), 2);
        assert_eq!((*exception).context, handled);
    }
    release(&[exception, exception, handled]);
}

#[test]
fn the_class_of_an_exception_is_its_type_object() {
    let exception = raise(ExceptionKind::KeyError, "'k'");
    // SAFETY: the exception is live.
    let class = unsafe { super::typy_rt_exception_class(exception) };
    assert!(core::ptr::eq(class, &raw const KEY_ERROR));
    release(&[exception]);
}

/// A class with no functions of its own to write its objects.
static PLAIN_ERROR: TypeObject = TypeObject::new("PlainError", &raw const VALUE_ERROR);

/// A class whose `repr` is its only writer.
static REPR_ERROR: TypeObject = TypeObject {
    repr: Some(typy_rt_exception_repr),
    ..TypeObject::new("ReprError", &raw const VALUE_ERROR)
};

#[test]
fn the_text_of_an_exception_comes_through_its_class() {
    let cases: [(&TypeObject, i64, &str); 5] = [
        (&VALUE_ERROR, 0, "x"),
        (&VALUE_ERROR, 1, "ValueError('x')"),
        (&REPR_ERROR, 0, "ReprError('x')"),
        (&PLAIN_ERROR, 0, "PlainError"),
        (&PLAIN_ERROR, 1, "PlainError"),
    ];
    for (class, repr, expected) in cases {
        let exception = built(class, &ONE_OBJECT, &[word_of("x")]);
        // SAFETY: the exception is live.
        let result =
            unsafe { super::typy_rt_exception_text(exception, int_word(repr).cast_signed()) };
        assert_eq!(text(result), expected, "{} {repr}", class.name());
        release(&[exception]);
    }
}

#[test]
fn setting_the_arguments_replaces_and_releases_the_old_ones() {
    let old = from_str("old");
    let exception = built(&VALUE_ERROR, &ONE_OBJECT, &[old.expose_provenance() as u64]);
    // SAFETY: the string is live; the exception holds a reference to it.
    unsafe { typy_rt_increment(old.cast()) };
    let words = [word_of("new"), int_word(2)];
    // SAFETY: the exception is live and the words are live items of the layout.
    unsafe {
        super::typy_rt_exception_set_arguments(
            exception,
            &raw const TEXT_AND_INT,
            words.as_ptr().cast(),
        );
        crate::tuple::release(
            &raw const TEXT_AND_INT,
            words.as_ptr().expose_provenance() as u64,
        );
        assert_eq!((*old).header.reference_count, 1);
        typy_rt_decrement(old.cast());
    }
    assert_eq!(texts(exception).0, "('new', 2)");
    release(&[exception]);
}

#[test]
fn the_fixed_part_of_an_exception_is_eighty_bytes() {
    // The compiler's `EXCEPTION_PREFIX` places a class's attributes after this many bytes.
    assert_eq!(super::PREFIX, 80);
}
