//! The set operators and comparisons of dict views of `int` keys, each with the text CPython
//! 3.12 to 3.15 give for the result.

use super::{address, int_set, ints, release, repr, word};
use crate::dict::DictObject;
use crate::dict::view::{ViewObject, typy_rt_dict_view};
use crate::list::ItemKind;
use crate::set::SetObject;
use crate::set::views::{typy_rt_dict_view_combine, typy_rt_dict_view_compare};
use crate::tuple::{Field, TupleType};

/// Returns a dict of each `int` key in `keys` to the value `value_of` gives it.
fn dict(keys: impl Iterator<Item = i64>, value_of: fn(i64) -> i64) -> *mut DictObject {
    let int = word(ItemKind::Int as i64);
    let dict = crate::dict::typy_rt_dict_new(int, 0, int, 0, word(0));
    for key in keys {
        // SAFETY: the dict is live, and the key and value are inline words.
        unsafe {
            crate::dict::typy_rt_dict_set(
                dict,
                word(key).cast_unsigned(),
                word(value_of(key)).cast_unsigned(),
            );
        }
    }
    dict
}

/// The two dicts of the tests: `{n: n * n for n in range(12)}` and
/// `{n: n for n in range(0, 24, 3)}`.
fn dicts() -> [*mut DictObject; 2] {
    [
        dict(0..12, |key| key * key),
        dict((0..24).step_by(3), |key| key),
    ]
}

/// Returns a view of the keys, 0, or the items, 2, of a dict.
fn view(dict: *mut DictObject, kind: i64) -> *mut ViewObject {
    // SAFETY: the dict is live.
    unsafe { typy_rt_dict_view(dict, word(kind)) }
}

/// The descriptor of a tuple of two `int` values.
fn pair() -> *const TupleType {
    let field = |offset| Field {
        kind: ItemKind::Int as u32,
        offset,
        tuple: core::ptr::null(),
    };
    let fields = Box::leak(Box::new([field(0), field(8)]));
    Box::leak(Box::new(TupleType {
        size: 16,
        length: 2,
        fields: fields.as_ptr(),
    }))
}

/// Returns the text of `left op right` into a new set of `int` values or of pairs.
fn combined(left: u64, right: u64, operator: i64, pairs: bool) -> String {
    let result: *mut SetObject = if pairs {
        crate::set::typy_rt_set_new(
            word(ItemKind::Tuple as i64),
            pair().expose_provenance() as u64,
        )
    } else {
        int_set()
    };
    // SAFETY: both operands are live collections of the result's items.
    unsafe { typy_rt_dict_view_combine(left, right, word(operator), result) };
    let shown = repr(result);
    release(result);
    shown
}

fn release_object<T>(object: *mut T) {
    // SAFETY: the test holds a reference.
    unsafe { crate::refcount::typy_rt_decrement(object.cast()) };
}

#[test]
fn keys_views_combine_with_views_and_sets_as_in_cpython() {
    let [squares, threes] = dicts();
    let (first, second) = (view(squares, 0), view(threes, 0));
    let fives = ints(&[0, 5, 10, 15, 20, 25]);
    let twenty = ints(&(0..20).collect::<Vec<_>>());
    let list = super::int_list(&[9, 30, 3]);
    let (a, b, s) = (address(first), address(second), address(fives));
    let cases: [(u64, u64, i64, &str); 13] = [
        (b, address(twenty), 1, "{0, 3, 6, 9, 12, 15, 18}"),
        (address(list), a, 1, "{9, 3}"),
        (
            a,
            b,
            0,
            "{0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 15, 18, 21}",
        ),
        (a, b, 1, "{0, 9, 3, 6}"),
        (b, a, 1, "{0, 9, 3, 6}"),
        (a, b, 2, "{1, 2, 4, 5, 7, 8, 10, 11}"),
        (a, b, 3, "{1, 2, 4, 5, 7, 8, 10, 11, 12, 15, 18, 21}"),
        (a, s, 1, "{0, 10, 5}"),
        (s, a, 1, "{0, 10, 5}"),
        (s, b, 1, "{0, 15}"),
        (s, a, 2, "{20, 25, 15}"),
        (s, b, 0, "{0, 3, 5, 6, 9, 10, 12, 15, 18, 20, 21, 25}"),
        (s, b, 3, "{3, 5, 6, 9, 10, 12, 18, 20, 21, 25}"),
    ];
    for (left, right, operator, expected) in cases {
        assert_eq!(
            combined(left, right, operator, false),
            expected,
            "{operator}"
        );
    }
    release(fives);
    release(twenty);
    release_object(list);
    for object in [first, second] {
        release_object(object);
    }
    for object in [squares, threes] {
        release_object(object);
    }
}

#[test]
fn items_views_combine_as_in_cpython_with_their_own_symmetric_difference() {
    let [squares, threes] = dicts();
    let (first, second) = (view(squares, 2), view(threes, 2));
    let (a, b) = (address(first), address(second));
    assert_eq!(combined(a, b, 1, true), "{(0, 0)}");
    assert_eq!(
        combined(a, b, 3, true),
        "{(7, 49), (2, 4), (4, 16), (9, 9), (1, 1), (5, 25), (8, 64), (6, 36), (12, 12), \
         (3, 3), (18, 18), (21, 21), (3, 9), (6, 6), (10, 100), (11, 121), (9, 81), (15, 15)}"
    );
    assert_eq!(
        combined(b, a, 3, true),
        "{(7, 49), (2, 4), (4, 16), (9, 9), (1, 1), (6, 6), (8, 64), (6, 36), (10, 100), \
         (3, 9), (11, 121), (3, 3), (12, 12), (5, 25), (18, 18), (21, 21), (9, 81), (15, 15)}"
    );
    assert_eq!(
        combined(a, b, 2, true),
        "{(7, 49), (2, 4), (4, 16), (1, 1), (8, 64), (6, 36), (10, 100), (3, 9), (11, 121), \
         (5, 25), (9, 81)}"
    );
    assert_eq!(
        combined(b, a, 0, true),
        "{(8, 64), (6, 36), (11, 121), (5, 25), (9, 81), (10, 100), (12, 12), (3, 3), \
         (21, 21), (3, 9), (7, 49), (2, 4), (18, 18), (4, 16), (9, 9), (0, 0), (1, 1), \
         (6, 6), (15, 15)}"
    );
    for object in [first, second] {
        release_object(object);
    }
    for object in [squares, threes] {
        release_object(object);
    }
}

#[test]
fn views_compare_by_their_sizes_and_items_as_in_cpython() {
    let [squares, threes] = dicts();
    let copy = dict(0..12, |key| key * key);
    let views = [
        view(squares, 0),
        view(threes, 0),
        view(squares, 2),
        view(copy, 2),
        view(threes, 2),
    ];
    let [a, b, items, same, other_items] = views.map(|view| address(view));
    let twelve = ints(&(0..12).collect::<Vec<_>>());
    let fives = ints(&[0, 5, 10, 15, 20, 25]);
    let small = ints(&[0, 3]);
    let int = word(ItemKind::Int as i64);
    let tuple = word(ItemKind::Tuple as i64);
    let pair = pair().expose_provenance() as u64;
    let cases: [(u64, u64, i64, i64, u64, bool); 15] = [
        (a, address(small), 5, int, 0, true),
        (address(small), a, 5, int, 0, false),
        (address(small), a, 2, int, 0, true),
        (a, b, 0, int, 0, false),
        (b, a, 4, int, 0, false),
        (a, b, 1, int, 0, true),
        (address(small), a, 3, int, 0, true),
        (items, same, 2, tuple, pair, false),
        (a, address(twelve), 0, int, 0, true),
        (a, address(fives), 2, int, 0, false),
        (b, address(small), 4, int, 0, true),
        (items, same, 0, tuple, pair, true),
        (items, other_items, 3, tuple, pair, false),
        (a, a, 1, int, 0, false),
        (address(small), b, 3, int, 0, true),
    ];
    for (left, right, comparison, kind, descriptor, expected) in cases {
        // SAFETY: both operands are live collections of the items `kind` gives.
        let holds =
            unsafe { typy_rt_dict_view_compare(left, right, word(comparison), kind, descriptor) };
        assert_eq!(holds, expected, "comparison {comparison}");
    }
    for set in [twelve, fives, small] {
        release(set);
    }
    for object in views {
        release_object(object);
    }
    for object in [squares, threes, copy] {
        release_object(object);
    }
}
