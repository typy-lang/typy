//! Tests of integers without a size limit.

use super::BigIntObject;

#[test]
fn the_fixed_part_takes_sixteen_bytes() {
    assert_eq!(core::mem::size_of::<BigIntObject>(), 16);
}

#[test]
fn a_repeat_count_is_none_below_one_and_refused_past_an_index_sized_integer() {
    use super::repeat_count;
    use super::value::Integer;
    let cases = [
        (-(1_i128 << 70), Some(0)),
        (-1, Some(0)),
        (0, Some(0)),
        (3, Some(3)),
        (1 << 62, Some(1 << 62)),
        (
            i128::from(i64::MAX),
            Some(usize::try_from(i64::MAX).unwrap()),
        ),
        (i128::from(i64::MAX) + 1, None),
        (1 << 70, None),
    ];
    for (count, wanted) in cases {
        let word = Integer::from_i128(count).into_word();
        // SAFETY: the word is a live `int` word.
        assert_eq!(unsafe { repeat_count(word) }, wanted, "{count}");
    }
}
