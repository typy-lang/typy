//! Tests of instances: making, freeing, typing, writing and hashing them.

use super::{
    typy_rt_identity_hash, typy_rt_instance_new, typy_rt_instance_repr, typy_rt_instance_text,
};
use crate::list::ItemKind;
use crate::tuple::{Field, TupleType};
use crate::type_object::{OBJECT, TypeObject, type_of};

/// Returns the type object of a class `Box` in `__main__`, whose instances hold one object
/// and one `int`.
fn class() -> &'static TypeObject {
    let fields = Box::leak(Box::new([
        Field {
            kind: ItemKind::Object as u32,
            offset: 0,
            tuple: core::ptr::null(),
        },
        Field {
            kind: ItemKind::Int as u32,
            offset: 8,
            tuple: core::ptr::null(),
        },
    ]));
    let layout = Box::leak(Box::new(TupleType {
        size: 16,
        length: 2,
        fields: fields.as_ptr(),
    }));
    let mut class = TypeObject::new("Box", &raw const OBJECT);
    class.module = "__main__".as_ptr();
    class.module_length = 8;
    class.fields = layout;
    Box::leak(Box::new(class))
}

/// Returns the text of a runtime string.
fn text(string: *mut crate::string::StrObject) -> String {
    use crate::string::{code_point, length};
    // SAFETY: the runtime returns a live string, and every index is in bounds.
    let text = (0..unsafe { length(string) })
        .map(|index| char::from_u32(unsafe { code_point(string, index) }).unwrap_or('?'))
        .collect();
    // SAFETY: the string is no longer used.
    unsafe { crate::refcount::typy_rt_decrement(string.cast()) };
    text
}

#[test]
fn a_new_instance_is_zero_and_frees_what_it_holds() {
    let class = class();
    // SAFETY: the class has a descriptor of its attributes, and each value stays live until
    // its last reference goes.
    unsafe {
        let instance = typy_rt_instance_new(class);
        // The attributes follow the fixed part, which is a multiple of eight bytes.
        let attributes = instance.add(1).cast::<u64>();
        assert_eq!([attributes.read(), attributes.add(1).read()], [0, 0]);
        assert!(core::ptr::eq(type_of(instance.cast()), class));
        let held = crate::string::from_str("held");
        crate::refcount::typy_rt_increment(held.cast());
        // The runtime reads an attribute as an address, so the string's address is exposed,
        // as the addresses generated code stores are.
        attributes.write(held.expose_provenance() as u64);
        crate::refcount::typy_rt_decrement(instance.cast());
        assert_eq!((*held).header.reference_count, 1);
        crate::refcount::typy_rt_decrement(held.cast());
    }
}

#[test]
fn an_instance_is_written_as_cpython_writes_it() {
    let class = class();
    // SAFETY: the class has a descriptor, and the instance is freed at the end.
    unsafe {
        let instance = typy_rt_instance_new(class);
        let address = instance.expose_provenance();
        let expected = format!("<__main__.Box object at {address:#x}>");
        assert_eq!(text(typy_rt_instance_text(address as u64)), expected);
        let written = typy_rt_instance_repr(instance.cast());
        assert!(written.exception.is_null());
        assert_eq!(text(written.value), expected);
        crate::refcount::typy_rt_decrement(instance.cast());
    }
}

#[test]
fn an_identity_hash_rotates_the_address_and_is_never_minus_one() {
    let at = |address: usize| core::ptr::with_exposed_provenance_mut(address);
    // SAFETY: the hash reads only the address.
    unsafe {
        assert_eq!(typy_rt_identity_hash(at(0x1230)), 0x123);
        assert_eq!(
            typy_rt_identity_hash(at(0xf)),
            (0xf_u64 << 60).cast_signed()
        );
        assert_eq!(typy_rt_identity_hash(at(usize::MAX)), -2);
    }
}
