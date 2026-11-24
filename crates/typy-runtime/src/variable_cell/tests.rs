//! Tests of cells of captured variables.

use crate::list::ItemKind;
use crate::string::from_str;
use crate::tuple::{Field, TupleType};

#[test]
fn fun_04_a_cell_starts_with_no_value_and_releases_its_value_when_it_is_freed() {
    let fields = [
        Field::plain(ItemKind::Object, 0),
        Field::plain(ItemKind::Bool, 8),
    ];
    let layout = TupleType {
        size: 16,
        length: 2,
        fields: fields.as_ptr(),
    };
    let held = from_str("held");
    // SAFETY: the layout and the string live for the whole test.
    unsafe {
        let cell = super::typy_rt_cell_new(&raw const layout);
        let payload =
            core::ptr::with_exposed_provenance_mut::<u64>(cell.expose_provenance() + super::PREFIX);
        assert_eq!([*payload, *payload.add(1)], [0, 0]);
        crate::refcount::typy_rt_increment(held.cast());
        *payload = held.expose_provenance() as u64;
        *payload.add(1) = 1;
        crate::refcount::typy_rt_decrement(cell.cast());
        assert_eq!((*held).header.reference_count, 1);
        crate::refcount::typy_rt_decrement(held.cast());
    }
}

#[test]
fn fun_04_an_unbound_free_variable_raises_cpythons_name_error() {
    let name = from_str("total");
    // SAFETY: the name is live; the exception is the caller's.
    unsafe {
        let error = super::typy_rt_free_variable_unbound(name);
        let message = (*error).message;
        let text: String = (0..crate::string::length(message))
            .map(|index| char::from_u32(crate::string::code_point(message, index)).unwrap())
            .collect();
        assert_eq!(
            text,
            "cannot access free variable 'total' where it is not associated with a value in \
                 enclosing scope"
        );
        crate::refcount::typy_rt_decrement(error.cast());
        crate::refcount::typy_rt_decrement(name.cast());
    }
}
