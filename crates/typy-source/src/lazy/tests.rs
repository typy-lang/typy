//! Tests of `lazy`.

use super::{is_lazy, lazy_modules};
use crate::parse::parse;
use crate::settings::PythonFloor;
use crate::source_file::SourceFile;
use crate::syntax::Stmt;
use std::collections::BTreeSet;
use typy_diagnostics::FileId;

fn body(source: &str) -> Vec<Stmt> {
    let file = SourceFile::from_text(FileId(0), "app.py".into(), source);
    parse(&file, PythonFloor::V3_15)
        .expect("the source is valid")
        .body
}

fn names(listed: &[&str]) -> BTreeSet<String> {
    listed.iter().map(|name| (*name).to_owned()).collect()
}

#[test]
fn every_module_level_assignment_of_lazy_modules_lists_its_strings() {
    let cases: [(&str, &[&str]); 6] = [
        ("__lazy_modules__ = [\"a\", \"b\"]\n", &["a", "b"]),
        ("__lazy_modules__: list[str] = [\"a\"]\n", &["a"]),
        (
            "__lazy_modules__ = (\"t\",)\n__lazy_modules__ = [\"u\"]\n",
            &["t", "u"],
        ),
        ("x = __lazy_modules__ = [\"c\", 1]\n", &["c"]),
        ("other = [\"d\"]\n__lazy_modules__ = \"e\"\n", &[]),
        ("__lazy_modules__: list[str]\n", &[]),
    ];
    for (source, expected) in cases {
        assert_eq!(lazy_modules(&body(source)), names(expected), "{source}");
    }
}

#[test]
fn an_import_is_lazy_when_written_lazy_or_when_it_names_only_listed_modules() {
    let listed = names(&["a", "b", "p.q"]);
    let cases = [
        ("import a\n", true),
        ("import a, b\n", true),
        ("import a, c\n", false),
        ("lazy import c\n", true),
        ("from a import x\n", true),
        ("from p.q import x\n", true),
        ("from c import x\n", false),
        ("lazy from c import x\n", true),
        ("from . import a\n", false),
        ("x = 1\n", false),
    ];
    for (source, expected) in cases {
        assert_eq!(is_lazy(&body(source)[0], &listed), expected, "{source}");
    }
}
