//! Tests of parsing and conversion to TyPy's syntax tree.

use super::{parse, sentence};
use crate::settings::PythonFloor;
use crate::source_file::SourceFile;
use crate::syntax::{
    BinaryOperator, BoolOperator, CompareOperator, Constant, Expr, NodeKind, ParameterKind, Stmt,
    dump,
};
use typy_diagnostics::FileId;

fn file(source: &str) -> SourceFile {
    SourceFile::from_text(FileId(0), "app.py".into(), source)
}

fn body(source: &str) -> Vec<Stmt> {
    parse(&file(source), PythonFloor::V3_12)
        .expect("the source is valid")
        .body
}

fn only_expression(source: &str) -> Expr {
    match body(source).remove(0) {
        Stmt::Expr { value, .. } => value,
        other => panic!("not an expression statement: {other:?}"),
    }
}

#[test]
fn function_definition_keeps_parameters_annotations_and_defaults() {
    let source =
        "def f(a: int, /, b: int = 1, *rest: int, c: str, **options: int) -> None:\n    pass\n";
    let statements = body(source);
    let [
        Stmt::FunctionDef {
            name,
            parameters,
            returns,
            body,
            decorators,
            type_parameter_count,
            ..
        },
    ] = statements.as_slice()
    else {
        panic!("not one function definition");
    };
    assert_eq!(name.name, "f");
    let kinds: Vec<ParameterKind> = parameters.iter().map(|parameter| parameter.kind).collect();
    assert_eq!(
        kinds,
        [
            ParameterKind::PositionalOnly,
            ParameterKind::Normal,
            ParameterKind::VarPositional,
            ParameterKind::KeywordOnly,
            ParameterKind::VarKeyword,
        ]
    );
    assert!(
        parameters
            .iter()
            .all(|parameter| parameter.annotation.is_some())
    );
    assert!(parameters[1].default.is_some());
    assert!(matches!(
        returns,
        Some(Expr::Constant {
            value: Constant::None,
            ..
        })
    ));
    assert!(matches!(body.as_slice(), [Stmt::Pass { .. }]));
    assert_eq!(decorators.len(), 0);
    assert_eq!(*type_parameter_count, 0);
}

#[test]
fn decorators_and_type_parameters_are_recorded() {
    let source = "@cache\ndef first[T, U](xs: T) -> T:\n    return xs\n";
    let statements = body(source);
    let [
        Stmt::FunctionDef {
            decorators,
            type_parameter_count,
            ..
        },
    ] = statements.as_slice()
    else {
        panic!("not one function definition");
    };
    assert_eq!(decorators.len(), 1);
    assert_eq!(*type_parameter_count, 2);
}

#[test]
fn elif_chain_becomes_nested_if_statements() {
    let source = "if a:\n    pass\nelif b:\n    pass\nelse:\n    x = 1\n";
    let statements = body(source);
    let [Stmt::If { orelse, .. }] = statements.as_slice() else {
        panic!("not one if statement");
    };
    let [Stmt::If { orelse: inner, .. }] = orelse.as_slice() else {
        panic!("elif is not a nested if");
    };
    assert!(matches!(inner.as_slice(), [Stmt::Assign { .. }]));
}

#[test]
fn loops_keep_their_else_clauses() {
    let source =
        "for i in range(3):\n    continue\nelse:\n    pass\nwhile x:\n    break\nelse:\n    pass\n";
    let statements = body(source);
    assert!(matches!(&statements[0], Stmt::For { orelse, .. } if orelse.len() == 1));
    assert!(matches!(&statements[1], Stmt::While { orelse, .. } if orelse.len() == 1));
}

#[test]
fn assignments_of_every_form_are_converted() {
    let statements = body("x = y = 1\nx += 2\nz: int = 3\nw: int\n");
    assert!(matches!(&statements[0], Stmt::Assign { targets, .. } if targets.len() == 2));
    assert!(matches!(
        &statements[1],
        Stmt::AugAssign {
            operator: BinaryOperator::Add,
            ..
        }
    ));
    assert!(matches!(
        &statements[2],
        Stmt::AnnAssign {
            value: Some(_),
            simple: true,
            ..
        }
    ));
    assert!(matches!(
        &statements[3],
        Stmt::AnnAssign { value: None, .. }
    ));
}

#[test]
fn chained_comparison_keeps_every_operand() {
    let Expr::Compare {
        operators,
        comparators,
        ..
    } = only_expression("a < b <= c\n")
    else {
        panic!("not a comparison");
    };
    assert_eq!(operators, [CompareOperator::Lt, CompareOperator::LtE]);
    assert_eq!(comparators.len(), 2);
}

#[test]
fn calls_keep_positional_and_keyword_arguments() {
    let Expr::Call {
        arguments,
        keywords,
        ..
    } = only_expression("print(1, 2, sep='-', **more)\n")
    else {
        panic!("not a call");
    };
    assert_eq!(arguments.len(), 2);
    assert_eq!(
        keywords[0].name.as_ref().map(|name| name.name.as_str()),
        Some("sep")
    );
    assert!(keywords[1].name.is_none());
}

#[test]
fn constants_of_every_kind_are_converted() {
    let cases = [
        ("None\n", Constant::None),
        ("True\n", Constant::Bool(true)),
        ("0x_ff\n", Constant::Int("255".to_owned())),
        (
            "123456789012345678901234567890\n",
            Constant::Int("123456789012345678901234567890".to_owned()),
        ),
        ("1.5e3\n", Constant::Float(1500.0)),
        ("2j\n", Constant::Complex { imaginary: 2.0 }),
        ("'a' \"b\"\n", Constant::Str("ab".to_owned())),
        ("b'\\x00z'\n", Constant::Bytes(vec![0, b'z'])),
        ("...\n", Constant::Ellipsis),
    ];
    for (source, expected) in cases {
        let Expr::Constant { value, .. } = only_expression(source) else {
            panic!("not a constant: {source}");
        };
        assert_eq!(value, expected, "{source}");
    }
}

#[test]
fn operators_are_converted() {
    assert!(matches!(
        only_expression("a and b\n"),
        Expr::BoolOp {
            operator: BoolOperator::And,
            ..
        }
    ));
    assert!(matches!(only_expression("-a\n"), Expr::UnaryOp { .. }));
    assert!(matches!(
        only_expression("a if c else b\n"),
        Expr::IfExp { .. }
    ));
    assert!(matches!(only_expression("s[0]\n"), Expr::Subscript { .. }));
    for (source, expected) in [
        ("a // b\n", BinaryOperator::FloorDiv),
        ("a ** b\n", BinaryOperator::Pow),
    ] {
        assert!(
            matches!(only_expression(source), Expr::BinOp { operator, .. } if operator == expected)
        );
    }
}

#[test]
fn constructs_beyond_the_release_keep_their_kind_and_span() {
    let source = "\
class C:
    pass
async def a():
    pass
type X = int
match v:
    case _:
        pass
try:
    pass
except E:
    pass
with f:
    pass
del x
assert x
raise E
global g
";
    let kinds: Vec<NodeKind> = body(source)
        .iter()
        .map(|statement| match statement {
            Stmt::Other { kind, .. } => *kind,
            other => panic!("converted in detail: {other:?}"),
        })
        .collect();
    assert_eq!(
        kinds,
        [
            NodeKind::ClassDef,
            NodeKind::AsyncFunctionDef,
            NodeKind::TypeAlias,
            NodeKind::Match,
            NodeKind::Try,
            NodeKind::With,
            NodeKind::Delete,
            NodeKind::Assert,
            NodeKind::Raise,
            NodeKind::Global,
        ]
    );
}

#[test]
fn expressions_beyond_the_release_keep_their_kind() {
    let cases = [
        ("[1]\n", NodeKind::List),
        ("(1, 2)\n", NodeKind::Tuple),
        ("{1: 2}\n", NodeKind::Dict),
        ("{1}\n", NodeKind::Set),
        ("[x for x in y]\n", NodeKind::ListComp),
        ("(x for x in y)\n", NodeKind::GeneratorExp),
        ("lambda: 1\n", NodeKind::Lambda),
        ("f'{x}'\n", NodeKind::JoinedStr),
        ("a.b\n", NodeKind::Attribute),
        ("(y := 1)\n", NodeKind::NamedExpr),
    ];
    for (source, expected) in cases {
        assert!(
            matches!(only_expression(source), Expr::Other { kind, .. } if kind == expected),
            "{source}"
        );
    }
}

#[test]
fn imports_keep_names_levels_and_the_lazy_marker() {
    let statements = parse(
        &file("lazy import json\nfrom .. import a as b\n"),
        PythonFloor::V3_15,
    )
    .expect("lazy imports parse with floor 3.15")
    .body;
    assert!(
        matches!(&statements[0], Stmt::Import { is_lazy: true, names, .. } if names[0].name.name == "json")
    );
    assert!(matches!(
        &statements[1],
        Stmt::ImportFrom { level: 2, module: None, is_lazy: false, names, .. }
            if names[0].as_name.as_ref().is_some_and(|name| name.name == "b")
    ));
}

#[test]
fn lazy_import_below_floor_3_15_is_a_syntax_error() {
    let errors = parse(&file("lazy import json\n"), PythonFloor::V3_12).unwrap_err();
    assert_eq!(errors.len(), 1);
    assert_eq!(errors[0].code().to_string(), "E0103");
    assert_eq!(
        errors[0].rule().map(typy_diagnostics::RuleId::as_str),
        Some("STD-01")
    );
    assert!(
        errors[0].message().contains("3.15"),
        "{}",
        errors[0].message()
    );
}

#[test]
fn every_syntax_error_is_reported() {
    let errors = parse(&file("x = (\ny = ]\n"), PythonFloor::V3_12).unwrap_err();
    assert_ne!(errors.len(), 0);
    for error in &errors {
        assert_eq!(error.code().to_string(), "E0103");
        assert!(!error.message().ends_with('.'));
        assert!(
            !error.message().starts_with(char::is_uppercase),
            "{}",
            error.message()
        );
    }
}

#[test]
fn messages_become_one_lower_case_sentence() {
    assert_eq!(
        sentence("Expected an expression."),
        "expected an expression"
    );
    assert_eq!(sentence("  "), "invalid syntax");
}

#[test]
fn dump_writes_one_node_per_line_with_positions() {
    let source = "def f(n: int) -> int:\n    return n + 1\nprint(f(2))\n";
    let tree = parse(&file(source), PythonFloor::V3_12).unwrap();
    let expected = "\
Module
  FunctionDef f @1:1
    Parameter n Normal @1:7
      Name int @1:10
    returns
      Name int @1:18
    body
      Return @2:5
        BinOp Add @2:12
          Name n @2:12
          Constant Int(\"1\") @2:16
  Expr @3:1
    Call @3:1
      Name print @3:1
      Call @3:7
        Name f @3:7
        Constant Int(\"2\") @3:9
";
    assert_eq!(dump(&tree, &file(source)), expected);
}

#[test]
fn remaining_statement_and_expression_kinds_are_kept() {
    let source = "\
async def a():
    async for x in y:
        pass
    async with z:
        pass
    await w
def g():
    nonlocal n
    yield 1
    yield from h
try:
    pass
except* E:
    pass
x[1:2]
{x for x in y}
{x: x for x in y}
f(*args)
";
    let statements = body(source);
    let Stmt::Other {
        kind: NodeKind::AsyncFunctionDef,
        ..
    } = &statements[0]
    else {
        panic!("not an async function");
    };
    let dumped = dump(
        &parse(&file(source), PythonFloor::V3_12).unwrap(),
        &file(source),
    );
    for kind in [
        "AsyncFunctionDef",
        "TryStar",
        "Slice",
        "SetComp",
        "DictComp",
        "Starred",
    ] {
        assert!(dumped.contains(kind), "{kind} missing from\n{dumped}");
    }
    let Stmt::FunctionDef { body: inner, .. } = &statements[1] else {
        panic!("not a function");
    };
    let kinds: Vec<String> = inner
        .iter()
        .map(|statement| match statement {
            Stmt::Other { kind, .. }
            | Stmt::Expr {
                value: Expr::Other { kind, .. },
                ..
            } => kind.to_string(),
            other => format!("{other:?}"),
        })
        .collect();
    assert_eq!(kinds, ["Nonlocal", "Yield", "YieldFrom"]);
}

#[test]
fn template_strings_need_floor_3_14() {
    assert!(parse(&file("t'{x}'\n"), PythonFloor::V3_12).is_err());
    let tree = parse(&file("t'{x}'\n"), PythonFloor::V3_14).unwrap();
    assert!(matches!(
        &tree.body[0],
        Stmt::Expr {
            value: Expr::Other {
                kind: NodeKind::TemplateStr,
                ..
            },
            ..
        }
    ));
}

#[test]
fn dump_shows_every_converted_node_kind() {
    let source = "\
@trace
def f[T](a: int, b: int = 2, *rest: int, **more: int) -> int:
    return
x = y = -a
x += 1
z: int = 1 if a else 2
w: int
for i in range(3):
    continue
else:
    pass
while a and b:
    break
else:
    pass
if a < b:
    pass
else:
    pass
import os
from ..pkg import name
print(s[0], sep='-', **options)
x
";
    let parsed = parse(&file(source), PythonFloor::V3_12).unwrap();
    let dumped = dump(&parsed, &file(source));
    for line in [
        "FunctionDef f @1:1",
        "  decorator",
        "    type_params 1",
        "Parameter a Normal",
        "Parameter rest VarPositional",
        "Parameter more VarKeyword",
        "      default",
        "    returns",
        "    body",
        "Return @3:5",
        "  Assign @4:1",
        "UnaryOp USub @4:9",
        "  AugAssign Add @5:1",
        "  AnnAssign @6:1",
        "IfExp @6:10",
        "  AnnAssign @7:1",
        "  For @8:1",
        "Continue @9:5",
        "    orelse",
        "  While @12:1",
        "BoolOp And @12:7",
        "Break @13:5",
        "  If @16:1",
        "Compare [Lt] @16:4",
        "Pass @19:5",
        "  Import os @20:1",
        "  ImportFrom ..pkg name @21:1",
        "Subscript @22:7",
        "keyword sep @22:13",
        "keyword ** @22:22",
        "  Expr @23:1",
    ] {
        assert!(dumped.contains(line), "{line:?} missing from\n{dumped}");
    }
}

#[test]
fn lazy_imports_are_marked_in_the_dump() {
    let source = "lazy import json\nlazy from os import path\n";
    let parsed = parse(&file(source), PythonFloor::V3_15).unwrap();
    let dumped = dump(&parsed, &file(source));
    assert!(dumped.contains("  Import lazy json @1:1\n"), "{dumped}");
    assert!(
        dumped.contains("  ImportFrom lazy os path @2:1\n"),
        "{dumped}"
    );
}

#[test]
fn every_node_reports_the_span_it_was_built_with() {
    let source = "def f() -> None:\n    return\nx = 1\nx += 1\ny: int\nfor i in r:\n    break\nwhile a:\n    continue\nif a:\n    pass\nimport m\nfrom m import n\nprint(a or b, -a, a if b else c, a < b, s[0], f(x))\nclass C:\n    pass\n";
    let source_file = file(source);
    let tree = parse(&source_file, PythonFloor::V3_12).unwrap();
    for statement in &tree.body {
        let span = statement.span();
        assert!(span.start() <= span.end());
        if let Stmt::Expr {
            value: Expr::Call { arguments, .. },
            ..
        } = statement
        {
            for argument in arguments {
                let span = argument.span();
                let text = &source[span.start().0 as usize..span.end().0 as usize];
                assert_ne!(text, "");
            }
        }
    }
    let lines: Vec<u32> = tree
        .body
        .iter()
        .map(|statement| source_file.position(statement.span().start()).line)
        .collect();
    assert_eq!(lines, [1, 3, 4, 5, 6, 8, 10, 12, 13, 14, 15]);
}
