//! Tests of `codec`.

use super::{codec, handler};

#[test]
fn codec_names_resolve_as_cpython_resolves_them() {
    let cases = [
        ("utf-8", Some(0)),
        ("UTF8", Some(0)),
        ("U8", Some(0)),
        ("Utf 8", Some(0)),
        ("utf--8", Some(0)),
        ("-utf-8-", Some(0)),
        ("utf.8", None),
        ("cp65001", Some(0)),
        ("US-ASCII", Some(1)),
        ("646", Some(1)),
        ("ANSI_X3.4-1968", Some(1)),
        ("iso_646.irv_1991", Some(1)),
        ("latin-1", Some(2)),
        ("ISO8859-1", Some(2)),
        ("iso-ir-100", Some(2)),
        ("latin_1_", Some(2)),
        ("l_1", None),
        ("ascii.", None),
        ("utf-16", None),
        ("", None),
    ];
    for (name, expected) in cases {
        assert_eq!(codec(name), expected, "{name:?}");
    }
}

#[test]
fn handlers_are_found_by_their_exact_name() {
    assert_eq!(handler("strict", true), Some(0));
    assert_eq!(handler("surrogateescape", true), Some(5));
    assert_eq!(handler("xmlcharrefreplace", false), Some(4));
    assert_eq!(handler("xmlcharrefreplace", true), None);
    assert_eq!(handler("Strict", false), None);
    assert_eq!(handler("namereplace", false), None);
}
