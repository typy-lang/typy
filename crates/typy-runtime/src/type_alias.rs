//! `typing.TypeAliasType` objects, the value a `type` statement binds (SMT-12): an immortal
//! static object the compiler emits for each alias, with the alias's name and the name of
//! its module. Its `repr` and `str` are the name, and it hashes and compares by identity.

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

/// Returns `repr` of a type alias: its name, as CPython writes an alias of a module's top
/// level.
unsafe extern "C" fn type_alias_repr(object: *mut ObjectHeader) -> Raising<*mut StrObject> {
    // SAFETY: the type of the object says it is a type alias object.
    Raising::ok(unsafe { typy_rt_type_alias_name(object.cast()) })
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
    fn a_type_alias_reads_as_its_name_in_its_module_and_type() {
        let (name, module) = (from_str("Score"), from_str("__main__"));
        let alias = super::TypeAliasObject {
            header: crate::header::ObjectHeader {
                reference_count: crate::header::IMMORTAL,
                type_id: crate::header::TYPE_ALIAS_TYPE_ID,
            },
            name,
            module,
        };
        // SAFETY: the alias and the strings are live for the whole test.
        unsafe {
            let object = core::ptr::from_ref(&alias).cast_mut().cast();
            let repr = super::type_alias_repr(object).value;
            assert_eq!(text(repr), "Score");
            assert_eq!(text(super::typy_rt_type_alias_module(object)), "__main__");
            let ty = crate::type_object::typy_rt_type_text(&raw const super::TYPE_ALIAS);
            assert_eq!(text(ty), "<class 'typing.TypeAliasType'>");
            for each in [repr, name, module, ty] {
                crate::refcount::typy_rt_decrement(each.cast());
            }
            // `__module__` gave a second reference to the module's name.
            crate::refcount::typy_rt_decrement(module.cast());
        }
    }
}
