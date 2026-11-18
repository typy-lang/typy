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
/// the code a call through the value runs, and the descriptor of its captures, null for none.
#[repr(C)]
#[derive(Debug)]
pub(crate) struct FunctionDescriptor {
    pub(crate) name: *const StrObject,
    pub(crate) qualified_name: *const StrObject,
    pub(crate) code: *const core::ffi::c_void,
    pub(crate) captures: *const crate::tuple::TupleType,
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
    hash: Some(crate::instance::typy_rt_identity_hash),
    repr: Some(function_repr),
    ..TypeObject::new("function", &raw const OBJECT)
};

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
    fn fun_05_a_function_object_writes_its_qualified_name_and_address() {
        let (name, qualified_name) = (from_str("area"), from_str("Shape.area"));
        let descriptor = super::FunctionDescriptor {
            name,
            qualified_name,
            code: core::ptr::null(),
            captures: core::ptr::null(),
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
