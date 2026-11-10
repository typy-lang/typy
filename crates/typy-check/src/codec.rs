//! The codecs and error handlers `str.encode` and `bytes.decode` accept, found by name as
//! CPython's `codecs.lookup` finds them (LIB-03). The names are checked when compiling, so
//! the runtime never meets one it does not have.

/// Returns the number of the codec `name` names, 0 `utf-8`, 1 `ascii` or 2 `latin-1`, or
/// `None` for a codec TyPy does not provide. The name is normalised as CPython does: lower
/// case, each run of characters other than letters, digits and `.` one `_`, none at the ends.
pub(crate) fn codec(name: &str) -> Option<i64> {
    const UTF8: [&str; 7] = [
        "utf_8",
        "u8",
        "utf",
        "utf8",
        "utf8_ucs2",
        "utf8_ucs4",
        "cp65001",
    ];
    const ASCII: [&str; 13] = [
        "ascii",
        "646",
        "ansi_x3.4_1968",
        "ansi_x3_4_1968",
        "ansi_x3.4_1986",
        "cp367",
        "csascii",
        "ibm367",
        "iso646_us",
        "iso_646.irv_1991",
        "iso_ir_6",
        "us",
        "us_ascii",
    ];
    const LATIN1: [&str; 13] = [
        "latin_1",
        "8859",
        "cp819",
        "csisolatin1",
        "ibm819",
        "iso8859",
        "iso8859_1",
        "iso_8859_1",
        "iso_8859_1_1987",
        "iso_ir_100",
        "l1",
        "latin",
        "latin1",
    ];
    let normal = normalised(name);
    [&UTF8[..], &ASCII[..], &LATIN1[..]]
        .iter()
        .position(|names| names.contains(&normal.as_str()))
        .and_then(|index| i64::try_from(index).ok())
}

fn normalised(name: &str) -> String {
    let mut normal = String::new();
    let mut gap = false;
    for character in name.chars() {
        if character.is_ascii_alphanumeric() || character == '.' {
            if gap && !normal.is_empty() {
                normal.push('_');
            }
            normal.push(character.to_ascii_lowercase());
            gap = false;
        } else {
            gap = true;
        }
    }
    normal
}

/// Returns the number of the error handler `name` names, in the order `strict`, `ignore`,
/// `replace`, `backslashreplace`, `xmlcharrefreplace` and `surrogateescape`, or `None` for one
/// TyPy does not provide or one that cannot handle decoding when `decoding` is set.
pub(crate) fn handler(name: &str, decoding: bool) -> Option<i64> {
    let handlers = [
        "strict",
        "ignore",
        "replace",
        "backslashreplace",
        "xmlcharrefreplace",
        "surrogateescape",
    ];
    let index = handlers.iter().position(|each| *each == name)?;
    if decoding && handlers[index] == "xmlcharrefreplace" {
        return None;
    }
    i64::try_from(index).ok()
}

#[cfg(test)]
mod tests {
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
}
