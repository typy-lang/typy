//! Bound methods (FUN-05, ADR-0060): a method read through an instance, which the program
//! makes each time it reads one. It holds the descriptor of its method and a reference to the
//! instance. Two are equal when they bind one instance to one method, and the hash combines
//! the two, so `a.m == a.m` holds while `a.m is a.m` does not, as in CPython. Its `repr` is
//! `<bound method QUALNAME of REPR>`, the instance written as its class writes it.

use crate::Raising;
use crate::function::{FunctionDescriptor, push_ascii};
use crate::header::{METHOD_TYPE_ID, ObjectHeader};
use crate::string::{StrBuilder, StrObject};
use crate::type_object::{OBJECT, TypeObject};

/// A bound method: a function object whose one capture is its instance.
#[repr(C)]
#[derive(Debug)]
pub(crate) struct MethodObject {
    header: ObjectHeader,
    descriptor: *const FunctionDescriptor,
    instance: *mut ObjectHeader,
}

/// The type `method`.
#[unsafe(export_name = "typy_rt_type_method")]
pub(crate) static METHOD: TypeObject = TypeObject {
    hash: Some(method_hash),
    equal: Some(method_equal),
    repr: Some(method_repr),
    ..TypeObject::new("method", &raw const OBJECT)
};

/// Returns a new bound method of the method whose descriptor is at `descriptor` and the
/// instance `instance`, given as words, with a reference of its own to the instance.
///
/// # Safety
///
/// `descriptor` is the address of a descriptor the compiler emitted, and `instance` the
/// address of a live object.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn typy_rt_method_new(descriptor: u64, instance: u64) -> *mut MethodObject {
    // SAFETY: the caller's guarantee.
    unsafe { new_method(descriptor, instance, METHOD_TYPE_ID) }
}

/// Returns the value a method read through `instance` gives when the instance's class chose
/// `chosen`: with `binds`, a new bound method of the method whose descriptor `chosen` is
/// address of; without, the function object at `chosen`, a static method's, with a reference
/// of its own (FUN-09).
///
/// # Safety
///
/// `chosen` is the address of a descriptor the compiler emitted when `binds` holds, and of a
/// live function object when it does not; `instance` is the address of a live object.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn typy_rt_method_or_function(
    chosen: u64,
    instance: u64,
    binds: bool,
) -> *mut ObjectHeader {
    if binds {
        // SAFETY: the caller's guarantee.
        return unsafe { new_method(chosen, instance, METHOD_TYPE_ID) }.cast();
    }
    let function = core::ptr::with_exposed_provenance_mut::<ObjectHeader>(
        usize::try_from(chosen).unwrap_or(0),
    );
    // SAFETY: the caller's guarantee.
    unsafe { crate::refcount::typy_rt_increment(function) };
    function
}

/// Returns a new method of a built-in type read through the value `instance`, such as
/// `words.append`, whose hidden function's descriptor is at `descriptor`, with a reference of
/// its own to the value (FUN-08, ADR-0077).
///
/// # Safety
///
/// `descriptor` is the address of a descriptor the compiler emitted, and `instance` the
/// address of a live object.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn typy_rt_builtin_method_new(
    descriptor: u64,
    instance: u64,
) -> *mut MethodObject {
    // SAFETY: the caller's guarantee.
    unsafe { new_method(descriptor, instance, crate::header::BUILTIN_METHOD_TYPE_ID) }
}

/// Returns a new bound method of the type ID `type_id`.
///
/// # Safety
///
/// As for `typy_rt_method_new`.
unsafe fn new_method(descriptor: u64, instance: u64, type_id: u32) -> *mut MethodObject {
    let descriptor = core::ptr::with_exposed_provenance::<FunctionDescriptor>(
        usize::try_from(descriptor).unwrap_or(0),
    );
    let instance = core::ptr::with_exposed_provenance_mut::<ObjectHeader>(
        usize::try_from(instance).unwrap_or(0),
    );
    // SAFETY: the caller's guarantee.
    unsafe { crate::refcount::typy_rt_increment(instance) };
    #[allow(
        clippy::cast_ptr_alignment,
        reason = "malloc returns memory aligned for every type"
    )]
    let method =
        crate::allocator::allocate(core::mem::size_of::<MethodObject>()).cast::<MethodObject>();
    // SAFETY: the allocation holds a bound method.
    unsafe {
        method.write(MethodObject {
            header: ObjectHeader::new(type_id),
            descriptor,
            instance,
        });
    }
    method
}

/// Frees a bound method after releasing its instance.
///
/// # Safety
///
/// `method` is a bound method whose last reference is gone.
pub(crate) unsafe fn destroy(method: *mut MethodObject) {
    // SAFETY: the caller's guarantee; the method owns a reference to its instance.
    unsafe { crate::refcount::typy_rt_decrement((*method).instance) };
    crate::allocator::free(method.cast());
}

/// Returns the descriptor and the instance of the bound method at `object`.
///
/// # Safety
///
/// `object` points to a live bound method.
#[allow(
    clippy::cast_ptr_alignment,
    reason = "a bound method comes from malloc, aligned for every type"
)]
unsafe fn parts(object: *mut ObjectHeader) -> (*const FunctionDescriptor, *mut ObjectHeader) {
    // SAFETY: the caller's guarantee.
    unsafe {
        let method = object.cast::<MethodObject>();
        ((*method).descriptor, (*method).instance)
    }
}

/// Returns the identity of the function a bound method binds, and its instance (ADR-0076).
///
/// # Safety
///
/// `object` points to a live bound method.
unsafe fn identity_parts(
    object: *mut ObjectHeader,
) -> (*const FunctionDescriptor, *mut ObjectHeader) {
    // SAFETY: the caller's guarantee; a descriptor is an immortal static.
    unsafe {
        let (descriptor, instance) = parts(object);
        let identity = (*descriptor).identity;
        (
            if identity.is_null() {
                descriptor
            } else {
                identity
            },
            instance,
        )
    }
}

/// Returns true when `right` is a bound method of the same kind that binds the same instance
/// to the same function as `left`.
pub(crate) unsafe extern "C" fn method_equal(
    left: *mut ObjectHeader,
    right: *mut ObjectHeader,
) -> bool {
    // SAFETY: the type of `left` says it is a bound method, and that of `right` is read.
    unsafe { (*right).type_id == (*left).type_id && identity_parts(left) == identity_parts(right) }
}

/// Returns the hash of a bound method, from the addresses of its instance and of the function
/// it binds, as CPython combines the identity of `__self__` with the hash of `__func__`.
pub(crate) unsafe extern "C" fn method_hash(object: *mut ObjectHeader) -> i64 {
    // SAFETY: the type of the object says it is a bound method.
    let (descriptor, instance) = unsafe { identity_parts(object) };
    // SAFETY: both are live: the instance is held, and the descriptor is a static.
    let (instance, function) = unsafe {
        (
            crate::instance::typy_rt_identity_hash(instance),
            crate::instance::typy_rt_identity_hash(descriptor.cast_mut().cast()),
        )
    };
    let hash = instance ^ function;
    if hash == -1 { -2 } else { hash }
}

/// Returns `repr` of a method of a built-in type read through a value:
/// `<built-in method append of list object at 0x...>`, as CPython writes it.
pub(crate) unsafe extern "C" fn builtin_method_repr(
    object: *mut ObjectHeader,
) -> Raising<*mut StrObject> {
    // SAFETY: the type of the object says it is a bound method.
    let (descriptor, instance) = unsafe { parts(object) };
    let mut digits = [0_u8; 16];
    let address = crate::instance::address_digits(instance, &mut digits);
    let mut text = StrBuilder::with_capacity(48);
    push_ascii(&mut text, "<built-in method ");
    // SAFETY: the descriptor and its name are immortal statics, and the instance is live
    // while the method holds it.
    unsafe {
        text.push_str((*descriptor).name);
        push_ascii(&mut text, " of ");
        push_ascii(&mut text, crate::type_object::type_of(instance).name());
    }
    push_ascii(&mut text, " object at 0x");
    push_ascii(&mut text, address);
    push_ascii(&mut text, ">");
    Raising::ok(text.finish())
}

/// Returns `repr` of a bound method: `<bound method QUALNAME of REPR>`, or the exception the
/// instance's `__repr__` raised.
unsafe extern "C" fn method_repr(object: *mut ObjectHeader) -> Raising<*mut StrObject> {
    // SAFETY: the type of the object says it is a bound method.
    let (descriptor, instance) = unsafe { parts(object) };
    // SAFETY: the instance is live while the method holds it.
    let written = unsafe { crate::type_object::typy_rt_object_repr(instance) };
    if !written.exception.is_null() {
        return Raising::error(core::ptr::null_mut(), written.exception);
    }
    let mut text = StrBuilder::with_capacity(48);
    push_ascii(&mut text, "<bound method ");
    // SAFETY: the descriptor and its names are immortal statics, and the instance's text is
    // live until it is released below.
    unsafe {
        text.push_str((*descriptor).qualified_name);
        push_ascii(&mut text, " of ");
        text.push_str(written.value);
        crate::refcount::typy_rt_decrement(written.value.cast());
    }
    push_ascii(&mut text, ">");
    Raising::ok(text.finish())
}

#[cfg(test)]
mod tests {
    use crate::function::FunctionDescriptor;
    use crate::string::from_str;

    fn text(string: *mut crate::string::StrObject) -> String {
        // SAFETY: the string is live.
        unsafe {
            (0..crate::string::length(string))
                .map(|index| char::from_u32(crate::string::code_point(string, index)).unwrap())
                .collect()
        }
    }

    fn word<T>(pointer: *const T) -> u64 {
        pointer.expose_provenance() as u64
    }

    #[test]
    fn fun_05_a_bound_method_is_equal_to_one_of_its_instance_and_method_only() {
        let names = [from_str("area"), from_str("Shape.area")];
        let descriptor = |code: usize| FunctionDescriptor {
            name: names[0],
            qualified_name: names[1],
            code: core::ptr::without_provenance(code),
            captures: core::ptr::null(),
            identity: core::ptr::null(),
        };
        let (first, second) = (descriptor(1), descriptor(2));
        let (left, right) = (from_str("left"), from_str("right"));
        // SAFETY: the descriptors and the strings stand for instances and live for the
        // whole test; each method holds its instance until it is released.
        unsafe {
            let bound = super::typy_rt_method_new(word(&raw const first), word(left));
            let again = super::typy_rt_method_new(word(&raw const first), word(left));
            let other = super::typy_rt_method_new(word(&raw const first), word(right));
            let second = super::typy_rt_method_new(word(&raw const second), word(left));
            let [bound, again, other, second] = [bound, again, other, second]
                .map(<*mut super::MethodObject>::cast::<crate::header::ObjectHeader>);
            assert!(super::method_equal(bound, again));
            assert!(!super::method_equal(bound, other));
            assert!(!super::method_equal(bound, second));
            assert!(!super::method_equal(bound, left.cast()));
            assert_eq!(super::method_hash(bound), super::method_hash(again));
            assert_ne!(super::method_hash(bound), super::method_hash(other));
            let ty = crate::type_object::type_of(bound);
            assert_eq!(
                text(crate::type_object::typy_rt_type_text(ty)),
                "<class 'method'>"
            );
            // A `str` instance writes itself with quotes.
            let repr = super::method_repr(bound).value;
            assert_eq!(text(repr), "<bound method Shape.area of 'left'>");
            crate::refcount::typy_rt_decrement(repr.cast());
            for each in [bound, again, other, second] {
                crate::refcount::typy_rt_decrement(each);
            }
            for each in [left, right, names[0], names[1]] {
                assert_eq!((*each).header.reference_count, 1);
                crate::refcount::typy_rt_decrement(each.cast());
            }
        }
    }
    #[test]
    fn fun_08_a_builtin_method_binds_its_value_and_compares_by_the_function_it_stands_for() {
        let names = [from_str("append"), from_str("list.append")];
        let first = FunctionDescriptor {
            name: names[0],
            qualified_name: names[1],
            code: core::ptr::without_provenance(1),
            captures: core::ptr::null(),
            identity: core::ptr::null(),
        };
        // A second hidden function of `list.append` stands for the first.
        let second = FunctionDescriptor {
            code: core::ptr::without_provenance(2),
            identity: &raw const first,
            ..first
        };
        let first = FunctionDescriptor {
            identity: &raw const first,
            ..first
        };
        let value = from_str("held");
        // SAFETY: the descriptors and the strings live for the whole test; each method holds
        // its value until it is released.
        unsafe {
            let bound = super::typy_rt_builtin_method_new(word(&raw const first), word(value));
            let other = super::typy_rt_builtin_method_new(word(&raw const second), word(value));
            let plain = super::typy_rt_method_new(word(&raw const first), word(value));
            let [bound, other, plain] = [bound, other, plain]
                .map(<*mut super::MethodObject>::cast::<crate::header::ObjectHeader>);
            assert!(super::method_equal(bound, other));
            assert_eq!(super::method_hash(bound), super::method_hash(other));
            assert!(!super::method_equal(bound, plain));
            let ty = crate::type_object::type_of(bound);
            assert_eq!(
                text(crate::type_object::typy_rt_type_text(ty)),
                "<class 'builtin_function_or_method'>"
            );
            let repr = super::builtin_method_repr(bound).value;
            let address = value.expose_provenance();
            assert_eq!(
                text(repr),
                format!("<built-in method append of str object at {address:#x}>")
            );
            crate::refcount::typy_rt_decrement(repr.cast());
            for each in [bound, other, plain] {
                crate::refcount::typy_rt_decrement(each);
            }
            assert_eq!((*value).header.reference_count, 1);
            for each in [value, names[0], names[1]] {
                crate::refcount::typy_rt_decrement(each.cast());
            }
        }
    }

    #[test]
    fn fun_09_a_chosen_method_binds_the_instance_and_a_chosen_function_is_itself() {
        let names = [from_str("sides"), from_str("Shape.sides")];
        let method = FunctionDescriptor {
            name: names[0],
            qualified_name: names[1],
            code: core::ptr::without_provenance(1),
            captures: core::ptr::null(),
            identity: core::ptr::null(),
        };
        // A `str` stands for the static method's function object, which is returned held.
        let (instance, function) = (from_str("shape"), from_str("function"));
        // SAFETY: the descriptor and the strings live for the whole test; the bound method
        // holds its instance until it is released.
        unsafe {
            let bound =
                super::typy_rt_method_or_function(word(&raw const method), word(instance), true);
            assert_eq!(
                (*bound).type_id,
                crate::header::METHOD_TYPE_ID,
                "a method binds the instance"
            );
            assert_eq!((*instance).header.reference_count, 2);
            let chosen = super::typy_rt_method_or_function(word(function), word(instance), false);
            assert_eq!(chosen, function.cast());
            assert_eq!((*function).header.reference_count, 2);
            assert_eq!((*instance).header.reference_count, 2);
            for each in [bound, chosen] {
                crate::refcount::typy_rt_decrement(each);
            }
            for each in [instance, function, names[0], names[1]] {
                assert_eq!((*each).header.reference_count, 1);
                crate::refcount::typy_rt_decrement(each.cast());
            }
        }
    }
}
