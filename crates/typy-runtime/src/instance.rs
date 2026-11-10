//! Instances of the program's classes: the header, the class's type object, then the
//! attributes, laid out as the tuple the class's type object describes (ADR-0012, CLS-01).

#[cfg(test)]
mod tests;

use crate::Raising;
use crate::header::{INSTANCE_TYPE_ID, ObjectHeader};
use crate::string::StrObject;
use crate::type_object::TypeObject;

/// The fixed part of an instance; the attributes follow it.
#[repr(C)]
#[derive(Debug)]
pub struct InstanceObject {
    /// The object header, with the instance type ID.
    pub header: ObjectHeader,
    /// The class, an immortal type object the program emits.
    pub class: *const TypeObject,
}

/// The bytes before the attributes.
const PREFIX: usize = core::mem::size_of::<InstanceObject>();

/// Returns the address of an instance's attributes.
fn attributes(instance: *mut InstanceObject) -> u64 {
    (instance.expose_provenance() + PREFIX) as u64
}

/// Returns a new instance of `class` with one reference and every attribute zero, which is
/// `None` of each optional type and holds no reference; `__init__` assigns each before the
/// program reads it (CLS-01).
///
/// # Safety
///
/// `class` points to a live type object of a class, with the descriptor of its attributes.
#[allow(
    clippy::cast_ptr_alignment,
    reason = "malloc returns memory aligned for every type"
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn typy_rt_instance_new(class: *const TypeObject) -> *mut InstanceObject {
    // SAFETY: the caller passes a class with a descriptor.
    let size = unsafe { (*(*class).fields).size };
    let length = PREFIX + usize::try_from(size).unwrap_or(0);
    let instance = crate::allocator::allocate(length).cast::<InstanceObject>();
    // SAFETY: the allocation holds `length` bytes, aligned for every type.
    unsafe {
        core::ptr::write_bytes(instance.cast::<u8>(), 0, length);
        instance.write(InstanceObject {
            header: ObjectHeader::new(INSTANCE_TYPE_ID),
            class,
        });
    }
    instance
}

/// Frees an instance whose last reference is gone, after releasing its attributes.
///
/// # Safety
///
/// `instance` points to a live instance that nothing else refers to.
pub(crate) unsafe fn destroy(instance: *mut InstanceObject) {
    // SAFETY: the caller's guarantee; the descriptor lays out the attributes.
    unsafe { crate::tuple::release((*(*instance).class).fields, attributes(instance)) };
    crate::allocator::free(instance.cast());
}

/// Returns the class of an instance.
///
/// # Safety
///
/// `instance` points to a live instance.
pub(crate) unsafe fn class_of(instance: *const InstanceObject) -> &'static TypeObject {
    // SAFETY: the caller's guarantee; a class is immortal.
    unsafe { &*(*instance).class }
}

/// Returns `object.__repr__()` of a class without one: `<module.Name object at 0x...>`.
///
/// # Safety
///
/// `object` points to a live instance.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn typy_rt_instance_repr(
    object: *mut ObjectHeader,
) -> Raising<*mut StrObject> {
    // SAFETY: the caller's guarantee.
    Raising::ok(unsafe { default_text(object.cast()) })
}

/// Returns the default text of the instance at the address `object`, for `print`, `str` and
/// `repr` of an instance whose class writes none of its own.
///
/// # Safety
///
/// `object` is the address of a live instance.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn typy_rt_instance_text(object: u64) -> *mut StrObject {
    let instance =
        core::ptr::with_exposed_provenance::<InstanceObject>(usize::try_from(object).unwrap_or(0));
    // SAFETY: the caller's guarantee.
    unsafe { default_text(instance) }
}

/// Returns `<module.Name object at 0x...>`, as CPython's `object.__repr__` writes it.
///
/// # Safety
///
/// `instance` points to a live instance.
unsafe fn default_text(instance: *const InstanceObject) -> *mut StrObject {
    // SAFETY: the caller's guarantee.
    let class = unsafe { class_of(instance) };
    let mut digits = [0_u8; 16];
    let mut address = instance.expose_provenance();
    let mut start = digits.len();
    loop {
        start -= 1;
        digits[start] = b"0123456789abcdef"[address % 16];
        address /= 16;
        if address == 0 {
            break;
        }
    }
    // SAFETY: the digits are ASCII.
    let hex = unsafe { core::str::from_utf8_unchecked(&digits[start..]) };
    let parts = [
        "<",
        class.module(),
        ".",
        class.name(),
        " object at 0x",
        hex,
        ">",
    ];
    crate::string::from_parts(&parts)
}

/// Returns the hash of an object that is equal only to itself, from its address, as CPython's
/// `object.__hash__` gives it.
///
/// # Safety
///
/// `object` points to a live object.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn typy_rt_identity_hash(object: *mut ObjectHeader) -> i64 {
    // CPython rotates the address right by four bits, since the low bits of an aligned
    // address are always zero, and keeps -1 for errors.
    let address = object.expose_provenance() as u64;
    let hash = address.rotate_right(4).cast_signed();
    if hash == -1 { -2 } else { hash }
}
