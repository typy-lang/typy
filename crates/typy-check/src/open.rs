//! The modes and encodings of `open` (LIB-04, LIB-14, ADR-0019): a text mode as CPython's
//! `_io.open` reads it, with its messages for a mode it refuses, and the names of UTF-8.

/// How a text file is opened, numbered as the runtime takes it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OpenAccess {
    /// `r`: an existing file, read.
    Read = 0,
    /// `w`: a file emptied or created, written.
    Write = 1,
    /// `a`: a file created if missing, written at its end.
    Append = 2,
    /// `x`: a new file, written.
    Create = 3,
}

/// Why a mode is refused.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum ModeRefusal {
    /// CPython raises `ValueError` with this message for the mode, whatever the file.
    Fails(String),
    /// A binary mode or a `+` mode, planned for 0.4 (LIB-14).
    Planned(&'static str),
}

/// Returns how `mode` opens a file, as CPython's `_io.open` checks it: each character one of
/// `rwxabt+` and given once, not both `t` and `b`, and exactly one of `r`, `w`, `a` and `x`.
pub(crate) fn open_access(mode: &str) -> Result<OpenAccess, ModeRefusal> {
    let invalid = || ModeRefusal::Fails(format!("invalid mode: '{mode}'"));
    for (index, character) in mode.char_indices() {
        if !"rwxabt+".contains(character)
            || mode[index + character.len_utf8()..].contains(character)
        {
            return Err(invalid());
        }
    }
    let has = |character: char| mode.contains(character);
    if has('t') && has('b') {
        return Err(ModeRefusal::Fails(
            "can't have text and binary mode at once".to_owned(),
        ));
    }
    let accesses = ['r', 'w', 'a', 'x']
        .into_iter()
        .filter(|access| has(*access));
    let access = match accesses.collect::<Vec<_>>()[..] {
        [one] => one,
        [] => {
            return Err(ModeRefusal::Fails(
                "Must have exactly one of create/read/write/append mode and at most one plus"
                    .to_owned(),
            ));
        }
        _ => {
            return Err(ModeRefusal::Fails(
                "must have exactly one of create/read/write/append mode".to_owned(),
            ));
        }
    };
    if has('b') {
        return Err(ModeRefusal::Planned("a binary mode"));
    }
    if has('+') {
        return Err(ModeRefusal::Planned("a mode with `+`"));
    }
    Ok(match access {
        'r' => OpenAccess::Read,
        'w' => OpenAccess::Write,
        'a' => OpenAccess::Append,
        _ => OpenAccess::Create,
    })
}

/// Returns whether CPython's codec lookup finds UTF-8 for `name`: lowercased, each run of
/// characters other than letters, digits and `.` read as one `_`, without one at either end.
pub(crate) fn is_utf8(name: &str) -> bool {
    let mut normalized = String::new();
    let mut gap = false;
    for character in name.chars().flat_map(char::to_lowercase) {
        if character.is_alphanumeric() || character == '.' {
            if gap && !normalized.is_empty() {
                normalized.push('_');
            }
            gap = false;
            normalized.push(character);
        } else {
            gap = true;
        }
    }
    matches!(
        normalized.as_str(),
        "utf_8" | "utf8" | "u8" | "utf" | "cp65001" | "csutf8" | "utf8_ucs2" | "utf8_ucs4"
    )
}

/// The methods and attributes of a text file planned for 0.4 (LIB-14).
const PLANNED_NAMES: [&str; 11] = [
    "closed", "fileno", "isatty", "mode", "name", "readable", "seek", "seekable", "tell",
    "truncate", "writable",
];

/// Returns the error message for a name of a text file that TyPy 0.3 does not have: planned
/// for 0.4, or not provided.
pub(crate) fn missing_file_name(name: &str) -> (String, bool) {
    if PLANNED_NAMES.contains(&name) {
        (
            format!("`{name}` of a text file is planned for TyPy 0.4 (LIB-14)"),
            true,
        )
    } else {
        (
            format!("a TextIOWrapper has no attribute `{name}` in TyPy"),
            false,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::{ModeRefusal, OpenAccess, is_utf8, open_access};

    #[test]
    fn text_modes_give_their_access() {
        for (mode, access) in [
            ("r", OpenAccess::Read),
            ("rt", OpenAccess::Read),
            ("tr", OpenAccess::Read),
            ("w", OpenAccess::Write),
            ("wt", OpenAccess::Write),
            ("a", OpenAccess::Append),
            ("x", OpenAccess::Create),
            ("xt", OpenAccess::Create),
        ] {
            assert_eq!(open_access(mode), Ok(access), "{mode}");
        }
    }

    #[test]
    fn modes_cpython_refuses_give_its_message() {
        for (mode, message) in [
            ("z", "invalid mode: 'z'"),
            ("rr", "invalid mode: 'rr'"),
            ("rU", "invalid mode: 'rU'"),
            ("\u{e9}", "invalid mode: '\u{e9}'"),
            ("bt", "can't have text and binary mode at once"),
            (
                "rw",
                "must have exactly one of create/read/write/append mode",
            ),
            (
                "",
                "Must have exactly one of create/read/write/append mode and at most one plus",
            ),
            (
                "t",
                "Must have exactly one of create/read/write/append mode and at most one plus",
            ),
            (
                "+",
                "Must have exactly one of create/read/write/append mode and at most one plus",
            ),
        ] {
            assert_eq!(
                open_access(mode),
                Err(ModeRefusal::Fails(message.to_owned())),
                "{mode}"
            );
        }
    }

    #[test]
    fn binary_and_plus_modes_are_planned() {
        for (mode, what) in [
            ("rb", "a binary mode"),
            ("wb+", "a binary mode"),
            ("r+", "a mode with `+`"),
            ("a+t", "a mode with `+`"),
        ] {
            assert_eq!(open_access(mode), Err(ModeRefusal::Planned(what)), "{mode}");
        }
    }

    #[test]
    fn the_names_of_utf_8_are_found_as_cpython_finds_them() {
        for name in [
            "utf-8",
            "UTF-8",
            "utf8",
            "U8",
            "utf",
            "Utf_8",
            "utf-8 ",
            "  utf8",
            "UTF8-UCS2",
            "cp65001",
            "utf--8",
            "csutf8",
        ] {
            assert!(is_utf8(name), "{name}");
        }
        for name in ["utf.8", "utf_8_sig", "latin-1", "ascii", "", "nope"] {
            assert!(!is_utf8(name), "{name}");
        }
    }
}
