//! Turns an ordering into the result of a comparison, numbered as TyIR's `Comparison::code`.

use core::cmp::Ordering;

use crate::word::inline_value;

/// Returns whether `ordering` satisfies the comparison whose code is the tagged `int` `comparison`: 0 `==`, 1 `!=`, 2 `<`, 3 `<=`, 4 `>`, 5 `>=`.
pub(crate) fn holds(ordering: Ordering, comparison: i64) -> bool {
    match inline_value(comparison) {
        0 => ordering.is_eq(),
        1 => ordering.is_ne(),
        2 => ordering.is_lt(),
        3 => ordering.is_le(),
        4 => ordering.is_gt(),
        _ => ordering.is_ge(),
    }
}

#[cfg(test)]
mod tests {
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
}
