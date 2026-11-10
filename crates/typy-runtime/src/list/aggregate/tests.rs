//! Tests of sorting, `min`, `max`, `sum`, `any` and `all` over lists.

use super::{
    typy_rt_list_all, typy_rt_list_any, typy_rt_list_extreme, typy_rt_list_sort,
    typy_rt_list_sum_float, typy_rt_list_sum_int,
};
use crate::int::value::Integer;
use crate::list::operations::typy_rt_list_append;
use crate::list::{ItemKind, ListObject, new_list};
use crate::word::{inline, inline_value};

fn word(value: i64) -> i64 {
    inline(value).unwrap()
}

fn list_of(kind: ItemKind, items: &[u64]) -> *mut ListObject {
    let list = new_list(kind, 0);
    for item in items {
        // SAFETY: the list is live, and the items need no reference or are kept alive.
        unsafe { typy_rt_list_append(list, *item) };
    }
    list
}

fn message(exception: *mut crate::ExceptionObject) -> String {
    // SAFETY: the exception is live; its message is a live string.
    let text = unsafe { (*exception).message };
    // SAFETY: the message is live and every index is in bounds.
    (0..unsafe { crate::string::length(text) })
        .map(|index| char::from_u32(unsafe { crate::string::code_point(text, index) }).unwrap())
        .collect()
}

fn ints(values: &[i64]) -> *mut ListObject {
    let words: Vec<u64> = values
        .iter()
        .map(|value| word(*value).cast_unsigned())
        .collect();
    list_of(ItemKind::Int, &words)
}

fn floats(values: &[f64]) -> *mut ListObject {
    let bits: Vec<u64> = values.iter().map(|value| value.to_bits()).collect();
    list_of(ItemKind::Float, &bits)
}

fn items(list: *const ListObject) -> Vec<u64> {
    // SAFETY: the list is live.
    unsafe { super::items_of(list).to_vec() }
}

fn int_values(list: *const ListObject) -> Vec<i64> {
    items(list)
        .into_iter()
        .map(|item| inline_value(item.cast_signed()))
        .collect()
}

#[test]
fn sorting_orders_short_and_long_lists_like_a_stable_sort() {
    let mut state: u64 = 0x2545_f491_4f6c_dd1d;
    let values: Vec<i64> = (0..1000)
        .map(|_| {
            state ^= state << 13;
            state ^= state >> 7;
            state ^= state << 17;
            i64::try_from(state % 200).unwrap() - 100
        })
        .collect();
    for length in [0, 1, 2, 31, 32, 33, 64, 65, 1000] {
        let list = ints(&values[..length]);
        let mut expected = values[..length].to_vec();
        expected.sort_unstable();
        // SAFETY: the list is live.
        unsafe { typy_rt_list_sort(list, word(0)) };
        assert_eq!(int_values(list), expected, "{length} items");
        // SAFETY: the list is live.
        unsafe { typy_rt_list_sort(list, word(1)) };
        expected.reverse();
        assert_eq!(int_values(list), expected, "{length} items reversed");
    }
}

#[test]
fn equal_items_keep_their_order_in_both_directions() {
    let zeros: Vec<f64> = (0..80)
        .map(|index| if index % 3 == 0 { -0.0 } else { 0.0 })
        .collect();
    let mixed: Vec<f64> = zeros.iter().copied().chain([-1.0, 1.0]).collect();
    for reverse in [0, 1] {
        let list = floats(&mixed);
        // SAFETY: the list is live.
        unsafe { typy_rt_list_sort(list, word(reverse)) };
        let sorted: Vec<f64> = items(list).into_iter().map(f64::from_bits).collect();
        let signs: Vec<bool> = sorted
            .iter()
            .filter(|value| **value == 0.0)
            .map(|value| value.is_sign_negative())
            .collect();
        let original: Vec<bool> = zeros.iter().map(|value| value.is_sign_negative()).collect();
        assert_eq!(signs, original, "reverse {reverse}");
        let ends = (sorted[0], sorted[sorted.len() - 1]);
        assert_eq!(
            ends,
            if reverse == 0 {
                (-1.0, 1.0)
            } else {
                (1.0, -1.0)
            }
        );
    }
}

#[test]
fn strings_sort_by_code_point() {
    let words: Vec<u64> = ["pear", "apple", "Zebra", "apples", ""]
        .iter()
        .map(|text| crate::string::from_str(text).expose_provenance() as u64)
        .collect();
    let list = list_of(ItemKind::Object, &words);
    // SAFETY: the list and its strings are live.
    unsafe { typy_rt_list_sort(list, word(0)) };
    let order: Vec<usize> = items(list)
        .iter()
        .map(|item| words.iter().position(|word| word == item).unwrap())
        .collect();
    assert_eq!(order, [4, 2, 1, 3, 0]);
}

#[test]
fn min_and_max_keep_the_first_of_equal_items_and_refuse_an_empty_list() {
    let list = floats(&[0.0, -0.0, 2.0, 2.0, -3.5]);
    // SAFETY: the list is live.
    unsafe {
        assert_eq!(typy_rt_list_extreme(list, word(1)).value, 2.0_f64.to_bits());
        assert_eq!(
            typy_rt_list_extreme(list, word(0)).value,
            (-3.5_f64).to_bits()
        );
        let zeros = floats(&[0.0, -0.0]);
        assert_eq!(
            typy_rt_list_extreme(zeros, word(1)).value,
            0.0_f64.to_bits()
        );
        assert_eq!(
            typy_rt_list_extreme(zeros, word(0)).value,
            0.0_f64.to_bits()
        );
        for (largest, expected) in [
            (1, "max() iterable argument is empty"),
            (0, "min() iterable argument is empty"),
        ] {
            let empty = typy_rt_list_extreme(ints(&[]), word(largest));
            assert_eq!(message(empty.exception), expected);
        }
    }
}

#[test]
fn the_extreme_of_large_integers_gets_a_reference_of_its_own() {
    let huge = Integer::from_i128(1 << 90).into_word();
    let list = list_of(
        ItemKind::Int,
        &[word(1).cast_unsigned(), huge.cast_unsigned()],
    );
    // SAFETY: the list and the large integer are live.
    unsafe {
        let found = typy_rt_list_extreme(list, word(1)).value;
        assert_eq!(found, huge.cast_unsigned());
        crate::refcount::typy_rt_int_decrement(found.cast_signed());
        assert!(crate::int::typy_rt_int_compare(
            typy_rt_list_extreme(list, word(1)).value.cast_signed(),
            huge,
            word(0)
        ));
    }
}

#[test]
fn integer_sums_are_exact() {
    let huge = Integer::from_i128(1 << 100).into_word();
    let list = list_of(
        ItemKind::Int,
        &[
            huge.cast_unsigned(),
            word(-5).cast_unsigned(),
            huge.cast_unsigned(),
        ],
    );
    // SAFETY: the list and the words are live.
    unsafe {
        let total = typy_rt_list_sum_int(list, word(7));
        let expected = Integer::from_i128((1 << 101) + 2).into_word();
        assert!(crate::int::typy_rt_int_compare(total, expected, word(0)));
        assert_eq!(typy_rt_list_sum_int(ints(&[]), word(3)), word(3));
        assert_eq!(typy_rt_list_sum_int(ints(&[1, 2, 3]), word(0)), word(6));
    }
}

#[test]
fn float_sums_are_compensated_as_cpython_compensates_them() {
    let cases: [(&[f64], f64, f64); 6] = [
        (&[1e100, 1.0, -1e100], 0.0, 1.0),
        (&[0.1; 10], 0.0, 1.0),
        (&[0.1, 0.2, 0.3], 0.0, 0.6),
        (&[f64::INFINITY, 1.0], 0.0, f64::INFINITY),
        (&[1e308, 1e308], 0.0, f64::INFINITY),
        (&[], -0.0, -0.0),
    ];
    for (values, start, expected) in cases {
        // SAFETY: the list is live.
        let total = unsafe { typy_rt_list_sum_float(floats(values), start) };
        assert_eq!(total.to_bits(), expected.to_bits(), "{values:?}");
    }
    let opposite = floats(&[f64::INFINITY, -f64::INFINITY]);
    // SAFETY: the list is live.
    let total = unsafe { typy_rt_list_sum_float(opposite, 0.0) };
    assert!(total.is_nan());
}

#[test]
fn any_and_all_test_the_truth_of_each_kind_of_item() {
    let empty_text = crate::string::from_str("").expose_provenance() as u64;
    let text = crate::string::from_str("a").expose_provenance() as u64;
    let huge = Integer::from_i128(1 << 80).into_word().cast_unsigned();
    let cases: [(*mut ListObject, bool, bool); 8] = [
        (ints(&[]), false, true),
        (ints(&[0, 0]), false, false),
        (
            list_of(ItemKind::Int, &[word(0).cast_unsigned(), huge]),
            true,
            false,
        ),
        (floats(&[f64::NAN, 1.0]), true, true),
        (floats(&[-0.0]), false, false),
        (list_of(ItemKind::Bool, &[1, 0]), true, false),
        (list_of(ItemKind::Object, &[empty_text, text]), true, false),
        (
            list_of(
                ItemKind::Object,
                &[new_list(ItemKind::Int, 0).expose_provenance() as u64],
            ),
            false,
            false,
        ),
    ];
    for (index, (list, any, all)) in cases.into_iter().enumerate() {
        // SAFETY: the list and its items are live.
        unsafe {
            assert_eq!(typy_rt_list_any(list), any, "case {index}");
            assert_eq!(typy_rt_list_all(list), all, "case {index}");
        }
    }
}
