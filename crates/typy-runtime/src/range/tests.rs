//! Tests of the checks and last values of ranges.

use super::{typy_rt_range_check_step, typy_rt_range_last};
use crate::int::value::Integer;

#[test]
fn the_last_value_of_a_range_is_cpythons() {
    let cases: [(i128, i128, i128, i128); 9] = [
        (0, 5, 1, 4),
        (0, 10, 3, 9),
        (10, 0, -3, 1),
        (5, 5, 1, 4),
        (5, 0, 1, 4),
        (0, 5, -1, 1),
        (-3, 4, 2, 3),
        (0, 1 << 80, 1 << 70, 1023 << 70),
        (1 << 70, 0, -(1 << 69), 1 << 69),
    ];
    for (start, stop, step, last) in cases {
        let words = [start, stop, step].map(|value| Integer::from_i128(value).into_word());
        // SAFETY: the words are live; each is released once.
        unsafe {
            let found = typy_rt_range_last(words[0], words[1], words[2]);
            let expected = Integer::from_i128(last).into_word();
            assert!(
                crate::int::typy_rt_int_compare(found, expected, inline(0).unwrap()),
                "range({start}, {stop}, {step})"
            );
            for word in [found, expected].into_iter().chain(words) {
                crate::refcount::typy_rt_int_decrement(word);
            }
        }
    }
}
use crate::word::inline;

#[test]
fn only_a_zero_step_raises() {
    assert!(typy_rt_range_check_step(inline(1).unwrap()).is_null());
    assert!(typy_rt_range_check_step(inline(-2).unwrap()).is_null());
    let exception = typy_rt_range_check_step(inline(0).unwrap());
    assert!(!exception.is_null());
    // SAFETY: the exception is live and its reference is dropped here.
    unsafe { crate::refcount::typy_rt_decrement(exception.cast()) };
}
