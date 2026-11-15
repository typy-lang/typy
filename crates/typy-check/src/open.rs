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
    /// `r+`: an existing file, read and written.
    ReadPlus = 4,
    /// `w+`: a file emptied or created, read and written.
    WritePlus = 5,
    /// `a+`: a file created if missing, read, and written at its end.
    AppendPlus = 6,
    /// `x+`: a new file, read and written.
    CreatePlus = 7,
}

/// Why a mode is refused: CPython raises `ValueError` with this message for the mode,
/// whatever the file.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct ModeRefusal(pub(crate) String);

/// What a mode of `open` gives: how the file is opened, and whether it is binary.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct OpenMode {
    pub(crate) access: OpenAccess,
    pub(crate) binary: bool,
}

impl OpenMode {
    /// Returns the mode a binary file shows, as CPython 3.15's `FileIO` writes it: the
    /// access letter, `b`, and `+` for a mode that reads and writes.
    pub(crate) const fn binary_mode(self) -> &'static str {
        match self.access {
            OpenAccess::Read => "rb",
            OpenAccess::Write => "wb",
            OpenAccess::Append => "ab",
            OpenAccess::Create => "xb",
            OpenAccess::ReadPlus => "rb+",
            OpenAccess::WritePlus => "wb+",
            OpenAccess::AppendPlus => "ab+",
            OpenAccess::CreatePlus => "xb+",
        }
    }
}

/// Returns how `mode` opens a file, as CPython's `_io.open` checks it: each character one of
/// `rwxabt+` and given once, not both `t` and `b`, and exactly one of `r`, `w`, `a` and `x`.
pub(crate) fn open_access(mode: &str) -> Result<OpenMode, ModeRefusal> {
    let invalid = || ModeRefusal(format!("invalid mode: '{mode}'"));
    for (index, character) in mode.char_indices() {
        if !"rwxabt+".contains(character)
            || mode[index + character.len_utf8()..].contains(character)
        {
            return Err(invalid());
        }
    }
    let has = |character: char| mode.contains(character);
    if has('t') && has('b') {
        return Err(ModeRefusal(
            "can't have text and binary mode at once".to_owned(),
        ));
    }
    let accesses = ['r', 'w', 'a', 'x']
        .into_iter()
        .filter(|access| has(*access));
    let access = match accesses.collect::<Vec<_>>()[..] {
        [one] => one,
        [] => {
            return Err(ModeRefusal(
                "Must have exactly one of create/read/write/append mode and at most one plus"
                    .to_owned(),
            ));
        }
        _ => {
            return Err(ModeRefusal(
                "must have exactly one of create/read/write/append mode".to_owned(),
            ));
        }
    };
    let access = match (access, has('+')) {
        ('r', false) => OpenAccess::Read,
        ('w', false) => OpenAccess::Write,
        ('a', false) => OpenAccess::Append,
        (_, false) => OpenAccess::Create,
        ('r', true) => OpenAccess::ReadPlus,
        ('w', true) => OpenAccess::WritePlus,
        ('a', true) => OpenAccess::AppendPlus,
        (_, true) => OpenAccess::CreatePlus,
    };
    Ok(OpenMode {
        access,
        binary: has('b'),
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
            assert_eq!(
                open_access(mode).map(|found| found.access),
                Ok(access),
                "{mode}"
            );
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
                Err(ModeRefusal(message.to_owned())),
                "{mode}"
            );
        }
    }

    #[test]
    fn plus_modes_read_and_write() {
        for (mode, access) in [
            ("r+", OpenAccess::ReadPlus),
            ("+r", OpenAccess::ReadPlus),
            ("w+t", OpenAccess::WritePlus),
            ("a+", OpenAccess::AppendPlus),
            ("x+", OpenAccess::CreatePlus),
        ] {
            assert_eq!(
                open_access(mode).map(|found| found.access),
                Ok(access),
                "{mode}"
            );
        }
    }

    #[test]
    fn binary_modes_show_as_cpython_3_15_shows_them() {
        for (mode, shown) in [
            ("rb", "rb"),
            ("br", "rb"),
            ("wb+", "wb+"),
            ("+ab", "ab+"),
            ("xb", "xb"),
            ("r+b", "rb+"),
        ] {
            let found = open_access(mode).unwrap();
            assert!(found.binary, "{mode}");
            assert_eq!(found.binary_mode(), shown, "{mode}");
        }
        assert!(!open_access("rt").unwrap().binary);
    }
}
