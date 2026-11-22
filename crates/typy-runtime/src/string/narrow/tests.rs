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
