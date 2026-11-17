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

/// The count an instance made in its function's frame starts above, so the program's
/// decrements never bring it to zero and free it; at this count no reference is left
/// (ADR-0042).
pub(crate) const FRAME_BIAS: u32 = 1 << 30;

/// Returns a new instance of `class` in `block`, a frame block of the instance's size whose
/// header the function cleared when it started, with every attribute zero. A block that holds
/// an instance no reference is left to is used again after its attributes are released; one
/// whose instance is still held gives way to a new instance from the heap (MEM-03, ADR-0042).
///
/// # Safety
///
/// `block` points to memory of the instance's size, aligned for it, holding a cleared header
/// or an instance this function made; `class` is as for `typy_rt_instance_new`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn typy_rt_instance_on_stack(
    block: *mut InstanceObject,
    class: *const TypeObject,
) -> *mut InstanceObject {
    // SAFETY: the caller's guarantee.
    unsafe {
        if (*block).header.type_id != 0 {
            if (*block).header.reference_count != FRAME_BIAS {
                return typy_rt_instance_new(class);
            }
            crate::tuple::release((*(*block).class).fields, attributes(block));
        }
        let size = (*(*class).fields).size;
        let length = PREFIX + usize::try_from(size).unwrap_or(0);
        core::ptr::write_bytes(block.cast::<u8>(), 0, length);
        block.write(InstanceObject {
            header: ObjectHeader {
                reference_count: FRAME_BIAS + 1,
                type_id: INSTANCE_TYPE_ID,
            },
            class,
        });
    }
    block
}

/// Releases the attributes of the instance a frame block holds, as its function returns, and
/// clears its header; a block that holds none is left alone. A reference still held would
/// read freed memory, so the program stops with a message instead (ADR-0042).
///
/// # Safety
///
/// `block` holds a cleared header or an instance `typy_rt_instance_on_stack` made.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn typy_rt_instance_off_stack(block: *mut InstanceObject) {
    // SAFETY: the caller's guarantee.
    unsafe {
        if (*block).header.type_id == 0 {
            return;
        }
        if (*block).header.reference_count != FRAME_BIAS {
            crate::libc::abort_with(
                b"typy runtime: an instance in a frame is still held when its function returns\n",
            );
        }
        crate::tuple::release((*(*block).class).fields, attributes(block));
        (*block).header.type_id = 0;
    }
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
    object_text(instance.cast_mut().cast(), class.module(), class.name())
}

/// Returns `<module.name object at 0x...>`, or `<name object at 0x...>` when `module` is
/// empty, for the object at `object`.
pub(crate) fn object_text(object: *mut ObjectHeader, module: &str, name: &str) -> *mut StrObject {
    let mut digits = [0_u8; 16];
    let mut address = object.expose_provenance();
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
    let dot = if module.is_empty() { "" } else { "." };
    let parts = ["<", module, dot, name, " object at 0x", hex, ">"];
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
