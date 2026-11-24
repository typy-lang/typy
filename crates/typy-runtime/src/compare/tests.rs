//! Tests of turning an ordering into the result of a comparison.

use super::holds;
use crate::word::inline;
use core::cmp::Ordering;

#[test]
fn each_code_tests_its_comparison() {
    use Ordering::{Equal, Greater, Less};
    // For each code, the orderings it holds for, out of Less, Equal and Greater.
    let table = [
        (0, [false, true, false]),
        (1, [true, false, true]),
        (2, [true, false, false]),
        (3, [true, true, false]),
        (4, [false, false, true]),
        (5, [false, true, true]),
    ];
    for (code, expected) in table {
        let word = inline(code).unwrap();
        let actual = [Less, Equal, Greater].map(|ordering| holds(ordering, word));
        assert_eq!(actual, expected, "code {code}");
    }
}
