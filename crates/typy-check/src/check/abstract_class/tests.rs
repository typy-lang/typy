//! Tests of `check::abstract_class`.

use typy_diagnostics::Diagnostics;
use typy_source::syntax::Stmt;
use typy_source::{Module, ModuleName, PythonFloor, SourceFile};

use super::{gives_no_value, read};

fn module(text: &str) -> Module {
    let file = SourceFile::from_text(typy_diagnostics::FileId(0), "app.py".into(), text);
    Module {
        name: ModuleName::main(),
        file: typy_diagnostics::FileId(0),
        syntax: typy_source::parse(&file, PythonFloor::V3_12).expect("parses"),
        is_package: false,
        spec_name: None,
    }
}

/// Returns the bases and, for each method, the decorators of the module's first class.
fn first_class(module: &Module) -> (usize, Vec<usize>) {
    let Some(Stmt::ClassDef { bases, body, .. }) = module
        .syntax
        .body
        .iter()
        .find(|statement| matches!(statement, Stmt::ClassDef { .. }))
    else {
        panic!("no class");
    };
    let decorators = body
        .iter()
        .filter_map(|member| match member {
            Stmt::FunctionDef { decorators, .. } => Some(decorators.len()),
            _ => None,
        })
        .collect();
    (bases.len(), decorators)
}

#[test]
fn a_module_that_names_nothing_of_abc_is_left_as_it_is() {
    let mut diagnostics = Diagnostics::new();
    let (read, _) = read(
        &module("class Shape:\n    def area(self) -> float:\n        return 1.0\n"),
        &mut diagnostics,
    );
    assert!(read.is_none());
    assert!(diagnostics.is_empty());
}

#[test]
fn abc_leaves_the_bases_and_the_last_decorators_it_marks() {
    let source = "import abc\nfrom abc import ABC as Base\nclass Plain:\n    pass\nclass Shape(Plain, Base):\n    @abc.abstractmethod\n    def area(self) -> float: ...\n    @property\n    @abc.abstractmethod\n    def sides(self) -> int: ...\n    def name(self) -> str:\n        return ''\n";
    let mut diagnostics = Diagnostics::new();
    let (read, marks) = read(&module(source), &mut diagnostics);
    let read = read.expect("changed");
    let shape = read
        .syntax
        .body
        .iter()
        .find(|statement| matches!(statement, Stmt::ClassDef { name, .. } if name.name == "Shape"))
        .expect("Shape");
    let Stmt::ClassDef {
        name, bases, body, ..
    } = shape
    else {
        unreachable!("found as a class");
    };
    assert_eq!(bases.len(), 1);
    assert!(marks.derived.contains(&name.span));
    let marked: Vec<&str> = body
        .iter()
        .filter_map(|member| match member {
            Stmt::FunctionDef { name, .. } if marks.methods.contains(&name.span) => {
                Some(name.name.as_str())
            }
            _ => None,
        })
        .collect();
    assert_eq!(marked, ["area", "sides"]);
    assert!(diagnostics.is_empty());
}

#[test]
fn abstractmethod_above_a_method_decorator_is_reported_and_taken_out() {
    let source = "from abc import ABC, abstractmethod\nclass Sized(ABC):\n    @abstractmethod\n    @property\n    def size(self) -> int: ...\n";
    let mut diagnostics = Diagnostics::new();
    let (read, marks) = read(&module(source), &mut diagnostics);
    assert_eq!(first_class(&read.expect("changed")), (0, vec![1]));
    assert!(marks.methods.is_empty());
    let codes: Vec<String> = diagnostics
        .iter()
        .map(|diagnostic| diagnostic.code().to_string())
        .collect();
    assert_eq!(codes, ["E0434"]);
}

#[test]
fn abstractmethod_above_another_decorator_is_left_for_its_rules() {
    let source = "from abc import ABC, abstractmethod\nclass Sized(ABC):\n    @abstractmethod\n    @other\n    def size(self) -> int: ...\n";
    let mut diagnostics = Diagnostics::new();
    let (read, marks) = read(&module(source), &mut diagnostics);
    assert_eq!(first_class(&read.expect("changed")), (0, vec![2]));
    assert!(marks.methods.is_empty());
    assert!(diagnostics.is_empty());
}

#[test]
fn only_a_docstring_pass_or_ellipsis_gives_no_value() {
    let cases = [
        ("    ...\n", true),
        ("    pass\n", true),
        ("    \"\"\"Text.\"\"\"\n", true),
        ("    \"\"\"Text.\"\"\"\n    ...\n", true),
        ("    return 1\n", false),
        ("    raise NotImplementedError\n", false),
        ("    1\n", false),
    ];
    for (body, expected) in cases {
        let parsed = module(&format!("def f() -> int:\n{body}"));
        assert_eq!(gives_no_value(&parsed.syntax.body[0]), expected, "{body}");
    }
    // A statement other than a `def` has no body to give a value.
    let assignment = module("x = 1\n");
    assert!(!gives_no_value(&assignment.syntax.body[0]));
}
