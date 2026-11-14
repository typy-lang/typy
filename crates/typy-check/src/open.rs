//! The modes of `open` (LIB-04, LIB-14, ADR-0019): a text mode as CPython's `_io.open` reads
//! it, with its messages for a mode it refuses.

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

#[cfg(test)]
mod tests {
    use super::{ModeRefusal, OpenAccess, open_access};

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
}
