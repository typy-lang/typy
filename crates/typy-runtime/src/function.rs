//! Function objects, a function of the program as a value (FUN-05, ADR-0060). The compiler
//! emits an immortal object for each function that captures nothing, which points to the
//! function's immortal descriptor. Its `repr` is `<function QUALNAME at 0x...>`, and it hashes
//! and compares by identity, as CPython's functions do.

use crate::Raising;
use crate::header::ObjectHeader;
use crate::string::{StrBuilder, StrObject};
use crate::type_object::{OBJECT, TypeObject};

/// What the compiler emits once for each function used as a value: its names, the address of
/// the code a call through the value runs, and the descriptor of its captures, null for none.
#[repr(C)]
#[derive(Debug)]
pub(crate) struct FunctionDescriptor {
    name: *const StrObject,
    qualified_name: *const StrObject,
    code: *const core::ffi::c_void,
    captures: *const core::ffi::c_void,
}

/// A function object: the header, then its descriptor, where an instance keeps its class.
#[repr(C)]
#[derive(Debug)]
pub(crate) struct FunctionObject {
    header: ObjectHeader,
    descriptor: *const FunctionDescriptor,
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
fn push_ascii(builder: &mut StrBuilder, text: &str) {
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
