//! Tests of `open`.

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
