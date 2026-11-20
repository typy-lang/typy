//! Tests of binding a call that unpacks, against the messages CPython 3.15 gives.

use super::{typy_rt_call_check, typy_rt_call_merge, typy_rt_call_name};
use crate::dict::{DictObject, typy_rt_dict_contains, typy_rt_dict_length, typy_rt_dict_new};
use crate::exception::{ExceptionKind, ExceptionObject};
use crate::list::ItemKind;
use crate::refcount::typy_rt_decrement;
use crate::string::{StrObject, code_point, from_str, length};
use crate::word::{inline, inline_value};

fn word(value: i64) -> i64 {
    inline(value).unwrap()
}

/// Returns a new dict of `str` keys and `int` values.
fn dict() -> *mut DictObject {
    let int = word(ItemKind::Int as i64);
    typy_rt_dict_new(word(ItemKind::Object as i64), 0, int, 0, word(0))
}

/// Returns a new dict of the keys, each with the value 1.
fn names(keys: &[&str]) -> *mut DictObject {
    let names = dict();
    for key in keys {
        let key = from_str(key);
        // SAFETY: the dict and the key are live; the dict takes its own reference.
        unsafe {
            crate::dict::typy_rt_dict_set(
                names,
                key.expose_provenance() as u64,
                word(1).cast_unsigned(),
            );
            typy_rt_decrement(key.cast());
        }
    }
    names
}

/// Returns the text of a raised `TypeError`, which is released, or `None` for no exception.
fn message(exception: *mut ExceptionObject) -> Option<String> {
    if exception.is_null() {
        return None;
    }
    // SAFETY: the exception is live and owned here.
    unsafe {
        assert_eq!((*exception).kind(), ExceptionKind::TypeError);
        let text: *const StrObject = (*exception).message;
        let found = (0..length(text))
            .map(|index| char::from_u32(code_point(text, index)).unwrap())
            .collect();
        typy_rt_decrement(exception.cast());
        Some(found)
    }
}

/// Checks a call of the callee `description` with `given` positional values and `keywords`.
fn check(description: &str, given: i64, keywords: &[&str]) -> Option<String> {
    let description = from_str(description);
    let names = if keywords.is_empty() {
        core::ptr::null_mut()
    } else {
        names(keywords)
    };
    // SAFETY: the description and the dict are live.
    let found =
        unsafe { typy_rt_call_check(description, word(given), names.expose_provenance() as u64) };
    // SAFETY: both are owned here.
    unsafe {
        typy_rt_decrement(description.cast());
        if !names.is_null() {
            typy_rt_decrement(names.cast());
        }
    }
    message(found)
}

const PAIR: &str = "pair\n__main__.pair\nN-a\nN-b";
const KEYED: &str = "keyed\n__main__.keyed\nN-a\nK=k";
const ONLY: &str = "only\n__main__.only\nP-a\nP-b";
const OPTIONAL: &str = "opt\n__main__.opt\nN-a\nN=b";
const NONE: &str = "none\n__main__.none";
const THREE: &str = "three\n__main__.three\nN-a\nN-b\nN-c";
const KEYWORDS: &str = "kw\n__main__.kw\nN-a\nK-k\nK-j";
const METHOD: &str = "Box.put\n__main__.Box.put\nN-self\nN-a\nN-b";
const NAMED: &str = "named\n__main__.named\nN-value\nN=count";
const SINK: &str = "sink\n__main__.sink\nA-args\nW-kwargs";

#[test]
fn a_call_that_fits_its_callee_binds() {
    for (description, given, keywords) in [
        (PAIR, 2, &[][..]),
        (PAIR, 1, &["b"][..]),
        (PAIR, 0, &["b", "a"][..]),
        (KEYED, 1, &[][..]),
        (OPTIONAL, 1, &[][..]),
        (KEYWORDS, 1, &["j", "k"][..]),
        (SINK, 3, &["k", "a"][..]),
        (NONE, 0, &[][..]),
    ] {
        assert_eq!(
            check(description, given, keywords),
            None,
            "{description} {given} {keywords:?}"
        );
    }
}

#[test]
#[allow(clippy::too_many_lines, reason = "one row per message CPython gives")]
fn a_call_that_does_not_fit_raises_cpython_s_type_error() {
    let cases = [
        (
            PAIR,
            3,
            &[][..],
            "pair() takes 2 positional arguments but 3 were given",
        ),
        (
            PAIR,
            1,
            &[][..],
            "pair() missing 1 required positional argument: 'b'",
        ),
        (
            PAIR,
            0,
            &[][..],
            "pair() missing 2 required positional arguments: 'a' and 'b'",
        ),
        (
            PAIR,
            2,
            &["b"][..],
            "pair() got multiple values for argument 'b'",
        ),
        (
            PAIR,
            0,
            &["a", "c"][..],
            "pair() got an unexpected keyword argument 'c'",
        ),
        (
            PAIR,
            0,
            &["a"][..],
            "pair() missing 1 required positional argument: 'b'",
        ),
        (
            KEYED,
            0,
            &["a", "k", "z"][..],
            "keyed() got an unexpected keyword argument 'z'",
        ),
        (
            ONLY,
            0,
            &["a", "b"][..],
            "only() got some positional-only arguments passed as keyword arguments: 'a, b'",
        ),
        (
            ONLY,
            2,
            &["c"][..],
            "only() got an unexpected keyword argument 'c'",
        ),
        (
            ONLY,
            3,
            &["z"][..],
            "only() got an unexpected keyword argument 'z'",
        ),
        (
            METHOD,
            4,
            &[][..],
            "Box.put() takes 3 positional arguments but 4 were given",
        ),
        (
            OPTIONAL,
            3,
            &[][..],
            "opt() takes from 1 to 2 positional arguments but 3 were given",
        ),
        (
            OPTIONAL,
            0,
            &["b"][..],
            "opt() missing 1 required positional argument: 'a'",
        ),
        (
            NONE,
            1,
            &[][..],
            "none() takes 0 positional arguments but 1 was given",
        ),
        (
            NONE,
            2,
            &[][..],
            "none() takes 0 positional arguments but 2 were given",
        ),
        (
            THREE,
            0,
            &[][..],
            "three() missing 3 required positional arguments: 'a', 'b', and 'c'",
        ),
        (
            KEYWORDS,
            2,
            &["k"][..],
            "kw() takes 1 positional argument but 2 positional arguments (and 1 keyword-only \
             argument) were given",
        ),
        (
            KEYWORDS,
            3,
            &["k", "j"][..],
            "kw() takes 1 positional argument but 3 positional arguments (and 2 keyword-only \
             arguments) were given",
        ),
        (
            KEYWORDS,
            1,
            &[][..],
            "kw() missing 2 required keyword-only arguments: 'k' and 'j'",
        ),
        (
            KEYWORDS,
            1,
            &["k"][..],
            "kw() missing 1 required keyword-only argument: 'j'",
        ),
    ];
    for (description, given, keywords, expected) in cases {
        assert_eq!(
            check(description, given, keywords).as_deref(),
            Some(expected),
            "{description} {given} {keywords:?}"
        );
    }
}

#[test]
fn an_unexpected_keyword_names_the_parameter_it_may_misspell() {
    let suggested = |keyword: &str, name: &str| {
        format!("named() got an unexpected keyword argument '{keyword}'. Did you mean '{name}'?")
    };
    let plain = |keyword: &str| format!("named() got an unexpected keyword argument '{keyword}'");
    let cases = [
        (0, vec!["cuont", "value"], suggested("cuont", "count")),
        (1, vec!["vlaue"], suggested("vlaue", "value")),
        (1, vec!["cont"], suggested("cont", "count")),
        (1, vec!["Count"], suggested("Count", "count")),
        (1, vec!["cOunt"], suggested("cOunt", "count")),
        (1, vec!["valeu"], suggested("valeu", "value")),
        (1, vec!["values"], suggested("values", "value")),
        (1, vec!["vale"], suggested("vale", "value")),
        (1, vec!["COUNT"], plain("COUNT")),
        (1, vec!["VALUE"], plain("VALUE")),
        (1, vec!["size"], plain("size")),
        (1, vec!["v"], plain("v")),
    ];
    for (given, keywords, expected) in cases {
        assert_eq!(
            check(NAMED, given, &keywords),
            Some(expected),
            "{keywords:?}"
        );
    }
}

#[test]
fn the_edit_distance_is_cpython_s() {
    use super::levenshtein;
    assert_eq!(levenshtein(b"value", b"vlaue", 10), 4);
    assert_eq!(levenshtein(b"count", b"COUNT", 10), 5);
    assert_eq!(levenshtein(b"abc", b"abc", 10), 0);
    assert_eq!(levenshtein(b"", b"abc", 1), 6);
    assert_eq!(levenshtein(b"a", b"abcdef", 3), 10);
    let long = [b'x'; 41];
    assert_eq!(levenshtein(&long, b"y", 3), 4);
}

#[test]
fn a_keyword_given_twice_raises_with_the_full_name() {
    let description = from_str(SINK);
    let names = names(&["k"]);
    let (k, other) = (from_str("k"), from_str("other"));
    // SAFETY: every object is live and owned here.
    unsafe {
        assert_eq!(message(typy_rt_call_name(description, names, other)), None);
        assert!(typy_rt_dict_contains(
            names,
            other.expose_provenance() as u64
        ));
        assert_eq!(
            message(typy_rt_call_name(description, names, k)).as_deref(),
            Some("__main__.sink() got multiple values for keyword argument 'k'")
        );
        for object in [description, k, other] {
            typy_rt_decrement(object.cast());
        }
        typy_rt_decrement(names.cast());
    }
}

#[test]
fn merging_a_dict_records_its_keys_and_passes_the_unnamed_to_kwargs() {
    let description = from_str("f\n__main__.f\nN-a\nP-b\nW-rest");
    let (names, merged, kwargs) = (names(&["x"]), dict(), dict());
    let source = names_with(&["a", "b", "c"]);
    // SAFETY: every object is live and owned here.
    unsafe {
        let found = typy_rt_call_merge(
            description,
            names,
            merged,
            source,
            kwargs.expose_provenance() as u64,
        );
        assert_eq!(message(found), None);
        assert_eq!(inline_value(typy_rt_dict_length(names)), 4);
        assert_eq!(inline_value(typy_rt_dict_length(merged)), 3);
        // `a` names a parameter; `b` names a positional-only one, which `**rest` takes.
        assert_eq!(inline_value(typy_rt_dict_length(kwargs)), 2);
        let again = typy_rt_call_merge(description, names, merged, source, 0);
        assert_eq!(
            message(again).as_deref(),
            Some("__main__.f() got multiple values for keyword argument 'a'")
        );
        typy_rt_decrement(description.cast());
        for dict in [names, merged, kwargs, source] {
            typy_rt_decrement(dict.cast());
        }
    }
}

/// Returns a new dict of the keys, each with the value 1, for a merge.
fn names_with(keys: &[&str]) -> *mut DictObject {
    names(keys)
}

#[test]
fn the_rarer_messages_are_cpython_s() {
    let keyword_only = "kw\n__main__.kw\nK-k";
    assert_eq!(
        check(keyword_only, 1, &["k"]).as_deref(),
        Some(
            "kw() takes 0 positional arguments but 1 positional argument (and 1 keyword-only \
             argument) were given"
        )
    );
    assert_eq!(
        check(ONLY, 3, &[]).as_deref(),
        Some("only() takes 2 positional arguments but 3 were given")
    );
    // Of two candidates as near, the first is named.
    let two = "two\n__main__.two\nN=abd\nN=abe";
    assert_eq!(
        check(two, 0, &["abc"]).as_deref(),
        Some("two() got an unexpected keyword argument 'abc'. Did you mean 'abd'?")
    );
}

#[test]
fn a_name_too_long_to_compare_gets_no_suggestion() {
    let long = "x".repeat(300);
    let with_long_parameter = format!("f\n__main__.f\nN-{long}y");
    assert_eq!(
        check(&with_long_parameter, 0, &["xy"]).as_deref(),
        Some("f() got an unexpected keyword argument 'xy'")
    );
    assert_eq!(
        check(NAMED, 1, &[long.as_str()]),
        Some(format!(
            "named() got an unexpected keyword argument '{long}'"
        ))
    );
    // CPython compares at most 749 candidates.
    let many = (0..750).fold(String::new(), |mut text, index| {
        use std::fmt::Write;
        let _ = write!(text, "\nN=p{index}");
        text
    });
    assert_eq!(
        check(&format!("f\n__main__.f{many}"), 0, &["p0x"]).as_deref(),
        Some("f() got an unexpected keyword argument 'p0x'")
    );
}

#[test]
fn a_byte_of_the_same_low_bits_is_no_change_of_case() {
    use super::levenshtein;
    assert_eq!(levenshtein(b"x!", b"xa", 10), 2);
    assert_eq!(levenshtein(b"xA", b"xa", 10), 1);
}

#[test]
fn merging_skips_removed_entries_and_may_have_no_kwargs() {
    let description = from_str("f\n__main__.f\nN-a");
    let (names, merged, source) = (dict(), dict(), names_with(&["gone", "kept", "a"]));
    let gone = from_str("gone");
    // SAFETY: every object is live and owned here.
    unsafe {
        crate::dict::typy_rt_dict_pop(source, gone.expose_provenance() as u64);
        let found = typy_rt_call_merge(description, names, merged, source, 0);
        assert_eq!(message(found), None);
        assert_eq!(inline_value(typy_rt_dict_length(merged)), 2);
        typy_rt_decrement(gone.cast());
        typy_rt_decrement(description.cast());
        for dict in [names, merged, source] {
            typy_rt_decrement(dict.cast());
        }
    }
}
