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
mod tests;
