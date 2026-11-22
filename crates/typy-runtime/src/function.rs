//! Function objects, a function of the program as a value (FUN-05, ADR-0060). The compiler
//! emits an immortal object for each function of a module, which points to the function's
//! immortal descriptor; each `def` of a nested function that runs makes a new one, which
//! holds its captures after the descriptor, laid out as the descriptor's tuple type says
//! (FUN-04, ADR-0061). Its `repr` is `<function QUALNAME at 0x...>`, and it hashes and
//! compares by identity, as CPython's functions do.

use crate::Raising;
use crate::header::ObjectHeader;
use crate::string::{StrBuilder, StrObject};
use crate::type_object::{OBJECT, TypeObject};

/// What the compiler emits once for each function used as a value: its names, the address of
/// the code a call through the value runs, the descriptor of its captures, null for none, and
/// the descriptor of the function it stands for, its own address for a function of the
/// program and one shared address for the hidden functions of one built-in (ADR-0076). The
/// adapter of a nested function or a `lambda` has none, null: it stands for the function
/// object it captures first (FUN-19, ADR-0080).
#[repr(C)]
#[derive(Debug)]
pub(crate) struct FunctionDescriptor {
    pub(crate) name: *const StrObject,
    pub(crate) qualified_name: *const StrObject,
    pub(crate) code: *const core::ffi::c_void,
    pub(crate) captures: *const crate::tuple::TupleType,
    pub(crate) identity: *const FunctionDescriptor,
}

/// A function object: the header, then its descriptor, where an instance keeps its class.
#[repr(C)]
#[derive(Debug)]
pub(crate) struct FunctionObject {
    header: ObjectHeader,
    descriptor: *const FunctionDescriptor,
}

/// The bytes before a function object's captures: its header and its descriptor.
const PREFIX: usize = core::mem::size_of::<FunctionObject>();

/// Returns a new function object of the nested function whose descriptor is `descriptor`,
/// with one reference and every capture zero, which the program then stores (FUN-04).
///
/// # Safety
///
/// `descriptor` is a descriptor the compiler emitted for a nested function, whose captures
/// it describes.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn typy_rt_closure_new(
    descriptor: *const FunctionDescriptor,
) -> *mut FunctionObject {
    // SAFETY: the caller's guarantee.
    let size = unsafe { (*(*descriptor).captures).size };
    let length = PREFIX + usize::try_from(size).unwrap_or(0);
    #[allow(
        clippy::cast_ptr_alignment,
        reason = "malloc returns memory aligned for every type"
    )]
    let function = crate::allocator::allocate(length).cast::<FunctionObject>();
    // SAFETY: the allocation holds `length` bytes, aligned for every type.
    unsafe {
        core::ptr::write_bytes(function.cast::<u8>(), 0, length);
        function.write(FunctionObject {
            header: ObjectHeader::new(crate::header::FUNCTION_TYPE_ID),
            descriptor,
        });
    }
    function
}

/// Frees a nested function's object after releasing each capture its descriptor counts.
///
/// # Safety
///
/// `function` is a function object a `def` made whose last reference is gone.
pub(crate) unsafe fn destroy(function: *mut FunctionObject) {
    // SAFETY: the caller's guarantee; the descriptor lays out the captures after the prefix.
    unsafe {
        let captures = (*(*function).descriptor).captures;
        crate::tuple::release(captures, (function.expose_provenance() + PREFIX) as u64);
    }
    crate::allocator::free(function.cast());
}

/// The type `function`.
#[unsafe(export_name = "typy_rt_type_function")]
pub(crate) static FUNCTION: TypeObject = TypeObject {
    hash: Some(function_hash),
    equal: Some(function_equal),
    repr: Some(function_repr),
    ..TypeObject::new("function", &raw const OBJECT)
};

/// Returns the address that names the function `object` stands for: the identity in the
/// descriptor of the immortal object of a function that captures nothing, what the first
/// capture of a nested function's adapter stands for, and the object itself for any other
/// object, such as a nested function's, made at each `def` (ADR-0076, ADR-0080).
///
/// # Safety
///
/// `object` points to a live object with a valid header.
#[allow(
    clippy::cast_ptr_alignment,
    reason = "a function object is aligned for its pointers"
)]
unsafe fn identity_of(object: *mut ObjectHeader) -> *const core::ffi::c_void {
    // SAFETY: the caller's guarantee; the type ID says the object is a function object,
    // whose descriptor is an immortal static, and an adapter's first capture is a live
    // function object it holds a reference to.
    unsafe {
        if !matches!(
            (*object).type_id,
            crate::header::FUNCTION_TYPE_ID
                | crate::header::BUILTIN_FUNCTION_TYPE_ID
                | crate::header::METHOD_DESCRIPTOR_TYPE_ID
                | crate::header::BUILTIN_TYPE_FUNCTION_TYPE_ID
        ) {
            return object.cast_const().cast();
        }
        let descriptor = (*object.cast::<FunctionObject>()).descriptor;
        let captures = (*descriptor).captures;
        if captures.is_null() {
            return (*descriptor).identity.cast();
        }
        if (*descriptor).identity.is_null() {
            let address = (object.expose_provenance() + PREFIX) as u64;
            let original = crate::tuple::field(captures, address, 0);
            let original = core::ptr::with_exposed_provenance_mut::<ObjectHeader>(
                usize::try_from(original).unwrap_or(0),
            );
            return identity_of(original);
        }
        object.cast_const().cast()
    }
}

/// Returns true when the function values `left` and `right` stand for one function: the same
/// object, or two that name one identity, as `len` filling two `Callable` types does
/// (ADR-0076), or two adapters of one nested function (ADR-0080).
///
/// # Safety
///
/// `left` and `right` point to live objects with valid headers.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn typy_rt_same_function(
    left: *mut ObjectHeader,
    right: *mut ObjectHeader,
) -> bool {
    // SAFETY: the caller's guarantee.
    core::ptr::eq(left, right) || unsafe { core::ptr::eq(identity_of(left), identity_of(right)) }
}

/// Returns CPython's `TypeError` when a call through the function value `function` with `*`
/// gives `given` positional values, an `int` word, and its `Callable` takes `expected`, an
/// `int` word, naming the function by its qualified name; null when they are equal (EXP-22,
/// ADR-0083). A bound method counts its instance, as CPython does, and a built-in writes the
/// count as a built-in does.
///
/// # Safety
///
/// `function` points to a live function object, bound method or built-in function value.
#[unsafe(no_mangle)]
#[allow(
    clippy::cast_ptr_alignment,
    reason = "a function object and a bound method keep their descriptor after the header"
)]
pub unsafe extern "C" fn typy_rt_value_arity(
    function: *mut ObjectHeader,
    expected: i64,
    given: i64,
) -> *mut crate::ExceptionObject {
    let (expected, given) = (
        crate::word::inline_value(expected),
        crate::word::inline_value(given),
    );
    if expected == given {
        return core::ptr::null_mut();
    }
    let mut message = crate::message::MessageBuffer::new();
    // SAFETY: the caller's guarantee; a bound method, as a function object, keeps its
    // descriptor right after its header, and the descriptor's names are immortal strings.
    let kind = unsafe {
        let descriptor = (*function.cast::<FunctionObject>()).descriptor;
        message.push_text((*descriptor).qualified_name);
        (*function).type_id
    };
    message.push_str("() ");
    let builtin = matches!(
        kind,
        crate::header::BUILTIN_FUNCTION_TYPE_ID
            | crate::header::BUILTIN_METHOD_TYPE_ID
            | crate::header::METHOD_DESCRIPTOR_TYPE_ID
            | crate::header::BUILTIN_TYPE_FUNCTION_TYPE_ID
    );
    let plural = |count: i64| if count == 1 { "" } else { "s" };
    if builtin {
        message.push_str("takes exactly ");
        if expected == 1 {
            message.push_str("one");
        } else {
            message.push_number(expected);
        }
        message.push_str(" argument");
        message.push_str(plural(expected));
        message.push_str(" (");
        message.push_number(given);
        message.push_str(" given)");
    } else if given > expected {
        let instance = i64::from(kind == crate::header::METHOD_TYPE_ID);
        message.push_str("takes ");
        message.push_number(expected + instance);
        message.push_str(" positional argument");
        message.push_str(plural(expected + instance));
        message.push_str(" but ");
        message.push_number(given + instance);
        message.push_str(if given + instance == 1 {
            " was"
        } else {
            " were"
        });
        message.push_str(" given");
    } else {
        message.push_str("missing ");
        message.push_number(expected - given);
        message.push_str(" required positional argument");
        message.push_str(plural(expected - given));
    }
    message.raise(crate::exception::ExceptionKind::TypeError)
}

/// Returns true when `right` stands for the function `left` stands for.
unsafe extern "C" fn function_equal(left: *mut ObjectHeader, right: *mut ObjectHeader) -> bool {
    // SAFETY: the slot is called with two live objects.
    unsafe { typy_rt_same_function(left, right) }
}

/// Returns the hash of a function value: that of the identity it stands for, so values that
/// compare equal hash alike.
unsafe extern "C" fn function_hash(object: *mut ObjectHeader) -> i64 {
    // SAFETY: the slot is called with a live object, and only the identity's address is
    // hashed.
    unsafe { crate::instance::typy_rt_identity_hash(identity_of(object).cast_mut().cast()) }
}

/// The type `builtin_function_or_method`, of a built-in function used as a value, whose
/// object the compiler emits immortal (FUN-08, ADR-0077).
#[unsafe(export_name = "typy_rt_type_builtin_function")]
pub(crate) static BUILTIN_FUNCTION: TypeObject = TypeObject {
    hash: Some(builtin_hash),
    equal: Some(builtin_equal),
    repr: Some(builtin_repr),
    ..TypeObject::new("builtin_function_or_method", &raw const OBJECT)
};

/// Returns true when `object` is a method of a built-in type read through a value, laid out
/// as a bound method, rather than a built-in function.
fn is_bound(object: *mut ObjectHeader) -> bool {
    // SAFETY: the slots of the type are called with live objects.
    unsafe { (*object).type_id == crate::header::BUILTIN_METHOD_TYPE_ID }
}

/// Returns the hash of a `builtin_function_or_method`: as a function, or as a bound method.
unsafe extern "C" fn builtin_hash(object: *mut ObjectHeader) -> i64 {
    // SAFETY: the slot is called with a live object of the type.
    unsafe {
        if is_bound(object) {
            crate::method::method_hash(object)
        } else {
            function_hash(object)
        }
    }
}

/// Returns true when `right` equals the `builtin_function_or_method` `left`.
unsafe extern "C" fn builtin_equal(left: *mut ObjectHeader, right: *mut ObjectHeader) -> bool {
    // SAFETY: the slot is called with live objects.
    unsafe {
        if is_bound(left) {
            crate::method::method_equal(left, right)
        } else {
            function_equal(left, right)
        }
    }
}

/// Returns `repr` of a `builtin_function_or_method`: a built-in function's or a bound one's.
unsafe extern "C" fn builtin_repr(object: *mut ObjectHeader) -> Raising<*mut StrObject> {
    // SAFETY: the slot is called with a live object of the type.
    unsafe {
        if is_bound(object) {
            crate::method::builtin_method_repr(object)
        } else {
            builtin_function_repr(object)
        }
    }
}

/// Returns `repr` of a built-in function: `<built-in function NAME>`, as CPython writes it.
#[allow(
    clippy::cast_ptr_alignment,
    reason = "a function object is a static the compiler aligns for its pointers"
)]
unsafe extern "C" fn builtin_function_repr(object: *mut ObjectHeader) -> Raising<*mut StrObject> {
    let mut text = StrBuilder::with_capacity(32);
    push_ascii(&mut text, "<built-in function ");
    // SAFETY: the type of the object says it is a function object, whose descriptor and
    // names are immortal statics.
    unsafe { text.push_str((*(*object.cast::<FunctionObject>()).descriptor).name) };
    push_ascii(&mut text, ">");
    Raising::ok(text.finish())
}

/// The type `method_descriptor`, of a method of a built-in type read through the type, such as
/// `str.upper`, whose object the compiler emits immortal (FUN-08, ADR-0077).
#[unsafe(export_name = "typy_rt_type_method_descriptor")]
pub(crate) static METHOD_DESCRIPTOR: TypeObject = TypeObject {
    hash: Some(function_hash),
    equal: Some(function_equal),
    repr: Some(method_descriptor_repr),
    ..TypeObject::new("method_descriptor", &raw const OBJECT)
};

/// Returns `repr` of a method of a built-in type: `<method 'upper' of 'str' objects>`, from its
/// qualified name, `str.upper`, as CPython writes it.
#[allow(
    clippy::cast_ptr_alignment,
    reason = "a function object is a static the compiler aligns for its pointers"
)]
unsafe extern "C" fn method_descriptor_repr(object: *mut ObjectHeader) -> Raising<*mut StrObject> {
    let mut text = StrBuilder::with_capacity(48);
    push_ascii(&mut text, "<method '");
    // SAFETY: the type of the object says it is a function object, whose descriptor and
    // names are immortal statics.
    unsafe {
        let descriptor = (*object.cast::<FunctionObject>()).descriptor;
        text.push_str((*descriptor).name);
        push_ascii(&mut text, "' of '");
        let qualified = (*descriptor).qualified_name;
        let dot = u32::from(b'.');
        for index in 0..crate::string::length(qualified) {
            let point = crate::string::code_point(qualified, index);
            if point == dot {
                break;
            }
            text.push(point);
        }
    }
    push_ascii(&mut text, "' objects>");
    Raising::ok(text.finish())
}

/// The type of a built-in type called as a function, such as `int` in `key=int`, written as
/// the type is and of the type `type`, as CPython's value is the type itself (FUN-14).
#[unsafe(export_name = "typy_rt_type_builtin_type_function")]
pub(crate) static BUILTIN_TYPE_FUNCTION: TypeObject = TypeObject {
    hash: Some(function_hash),
    equal: Some(function_equal),
    repr: Some(builtin_type_repr),
    ..TypeObject::new("type", &raw const OBJECT)
};

/// Returns `repr` of a built-in type called as a function: `<class 'int'>`.
#[allow(
    clippy::cast_ptr_alignment,
    reason = "a function object is a static the compiler aligns for its pointers"
)]
unsafe extern "C" fn builtin_type_repr(object: *mut ObjectHeader) -> Raising<*mut StrObject> {
    let mut text = StrBuilder::with_capacity(24);
    push_ascii(&mut text, "<class '");
    // SAFETY: the type of the object says it is a function object, whose descriptor and
    // names are immortal statics.
    unsafe { text.push_str((*(*object.cast::<FunctionObject>()).descriptor).name) };
    push_ascii(&mut text, "'>");
    Raising::ok(text.finish())
}

/// Returns `repr` of a function object: `<function QUALNAME at 0x...>`, with the object's
/// address, as CPython writes it.
#[allow(
    clippy::cast_ptr_alignment,
    reason = "a function object is a static the compiler aligns for its pointers"
)]
unsafe extern "C" fn function_repr(object: *mut ObjectHeader) -> Raising<*mut StrObject> {
    let mut digits = [0_u8; 16];
    let address = crate::instance::address_digits(object, &mut digits);
    let mut text = StrBuilder::with_capacity(32);
    push_ascii(&mut text, "<function ");
    // SAFETY: the type of the object says it is a function object, whose descriptor and
    // names are immortal statics.
    unsafe { text.push_str((*(*object.cast::<FunctionObject>()).descriptor).qualified_name) };
    push_ascii(&mut text, " at 0x");
    push_ascii(&mut text, address);
    push_ascii(&mut text, ">");
    Raising::ok(text.finish())
}

/// Appends the characters of `text`, which is ASCII.
pub(crate) fn push_ascii(builder: &mut StrBuilder, text: &str) {
    for byte in text.bytes() {
        builder.push(u32::from(byte));
    }
}

#[cfg(test)]
mod tests {
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
}
