//! Tests of the byte paths: each gives what the general path gives for the same text held two
//! bytes a code point.

use super::{count, find, rfind};
use crate::string::{StrKind, StrObject, code_point, from_str, length, new_string, set_code_point};
use crate::word::inline;

/// Returns the code points of a string as Rust text.
fn text(string: *const StrObject) -> String {
    // SAFETY: the string is live and every index is in bounds.
    (0..unsafe { length(string) })
        .map(|index| char::from_u32(unsafe { code_point(string, index) }).unwrap())
        .collect()
}

/// Returns `value` held two bytes a code point, which the methods read by the general path.
fn wide(value: &str) -> *mut StrObject {
    let points: Vec<u32> = value.chars().map(u32::from).collect();
    let string = new_string(points.len(), StrKind::Ucs2);
    for (index, point) in points.iter().enumerate() {
        // SAFETY: the string is new, with room for every code point.
        unsafe { set_code_point(string, index, *point) };
    }
    string
}

/// Returns both forms of `value`: one byte a code point and two.
fn both(value: &str) -> [*mut StrObject; 2] {
    [from_str(value), wide(value)]
}

/// Returns the strings of a list of strings.
fn texts(list: *mut crate::ListObject) -> Vec<String> {
    // SAFETY: the list is live and holds live strings.
    unsafe {
        (0..(*list).length)
            .map(|index| {
                text(core::ptr::with_exposed_provenance::<StrObject>(
                    usize::try_from((*list).item(index)).unwrap(),
                ))
            })
            .collect()
    }
}

#[test]
fn nfr_21_searches_of_bytes_find_what_a_search_of_code_points_finds() {
    assert_eq!(find(b"abcabc", b"ca"), Some(2));
    assert_eq!(find(b"abc", b""), Some(0));
    assert_eq!(find(b"ab", b"abc"), None);
    assert_eq!(rfind(b"abcabc", b"bc"), Some(4));
    assert_eq!(rfind(b"abc", b""), Some(3));
    assert_eq!(rfind(b"a", b"ab"), None);
    assert_eq!(count(b"aaaa", b"aa", u64::MAX), 2);
    assert_eq!(count(b"aaaa", b"aa", 1), 1);
    assert_eq!(count(b"abab", b"x", u64::MAX), 0);
}

#[test]
fn nfr_21_byte_paths_give_what_the_general_path_gives() {
    let cases = [
        "a,b,,c",
        ",lead and trail,",
        "\u{e9}t\u{e9},caf\u{e9}",
        "",
        "no separator",
    ];
    for value in cases {
        let [narrow_text, wide_text] = both(value);
        // SAFETY: every string is live.
        unsafe {
            for (separator, limit, flags) in [
                (",", -1, 1),
                (",", 1, 1),
                (",", 1, 3),
                (",", -1, 3),
                ("t\u{e9}", -1, 1),
            ] {
                let [narrow_separator, wide_separator] = both(separator);
                let split = |text, separator| {
                    texts(
                        crate::string::typy_rt_str_split(
                            text,
                            separator,
                            inline(limit).unwrap(),
                            inline(flags).unwrap(),
                        )
                        .value,
                    )
                };
                assert_eq!(
                    split(narrow_text, narrow_separator),
                    split(wide_text, wide_separator),
                    "{value:?} {separator:?}"
                );
            }
            for needle in [",", "a", "\u{e9}", "zz", "e"] {
                let [narrow_needle, wide_needle] = both(needle);
                for operation in 0..5 {
                    for (start, end, given) in [(0, 0, 0), (1, 4, 3), (-3, -1, 3)] {
                        let search = |text, needle| {
                            crate::string::typy_rt_str_search(
                                text,
                                needle,
                                inline(start).unwrap(),
                                inline(end).unwrap(),
                                inline(given | (operation << 2)).unwrap(),
                            )
                            .value
                        };
                        assert_eq!(
                            search(narrow_text, narrow_needle),
                            search(wide_text, wide_needle),
                            "{value:?} {needle:?} {operation} {start} {end}"
                        );
                    }
                }
                for count in [-1, 0, 1] {
                    let [narrow_new, wide_new] = both("<\u{e9}>");
                    let replaced = |text, needle, new| {
                        text_of(crate::string::typy_rt_str_replace(
                            text,
                            needle,
                            new,
                            inline(count).unwrap(),
                        ))
                    };
                    assert_eq!(
                        replaced(narrow_text, narrow_needle, narrow_new),
                        replaced(wide_text, wide_needle, wide_new),
                        "{value:?} {needle:?} {count}"
                    );
                }
            }
            for (chars, flags) in [(",c", 7), ("\u{e9},", 3), (",", 5), ("", 6)] {
                let [narrow_chars, wide_chars] = both(chars);
                let stripped = |text, chars| {
                    text_of(crate::string::typy_rt_str_strip(
                        text,
                        chars,
                        inline(flags).unwrap(),
                    ))
                };
                assert_eq!(
                    stripped(narrow_text, narrow_chars),
                    stripped(wide_text, wide_chars),
                    "{value:?} {chars:?} {flags}"
                );
            }
        }
    }
}

#[test]
fn nfr_21_join_of_bytes_joins_as_the_general_path_does() {
    let parts = ["x", "", "\u{e9}t\u{e9}"];
    for separator in [", ", "", "\u{e9}"] {
        let joined: Vec<String> = [false, true]
            .into_iter()
            .map(|widen| {
                let list = crate::list::new_list(crate::list::ItemKind::Object, parts.len());
                for part in parts {
                    let string = if widen { wide(part) } else { from_str(part) };
                    // SAFETY: the list is live and takes its own reference.
                    unsafe {
                        crate::list::typy_rt_list_append(list, string.expose_provenance() as u64);
                    }
                }
                let separator = if widen {
                    wide(separator)
                } else {
                    from_str(separator)
                };
                // SAFETY: the separator and the list are live.
                text_of(unsafe { crate::string::typy_rt_str_join(separator, list) })
            })
            .collect();
        assert_eq!(joined[0], joined[1], "{separator:?}");
    }
}

/// Returns the code points of a new string as Rust text.
fn text_of(string: *mut StrObject) -> String {
    text(string)
}

#[test]
fn nfr_21_strings_compare_by_their_stored_data_as_by_their_code_points() {
    let values = [
        "",
        "a",
        "ab",
        "b",
        "\u{e9}",
        "\u{20ac}",
        "\u{20ac}a",
        "\u{1f600}",
    ];
    for left in values {
        for right in values {
            for comparison in 0..6 {
                let compare = |left: *const StrObject, right: *const StrObject| {
                    // SAFETY: both strings are live.
                    unsafe {
                        crate::string::typy_rt_str_compare(left, right, inline(comparison).unwrap())
                    }
                };
                let expected = match comparison {
                    0 => left == right,
                    1 => left != right,
                    2 => left < right,
                    3 => left <= right,
                    4 => left > right,
                    _ => left >= right,
                };
                assert_eq!(
                    compare(from_str(left), from_str(right)),
                    expected,
                    "{left:?} {right:?} {comparison}"
                );
            }
        }
    }
}

#[test]
fn nfr_21_strings_of_one_character_or_none_are_shared_and_bytes_are_not() {
    let text = from_str("ab\u{e9}\u{20ac}");
    // SAFETY: every string is live; a shared one is immortal.
    unsafe {
        let first = crate::string::typy_rt_str_index(text, inline(0).unwrap()).value;
        let again = crate::string::typy_rt_chr(inline(97).unwrap()).value;
        assert!(core::ptr::eq(first, again));
        assert_eq!((*first).header.reference_count, crate::header::IMMORTAL);
        let sliced = crate::string::typy_rt_str_slice(
            text,
            inline(2).unwrap(),
            inline(3).unwrap(),
            inline(1).unwrap(),
            inline(3).unwrap(),
        )
        .value;
        assert!(core::ptr::eq(
            sliced,
            crate::string::character(0xe9).unwrap()
        ));
        let euro = crate::string::typy_rt_str_index(text, inline(3).unwrap()).value;
        assert_eq!(text_of(euro), "\u{20ac}");
        assert_ne!((*euro).header.reference_count, crate::header::IMMORTAL);
        let empty = crate::string::typy_rt_str_slice(
            text,
            inline(1).unwrap(),
            inline(1).unwrap(),
            inline(1).unwrap(),
            inline(3).unwrap(),
        )
        .value;
        assert!(core::ptr::eq(empty, crate::string::empty()));
        let bytes = crate::bytes::from_slice(b"xy");
        let part = crate::bytes::typy_rt_bytes_slice(
            bytes,
            inline(0).unwrap(),
            inline(1).unwrap(),
            inline(1).unwrap(),
            inline(3).unwrap(),
        )
        .value;
        assert_eq!((*part).header.type_id, crate::header::BYTES_TYPE_ID);
        assert!(!core::ptr::eq(
            part,
            crate::string::character(u32::from(b'x')).unwrap()
        ));
    }
}

#[test]
fn nfr_21_slices_of_bytes_give_what_slices_of_code_points_give() {
    let cases = ["abcdefgh", "\u{e9}t\u{e9} caf\u{e9}", "", "a"];
    // Each slice is a start, a stop, a step and the presence mask of the three.
    let bounds = [
        (0, 3, 1, 3),
        (2, 0, 1, 1),
        (1, -1, 2, 7),
        (-1, 0, -1, 7),
        (0, 0, -3, 4),
        (5, 100, 1, 3),
    ];
    for value in cases {
        let [narrow_text, wide_text] = both(value);
        for (start, stop, step, present) in bounds {
            let sliced = |text| {
                // SAFETY: the string is live.
                text_of(unsafe {
                    crate::string::typy_rt_str_slice(
                        text,
                        inline(start).unwrap(),
                        inline(stop).unwrap(),
                        inline(step).unwrap(),
                        inline(present).unwrap(),
                    )
                    .value
                })
            };
            assert_eq!(
                sliced(narrow_text),
                sliced(wide_text),
                "{value:?} {start} {stop} {step}"
            );
        }
    }
}
