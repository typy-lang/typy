//! `typing.TypeAliasType` objects, the value a `type` statement binds (SMT-12): an immortal
//! static object the compiler emits for each alias, with the alias's name, the name of its
//! module and its qualified name, which its `repr` and `str` write. One a function defines is
//! a new copy of its static description each time its `type` statement runs (TYP-18). It
//! hashes and compares by identity.

use crate::Raising;
use crate::header::ObjectHeader;
use crate::string::StrObject;
use crate::type_object::{OBJECT, TypeObject};

/// A type alias object, which the compiler emits with static strings.
#[repr(C)]
#[derive(Debug)]
pub(crate) struct TypeAliasObject {
    header: ObjectHeader,
    name: *const StrObject,
    module: *const StrObject,
    qualified_name: *const StrObject,
}

/// The type `typing.TypeAliasType`.
#[unsafe(export_name = "typy_rt_type_type_alias")]
pub(crate) static TYPE_ALIAS: TypeObject = TypeObject {
    hash: Some(crate::instance::typy_rt_identity_hash),
    repr: Some(type_alias_repr),
    module: "typing".as_ptr(),
    module_length: 6,
    ..TypeObject::new("TypeAliasType", &raw const OBJECT)
};

/// Returns `repr` of a type alias: its qualified name, which is its name for an alias of a
/// module's top level, as CPython 3.15 writes it.
#[allow(
    clippy::cast_ptr_alignment,
    reason = "a type alias object is a static the compiler aligns for its pointers, or a copy of one"
)]
unsafe extern "C" fn type_alias_repr(object: *mut ObjectHeader) -> Raising<*mut StrObject> {
    // SAFETY: the type of the object says it is a type alias object, whose qualified name is
    // an immortal static string.
    unsafe {
        let name = (*object.cast::<TypeAliasObject>())
            .qualified_name
            .cast_mut();
        crate::refcount::typy_rt_increment(name.cast());
        Raising::ok(name)
    }
}

/// Returns a new type alias of one reference, a copy of `description`, the static object of
/// an alias a function defines: each run of its `type` statement makes one (TYP-18).
///
/// # Safety
///
/// `description` points to a type alias object the compiler emitted.
#[unsafe(no_mangle)]
#[allow(
    clippy::cast_ptr_alignment,
    reason = "malloc returns memory aligned for every type"
)]
pub unsafe extern "C" fn typy_rt_type_alias_new(
    description: *const ObjectHeader,
) -> *mut ObjectHeader {
    let alias = crate::allocator::allocate(core::mem::size_of::<TypeAliasObject>())
        .cast::<TypeAliasObject>();
    // SAFETY: the allocation holds a type alias object; the caller's guarantee gives the
    // description's immortal strings, which the copy shares.
    unsafe {
        let description = description.cast::<TypeAliasObject>();
        alias.write(TypeAliasObject {
            header: ObjectHeader::new(crate::header::TYPE_ALIAS_TYPE_ID),
            name: (*description).name,
            module: (*description).module,
            qualified_name: (*description).qualified_name,
        });
    }
    alias.cast()
}

/// Returns a type alias's `__name__`, a reference of the caller's own.
///
/// # Safety
///
/// `alias` points to a live type alias object.
#[unsafe(no_mangle)]
#[allow(
    clippy::cast_ptr_alignment,
    reason = "a type alias object is a static the compiler aligns for its pointers"
)]
pub unsafe extern "C" fn typy_rt_type_alias_name(alias: *const ObjectHeader) -> *mut StrObject {
    // SAFETY: the caller's guarantee; the name is an immortal static string.
    unsafe {
        let name = (*alias.cast::<TypeAliasObject>()).name.cast_mut();
        crate::refcount::typy_rt_increment(name.cast());
        name
    }
}

/// Returns a type alias's `__module__`, a reference of the caller's own.
///
/// # Safety
///
/// `alias` points to a live type alias object.
#[unsafe(no_mangle)]
#[allow(
    clippy::cast_ptr_alignment,
    reason = "a type alias object is a static the compiler aligns for its pointers"
)]
pub unsafe extern "C" fn typy_rt_type_alias_module(alias: *const ObjectHeader) -> *mut StrObject {
    // SAFETY: the caller's guarantee; the module's name is an immortal static string.
    unsafe {
        let module = (*alias.cast::<TypeAliasObject>()).module.cast_mut();
        crate::refcount::typy_rt_increment(module.cast());
        module
    }
}

#[cfg(test)]
mod tests;
