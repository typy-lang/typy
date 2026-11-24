//! Tests of the import of `frozendict` a module of the Python floor 3.15 gets.

use typy_diagnostics::FileId;

use super::add_built_in_imports;
use crate::parse::parse;
use crate::settings::PythonFloor;
use crate::source_file::SourceFile;
use crate::syntax::Stmt;

/// Returns where the import lands in `source`'s body at `floor`, or `None` without one.
fn import_at(source: &str, floor: PythonFloor) -> Option<usize> {
    let file = SourceFile::from_text(FileId(0), "app.py".into(), source);
    let mut body = parse(&file, floor).expect("the source is valid").body;
    let before = body.len();
    add_built_in_imports(&mut body, source, floor, FileId(0));
    if body.len() == before {
        return None;
    }
    body.iter().position(|statement| {
        matches!(statement, Stmt::ImportFrom { module: Some(module), .. }
            if module.name == "_frozendict")
    })
}

#[test]
fn a_module_that_names_frozendict_imports_it_from_the_floor_3_15() {
    let source = "x = frozendict({1: 2})\n";
    assert_eq!(import_at(source, PythonFloor::V3_15), Some(0));
    assert_eq!(import_at(source, PythonFloor::V3_14), None);
    assert_eq!(
        import_at(
            "\"\"\"Prices.\"\"\"\nx = frozendict()\n",
            PythonFloor::V3_15
        ),
        Some(1)
    );
}

#[test]
fn a_module_that_binds_frozendict_or_names_another_word_imports_nothing() {
    for source in [
        "def frozendict() -> int:\n    return 1\n",
        "class frozendict:\n    pass\n",
        "from mine import frozendict\n",
        "import mine as frozendict\n",
        "frozendict = 1\n",
        "(a, frozendict) = (1, 2)\n",
        "for frozendict in [1]:\n    pass\n",
        "frozendicts = 1\nmy_frozendict = 2\n",
    ] {
        assert_eq!(import_at(source, PythonFloor::V3_15), None, "{source}");
    }
}
