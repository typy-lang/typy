//! Tests of bound methods.

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
