//! Tests of `typing.TypeAliasType` objects.

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
        qualified_name: name,
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

#[test]
fn typ_18_a_function_s_alias_is_a_new_object_written_by_its_qualified_name() {
    let (name, module) = (from_str("Local"), from_str("__main__"));
    let qualified_name = from_str("make.<locals>.Local");
    let description = super::TypeAliasObject {
        header: crate::header::ObjectHeader {
            reference_count: crate::header::IMMORTAL,
            type_id: crate::header::TYPE_ALIAS_TYPE_ID,
        },
        name,
        module,
        qualified_name,
    };
    // SAFETY: the description and the strings are live for the whole test.
    unsafe {
        let description = core::ptr::from_ref(&description).cast();
        let first = super::typy_rt_type_alias_new(description);
        let second = super::typy_rt_type_alias_new(description);
        assert!(!core::ptr::eq(first, second));
        assert_eq!((*first).reference_count, 1);
        let repr = super::type_alias_repr(first).value;
        assert_eq!(text(repr), "make.<locals>.Local");
        let own = super::typy_rt_type_alias_name(first);
        assert_eq!(text(own), "Local");
        for each in [repr, own] {
            crate::refcount::typy_rt_decrement(each.cast());
        }
        crate::refcount::typy_rt_decrement(first);
        crate::refcount::typy_rt_decrement(second);
        for each in [name, module, qualified_name] {
            crate::refcount::typy_rt_decrement(each.cast());
        }
    }
}
