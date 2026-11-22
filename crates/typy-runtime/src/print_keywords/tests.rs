use crate::dict::DictObject;
use crate::list::ItemKind;
use crate::string::from_str;
use crate::word::inline;

/// Returns a new dict of `str` keys and values of `kind`, holding `entries`.
fn dict_of(kind: ItemKind, entries: &[(&str, u64)]) -> *mut DictObject {
    // SAFETY: the kinds are those of plain items, which need no descriptor.
    unsafe {
        let dict = crate::dict::typy_rt_dict_new(
            inline(ItemKind::Object as i64).unwrap(),
            0,
            inline(kind as i64).unwrap(),
            0,
            inline(0).unwrap(),
        );
        for (name, value) in entries {
            let key = from_str(name);
            crate::dict::typy_rt_dict_set(dict, key.expose_provenance() as u64, *value);
            crate::refcount::typy_rt_decrement(key.cast());
        }
        dict
    }
}

/// Returns the text of `str` of an exception, and releases it.
fn shown(exception: *mut crate::ExceptionObject) -> String {
    // SAFETY: the exception is live and owned here.
    unsafe {
        let text = crate::exception::classes::BASE_EXCEPTION.str.unwrap()(exception.cast());
        let shown = (0..crate::string::length(text.value))
            .map(|index| char::from_u32(crate::string::code_point(text.value, index)).unwrap())
            .collect();
        crate::refcount::typy_rt_decrement(text.value.cast());
        crate::refcount::typy_rt_decrement(exception.cast());
        shown
    }
}

#[test]
fn exp_22_print_checks_each_keyword_by_the_value_it_holds() {
    let stdout = crate::text_file::typy_rt_sys_stdout().expose_provenance() as u64;
    let dash = from_str("-");
    let dash_bits = dash.expose_provenance() as u64;
    let nothing = dict_of(
        ItemKind::OptionalObject,
        &[("sep", 0), ("end", 0), ("file", 0)],
    );
    let files = dict_of(
        ItemKind::OptionalObject,
        &[("file", stdout), ("sep", dash_bits)],
    );
    // SAFETY: the dicts are live for the whole test.
    unsafe {
        assert!(super::typy_rt_print_check(nothing).is_null());
        assert!(super::typy_rt_print_check(files).is_null());
        assert_eq!(
            super::typy_rt_print_file(files).expose_provenance() as u64,
            stdout
        );
        for each in [nothing, files] {
            crate::refcount::typy_rt_decrement(each.cast());
        }
        crate::refcount::typy_rt_decrement(dash.cast());
    }
}

#[test]
fn exp_22_print_takes_each_keyword_its_values_fit_and_raises_for_any_other() {
    let dash = from_str("-");
    let dash_bits = dash.expose_provenance() as u64;
    let texts = dict_of(ItemKind::Object, &[("sep", dash_bits), ("file", dash_bits)]);
    let numbers = dict_of(
        ItemKind::Int,
        &[("end", inline(3).unwrap().cast_unsigned())],
    );
    let flags = dict_of(ItemKind::Bool, &[("flush", 1)]);
    let (space, sep) = (from_str(" "), from_str("sep"));
    // SAFETY: the dicts and strings are live for the whole test.
    unsafe {
        assert_eq!(
            shown(super::typy_rt_print_check(texts)),
            "'str' object has no attribute 'write'"
        );
        assert_eq!(
            shown(super::typy_rt_print_check(numbers)),
            "end must be None or a string, not int"
        );
        assert!(super::typy_rt_print_check(flags).is_null());
        assert!(core::ptr::eq(
            super::typy_rt_print_text(texts, sep, space),
            dash
        ));
        assert!(core::ptr::eq(
            super::typy_rt_print_text(numbers, sep, space),
            space
        ));
        assert!(super::typy_rt_print_flush(flags));
        assert!(!super::typy_rt_print_flush(texts));
        assert!(core::ptr::eq(
            super::typy_rt_print_file(numbers),
            crate::text_file::typy_rt_sys_stdout()
        ));
        for each in [texts, numbers, flags] {
            crate::refcount::typy_rt_decrement(each.cast());
        }
        for each in [dash, space, sep] {
            crate::refcount::typy_rt_decrement(each.cast());
        }
    }
}
