//! Tests of `body::first_add`.

use typy_diagnostics::FileId;
use typy_source::syntax::Stmt;
use typy_source::{PythonFloor, SourceFile, parse};

use super::assigns_empty;

/// Returns the statements of `source`.
fn statements(source: &str) -> Vec<Stmt> {
    let file = SourceFile::from_text(FileId(0), "app.py".into(), source);
    parse(&file, PythonFloor::V3_12)
        .expect("the source parses")
        .body
}

#[test]
fn an_empty_collection_assigned_to_a_name_is_found_in_nested_blocks() {
    let found = [
        "items = []\n",
        "items = {}\n",
        "items = set()\n",
        "items = dict()\n",
        "items = list()\n",
        "if True:\n    items = []\n",
        "for i in range(2):\n    while True:\n        items = []\n",
        "try:\n    pass\nexcept ValueError:\n    items = []\n",
        "with open('a') as f:\n    items = []\n",
        "match 1:\n    case 1:\n        items = []\n",
    ];
    for source in found {
        assert!(statements(source).iter().any(assigns_empty), "{source}");
    }
}

#[test]
fn other_assignments_and_nested_functions_are_not_scanned() {
    let not_found = [
        "items = [1]\n",
        "items = {1: 2}\n",
        "items = set([1])\n",
        "items = dict(a=1)\n",
        "items = ()\n",
        "items.values = []\n",
        "a = b = []\n",
        "a, b = [], []\n",
        "items: list[int] = []\n",
        "print([])\n",
        "def inner() -> None:\n    items = []\n",
        "class Box:\n    items = []\n",
    ];
    for source in not_found {
        assert!(!statements(source).iter().any(assigns_empty), "{source}");
    }
}
