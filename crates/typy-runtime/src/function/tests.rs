//! Tests of function objects.

use crate::string::from_str;

fn text(string: *mut crate::string::StrObject) -> String {
    // SAFETY: the string is live.
    unsafe {
        (0..crate::string::length(string))
            .map(|index| char::from_u32(crate::string::code_point(string, index)).unwrap())
            .collect()
    }
}

#[test]
fn fun_04_a_closure_starts_with_its_captures_zero_and_releases_them_when_it_is_freed() {
    use crate::list::ItemKind;
    use crate::tuple::{Field, TupleType};
    let fields = [
        Field::plain(ItemKind::Object, 0),
        Field::plain(ItemKind::Int, 8),
    ];
    let captures = TupleType {
        size: 16,
        length: 2,
        fields: fields.as_ptr(),
    };
    let (name, qualified_name) = (from_str("add"), from_str("outer.<locals>.add"));
    let descriptor = super::FunctionDescriptor {
        name,
        qualified_name,
        code: core::ptr::null(),
        captures: &raw const captures,
        identity: core::ptr::null(),
    };
    let held = from_str("captured");
    // SAFETY: the descriptor, its layout and the strings live for the whole test.
    unsafe {
        let closure = super::typy_rt_closure_new(&raw const descriptor);
        let slots = core::ptr::with_exposed_provenance_mut::<u64>(
            closure.expose_provenance() + super::PREFIX,
        );
        assert_eq!([*slots, *slots.add(1)], [0, 0]);
        // The closure takes over a reference to what it captures, as `SetCapture` does.
        crate::refcount::typy_rt_increment(held.cast());
        *slots = held.expose_provenance() as u64;
        *slots.add(1) = crate::word::inline(5).unwrap().cast_unsigned();
        assert_eq!((*held).header.reference_count, 2);
        crate::refcount::typy_rt_decrement(closure.cast());
        assert_eq!((*held).header.reference_count, 1);
        for each in [held, name, qualified_name] {
            crate::refcount::typy_rt_decrement(each.cast());
        }
    }
}

#[test]
fn fun_19_adapters_of_one_nested_function_stand_for_it_and_others_do_not() {
    use crate::list::ItemKind;
    use crate::tuple::{Field, TupleType};
    let fields = [Field::plain(ItemKind::Object, 0)];
    let captures = TupleType {
        size: 8,
        length: 1,
        fields: fields.as_ptr(),
    };
    let (name, qualified_name) = (from_str("add"), from_str("outer.<locals>.add"));
    let original = super::FunctionDescriptor {
        name,
        qualified_name,
        code: core::ptr::null(),
        captures: &raw const captures,
        identity: core::ptr::null(),
    };
    let original = super::FunctionDescriptor {
        identity: &raw const original,
        ..original
    };
    // An adapter's descriptor names no identity of its own.
    let adapter = super::FunctionDescriptor {
        identity: core::ptr::null(),
        ..original
    };
    // SAFETY: the descriptors, their layout and the strings live for the whole test, and
    // each closure holds a reference to what it captures, as `SetCapture` gives one.
    unsafe {
        let first = super::typy_rt_closure_new(&raw const original);
        let second = super::typy_rt_closure_new(&raw const original);
        let wrap = |held: *mut super::FunctionObject| {
            let adapter = super::typy_rt_closure_new(&raw const adapter);
            let slot = core::ptr::with_exposed_provenance_mut::<u64>(
                adapter.expose_provenance() + super::PREFIX,
            );
            crate::refcount::typy_rt_increment(held.cast());
            *slot = held.expose_provenance() as u64;
            adapter.cast::<crate::header::ObjectHeader>()
        };
        let (left, right, other) = (wrap(first), wrap(first), wrap(second));
        assert!(super::typy_rt_same_function(left, right));
        assert!(super::typy_rt_same_function(left, first.cast()));
        assert_eq!(
            super::function_hash(left),
            super::function_hash(first.cast())
        );
        assert!(!super::typy_rt_same_function(left, other));
        assert!(!super::typy_rt_same_function(first.cast(), second.cast()));
        // An object is itself, and an object of another type is no function.
        assert!(super::typy_rt_same_function(left, left));
        assert!(!super::typy_rt_same_function(left, name.cast()));
        for each in [left, right, other, first.cast(), second.cast()] {
            crate::refcount::typy_rt_decrement(each);
        }
        for each in [name, qualified_name] {
            crate::refcount::typy_rt_decrement(each.cast());
        }
    }
}

#[test]
fn exp_22_a_value_called_with_the_wrong_count_names_the_function_as_cpython_does() {
    use crate::word::inline;
    let (name, qualified_name) = (from_str("total"), from_str("total"));
    let descriptor = super::FunctionDescriptor {
        name,
        qualified_name,
        code: core::ptr::null(),
        captures: core::ptr::null(),
        identity: core::ptr::null(),
    };
    let object = |type_id| super::FunctionObject {
        header: crate::header::ObjectHeader {
            reference_count: crate::header::IMMORTAL,
            type_id,
        },
        descriptor: &raw const descriptor,
    };
    let (function, method, builtin) = (
        object(crate::header::FUNCTION_TYPE_ID),
        object(crate::header::METHOD_TYPE_ID),
        object(crate::header::BUILTIN_FUNCTION_TYPE_ID),
    );
    let message = |value: &super::FunctionObject, expected: i64, given: i64| {
        let value = core::ptr::from_ref(value).cast_mut().cast();
        // SAFETY: the objects, their descriptor and the strings live for the whole test.
        let error = unsafe {
            super::typy_rt_value_arity(value, inline(expected).unwrap(), inline(given).unwrap())
        };
        if error.is_null() {
            return String::new();
        }
        // SAFETY: the exception is live and owned here.
        unsafe {
            let shown = crate::exception::classes::BASE_EXCEPTION.str.unwrap()(error.cast());
            let written = text(shown.value);
            crate::refcount::typy_rt_decrement(shown.value.cast());
            crate::refcount::typy_rt_decrement(error.cast());
            written
        }
    };
    assert_eq!(message(&function, 2, 2), "");
    assert_eq!(
        message(&function, 2, 3),
        "total() takes 2 positional arguments but 3 were given"
    );
    assert_eq!(
        message(&function, 0, 1),
        "total() takes 0 positional arguments but 1 was given"
    );
    assert_eq!(
        message(&function, 2, 1),
        "total() missing 1 required positional argument"
    );
    assert_eq!(
        message(&method, 1, 2),
        "total() takes 2 positional arguments but 3 were given"
    );
    assert_eq!(
        message(&builtin, 1, 2),
        "total() takes exactly one argument (2 given)"
    );
    assert_eq!(
        message(&builtin, 2, 3),
        "total() takes exactly 2 arguments (3 given)"
    );
    for each in [name, qualified_name] {
        // SAFETY: the strings are live.
        unsafe { crate::refcount::typy_rt_decrement(each.cast()) };
    }
}

#[test]
fn fun_08_a_builtin_function_writes_its_name_and_its_type() {
    let (name, qualified_name) = (from_str("len"), from_str("len"));
    let descriptor = super::FunctionDescriptor {
        name,
        qualified_name,
        code: core::ptr::null(),
        captures: core::ptr::null(),
        identity: core::ptr::null(),
    };
    let function = super::FunctionObject {
        header: crate::header::ObjectHeader {
            reference_count: crate::header::IMMORTAL,
            type_id: crate::header::BUILTIN_FUNCTION_TYPE_ID,
        },
        descriptor: &raw const descriptor,
    };
    // SAFETY: the object, its descriptor and the strings are live for the whole test.
    unsafe {
        let object = core::ptr::from_ref(&function).cast_mut().cast();
        let repr = super::builtin_function_repr(object).value;
        assert_eq!(text(repr), "<built-in function len>");
        let ty = crate::type_object::type_of(object);
        assert!(core::ptr::eq(ty, &raw const super::BUILTIN_FUNCTION));
        let class = crate::type_object::typy_rt_type_text(ty);
        assert_eq!(text(class), "<class 'builtin_function_or_method'>");
        for each in [repr, class, name, qualified_name] {
            crate::refcount::typy_rt_decrement(each.cast());
        }
    }
}

#[test]
fn fun_08_a_method_of_a_builtin_type_writes_its_name_and_its_type() {
    let (name, qualified_name) = (from_str("upper"), from_str("str.upper"));
    let descriptor = super::FunctionDescriptor {
        name,
        qualified_name,
        code: core::ptr::null(),
        captures: core::ptr::null(),
        identity: core::ptr::null(),
    };
    let function = super::FunctionObject {
        header: crate::header::ObjectHeader {
            reference_count: crate::header::IMMORTAL,
            type_id: crate::header::METHOD_DESCRIPTOR_TYPE_ID,
        },
        descriptor: &raw const descriptor,
    };
    // SAFETY: the object, its descriptor and the strings are live for the whole test.
    unsafe {
        let object = core::ptr::from_ref(&function).cast_mut().cast();
        let repr = super::method_descriptor_repr(object).value;
        assert_eq!(text(repr), "<method 'upper' of 'str' objects>");
        let class = crate::type_object::typy_rt_type_text(crate::type_object::type_of(object));
        assert_eq!(text(class), "<class 'method_descriptor'>");
        for each in [repr, class, name, qualified_name] {
            crate::refcount::typy_rt_decrement(each.cast());
        }
    }
}

#[test]
fn fun_14_a_builtin_type_as_a_function_writes_itself_as_the_type() {
    let name = from_str("int");
    let descriptor = super::FunctionDescriptor {
        name,
        qualified_name: name,
        code: core::ptr::null(),
        captures: core::ptr::null(),
        identity: core::ptr::null(),
    };
    let function = super::FunctionObject {
        header: crate::header::ObjectHeader {
            reference_count: crate::header::IMMORTAL,
            type_id: crate::header::BUILTIN_TYPE_FUNCTION_TYPE_ID,
        },
        descriptor: &raw const descriptor,
    };
    // SAFETY: the object, its descriptor and the string are live for the whole test.
    unsafe {
        let object = core::ptr::from_ref(&function).cast_mut().cast();
        let repr = super::builtin_type_repr(object).value;
        assert_eq!(text(repr), "<class 'int'>");
        let class = crate::type_object::typy_rt_type_text(crate::type_object::type_of(object));
        assert_eq!(text(class), "<class 'type'>");
        for each in [repr, class, name] {
            crate::refcount::typy_rt_decrement(each.cast());
        }
    }
}

#[test]
fn fun_08_function_values_are_one_function_when_they_name_one_identity() {
    let (name, qualified_name) = (from_str("len"), from_str("len"));
    let first = super::FunctionDescriptor {
        name,
        qualified_name,
        code: core::ptr::null(),
        captures: core::ptr::null(),
        identity: core::ptr::null(),
    };
    let shared = &raw const first;
    let second = super::FunctionDescriptor {
        identity: shared,
        ..first
    };
    let first = super::FunctionDescriptor {
        identity: shared,
        ..first
    };
    let object = |descriptor: &super::FunctionDescriptor| super::FunctionObject {
        header: crate::header::ObjectHeader {
            reference_count: crate::header::IMMORTAL,
            type_id: crate::header::BUILTIN_FUNCTION_TYPE_ID,
        },
        descriptor: core::ptr::from_ref(descriptor),
    };
    let (left, right) = (object(&first), object(&second));
    let other_descriptor = super::FunctionDescriptor {
        identity: core::ptr::null(),
        ..first
    };
    let other = object(&other_descriptor);
    // SAFETY: the objects, their descriptors and the strings live for the whole test.
    unsafe {
        let left = core::ptr::from_ref(&left).cast_mut().cast();
        let right = core::ptr::from_ref(&right).cast_mut().cast();
        let other = core::ptr::from_ref(&other).cast_mut().cast();
        assert!(super::typy_rt_same_function(left, right));
        assert!(super::function_equal(left, right));
        assert_eq!(super::function_hash(left), super::function_hash(right));
        assert!(!super::typy_rt_same_function(left, other));
        for each in [name, qualified_name] {
            crate::refcount::typy_rt_decrement(each.cast());
        }
    }
}

#[test]
fn fun_05_a_function_object_writes_its_qualified_name_and_address() {
    let (name, qualified_name) = (from_str("area"), from_str("Shape.area"));
    let descriptor = super::FunctionDescriptor {
        name,
        qualified_name,
        code: core::ptr::null(),
        captures: core::ptr::null(),
        identity: core::ptr::null(),
    };
    let function = super::FunctionObject {
        header: crate::header::ObjectHeader {
            reference_count: crate::header::IMMORTAL,
            type_id: crate::header::FUNCTION_TYPE_ID,
        },
        descriptor: &raw const descriptor,
    };
    // SAFETY: the object, its descriptor and the strings are live for the whole test.
    unsafe {
        let object = core::ptr::from_ref(&function).cast_mut().cast();
        let repr = super::function_repr(object).value;
        let address = object.expose_provenance();
        assert_eq!(text(repr), format!("<function Shape.area at {address:#x}>"));
        let ty = crate::type_object::type_of(object);
        assert!(core::ptr::eq(ty, &raw const super::FUNCTION));
        let class = crate::type_object::typy_rt_type_text(ty);
        assert_eq!(text(class), "<class 'function'>");
        let hash = crate::instance::typy_rt_identity_hash(object);
        assert_eq!(hash, crate::instance::typy_rt_identity_hash(object));
        for each in [repr, class, name, qualified_name] {
            crate::refcount::typy_rt_decrement(each.cast());
        }
    }
}
