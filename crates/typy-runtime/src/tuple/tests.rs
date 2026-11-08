//! Tests of lists of tuples: storage, counting, comparison, text and every list operation.

use super::{Field, TupleType};
use crate::list::{ItemKind, ItemType, ListObject, new_list};
use crate::string::{StrObject, code_point, from_str, length};
use crate::word::{inline, inline_value};

fn word(value: i64) -> i64 {
    inline(value).unwrap()
}

fn text(string: *const StrObject) -> String {
    // SAFETY: the string is live and every index is in bounds.
    (0..unsafe { length(string) })
        .map(|index| char::from_u32(unsafe { code_point(string, index) }).unwrap())
        .collect()
}

fn descriptor(size: u64, fields: Vec<Field>) -> &'static TupleType {
    let length = u64::try_from(fields.len()).unwrap();
    let fields = Box::leak(fields.into_boxed_slice());
    Box::leak(Box::new(TupleType {
        size,
        length,
        fields: fields.as_ptr(),
    }))
}

fn field(kind: ItemKind, offset: u32) -> Field {
    Field {
        kind: kind as u32,
        offset,
        tuple: core::ptr::null(),
    }
}

/// `tuple[int, str]`.
fn int_and_str() -> &'static TupleType {
    descriptor(
        16,
        vec![field(ItemKind::Int, 0), field(ItemKind::Object, 8)],
    )
}

fn string(value: &str) -> u64 {
    from_str(value).expose_provenance() as u64
}

fn count(object: u64) -> u32 {
    // SAFETY: the object is live.
    unsafe {
        (*core::ptr::with_exposed_provenance::<crate::header::ObjectHeader>(
            usize::try_from(object).unwrap(),
        ))
        .reference_count
    }
}

/// Appends a tuple held in `words` and returns the list.
fn append(list: *mut ListObject, words: &[u64]) {
    // SAFETY: the list is live, and `words` is a tuple of its layout.
    unsafe { crate::list::typy_rt_list_append(list, words.as_ptr().expose_provenance() as u64) };
}

fn pairs(values: &[(i64, u64)]) -> *mut ListObject {
    let list = new_list(ItemType::tuple(int_and_str()), 0);
    for (number, text) in values {
        append(list, &[word(*number).cast_unsigned(), *text]);
    }
    list
}

fn repr(list: *const ListObject) -> String {
    // SAFETY: the list is live.
    let result = unsafe { crate::list::typy_rt_list_repr(list) };
    assert!(result.exception.is_null());
    text(result.value)
}

#[test]
fn tuples_are_stored_inline_and_counted() {
    let a = string("a");
    let list = pairs(&[(1, a), (2, string("b"))]);
    // SAFETY: the list is live.
    unsafe {
        assert_eq!((*list).words, 2);
        assert_eq!((*list).length, 2);
    }
    assert_eq!(count(a), 2);
    assert_eq!(repr(list), "[(1, 'a'), (2, 'b')]");
    // SAFETY: the list is live and nothing else refers to it.
    unsafe { crate::list::destroy(list) };
    assert_eq!(count(a), 1);
}

#[test]
fn every_kind_of_field_is_written_as_cpython_writes_it() {
    let inner = descriptor(
        16,
        vec![field(ItemKind::Object, 0), field(ItemKind::Int, 8)],
    );
    let nested = descriptor(
        32,
        vec![
            field(ItemKind::Bool, 0),
            field(ItemKind::NoneValue, 0),
            field(ItemKind::Float, 8),
            Field {
                kind: ItemKind::Tuple as u32,
                offset: 16,
                tuple: inner,
            },
        ],
    );
    let list = new_list(ItemType::tuple(nested), 0);
    append(
        list,
        &[
            1,
            (-0.5_f64).to_bits(),
            string("x"),
            word(7).cast_unsigned(),
        ],
    );
    let single = descriptor(8, vec![field(ItemKind::Int, 0)]);
    let singles = new_list(ItemType::tuple(single), 0);
    append(singles, &[word(5).cast_unsigned()]);
    let empty = descriptor(8, Vec::new());
    let empties = new_list(ItemType::tuple(empty), 0);
    append(empties, &[0]);
    assert_eq!(repr(list), "[(True, None, -0.5, ('x', 7))]");
    assert_eq!(repr(singles), "[(5,)]");
    assert_eq!(repr(empties), "[()]");
    // SAFETY: the lists are live.
    unsafe {
        assert!(crate::list::typy_rt_list_any(singles));
        assert!(!crate::list::typy_rt_list_any(empties));
    }
}

#[test]
fn tuples_compare_item_by_item() {
    let a = string("a");
    let b = string("b");
    let list = pairs(&[(2, a), (1, b), (1, a), (1, b)]);
    let other = pairs(&[(2, a), (1, b), (1, a), (1, b)]);
    let shorter = pairs(&[(2, a), (1, b)]);
    let item: [u64; 2] = [word(1).cast_unsigned(), b];
    let address = item.as_ptr().expose_provenance() as u64;
    // SAFETY: the lists and the tuple are live.
    unsafe {
        use crate::list::typy_rt_list_equal;
        assert!(typy_rt_list_equal(list, other));
        assert!(crate::list::typy_rt_list_compare(shorter, list, word(2)));
        assert!(crate::list::typy_rt_list_contains(list, address));
        assert_eq!(
            inline_value(crate::list::typy_rt_list_count(list, address)),
            2
        );
        assert_eq!(
            inline_value(crate::list::typy_rt_list_index(list, address).value),
            1
        );
        crate::list::typy_rt_list_sort(list, word(0));
        assert_eq!(repr(list), "[(1, 'a'), (1, 'b'), (1, 'b'), (2, 'a')]");
        crate::list::typy_rt_list_sort(list, word(1));
        assert_eq!(repr(list), "[(2, 'a'), (1, 'b'), (1, 'b'), (1, 'a')]");
        let largest = crate::list::typy_rt_list_extreme(list, word(1)).value;
        assert_eq!(
            *core::ptr::with_exposed_provenance::<u64>(usize::try_from(largest).unwrap()),
            word(2).cast_unsigned()
        );
        assert_eq!(count(a), 7);
        crate::tuple::release(int_and_str(), largest);
    }
}

#[test]
fn equal_tuples_keep_their_order_when_sorted() {
    let zeros = descriptor(8, vec![field(ItemKind::Float, 0)]);
    let list = new_list(ItemType::tuple(zeros), 0);
    let values: Vec<f64> = (0..70)
        .map(|index| if index % 2 == 0 { 0.0 } else { -0.0 })
        .collect();
    for value in &values {
        append(list, &[value.to_bits()]);
    }
    // SAFETY: the list is live.
    unsafe { crate::list::typy_rt_list_sort(list, word(1)) };
    // SAFETY: the list is live, and each item is one word.
    let signs: Vec<bool> = (0..values.len())
        .map(|index| unsafe { f64::from_bits(*(*list).items.add(index)) }.is_sign_negative())
        .collect();
    let original: Vec<bool> = values
        .iter()
        .map(|value| value.is_sign_negative())
        .collect();
    assert_eq!(signs, original);
}

#[test]
fn removing_and_moving_tuples_keeps_their_references() {
    let a = string("a");
    let list = pairs(&[(1, a), (2, a), (3, a)]);
    // SAFETY: the lists and items are live.
    unsafe {
        use crate::list::{typy_rt_list_extend, typy_rt_list_pop};
        let popped = typy_rt_list_pop(list, word(0), word(1)).value;
        let words = core::ptr::with_exposed_provenance::<u64>(usize::try_from(popped).unwrap());
        assert_eq!(inline_value((*words).cast_signed()), 1);
        assert_eq!(count(a), 4);
        crate::tuple::release(int_and_str(), popped);
        assert_eq!(repr(list), "[(2, 'a'), (3, 'a')]");
        typy_rt_list_extend(list, list);
        assert_eq!(repr(list), "[(2, 'a'), (3, 'a'), (2, 'a'), (3, 'a')]");
        let item: [u64; 2] = [word(9).cast_unsigned(), a];
        crate::list::typy_rt_list_insert(list, word(1), item.as_ptr().expose_provenance() as u64);
        crate::list::typy_rt_list_reverse(list);
        assert_eq!(
            repr(list),
            "[(3, 'a'), (2, 'a'), (3, 'a'), (9, 'a'), (2, 'a')]"
        );
        assert!(
            crate::list::typy_rt_list_remove(list, item.as_ptr().expose_provenance() as u64)
                .is_null()
        );
        assert_eq!(count(a), 5);
        let doubled = crate::list::typy_rt_list_repeat(list, word(2)).value;
        assert_eq!((*doubled).length, 8);
        let joined = crate::list::typy_rt_list_concat(list, doubled);
        assert_eq!((*joined).length, 12);
        crate::list::destroy(joined);
        crate::list::destroy(doubled);
        crate::list::typy_rt_list_clear(list);
        assert_eq!(count(a), 1);
    }
}
