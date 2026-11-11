//! Tests of parsing and conversion to TyPy's syntax tree.

use super::{parse, sentence};
use crate::settings::PythonFloor;
use crate::source_file::SourceFile;
use crate::syntax::{
    BinaryOperator, BoolOperator, CompareOperator, Constant, Expr, FStringPart, NodeKind,
    ParameterKind, Stmt, dump,
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
async def a():
    pass
type X = int
match v:
    case _:
        pass
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
            NodeKind::AsyncFunctionDef,
            NodeKind::TypeAlias,
            NodeKind::Match,
            NodeKind::Global,
        ]
    );
}

#[test]
fn expressions_beyond_the_release_keep_their_kind() {
    let cases = [
        ("(x for x in y)\n", NodeKind::GeneratorExp),
        ("lambda: 1\n", NodeKind::Lambda),
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
fn fstrings_keep_their_text_fields_conversions_specs_and_debug_text() {
    let source = "f'a{{{x!r:>{w}.2f}' 'b' f'{y = }{z:}'\n";
    let Expr::FString { parts, .. } = only_expression(source) else {
        panic!("an f-string");
    };
    assert_eq!(parts.len(), 5);
    assert_eq!(parts[0], FStringPart::Literal("a{".to_owned()));
    let FStringPart::Field(field) = &parts[1] else {
        panic!("a field");
    };
    assert_eq!(field.conversion, Some('r'));
    assert!(matches!(&field.value, Expr::Name { id, .. } if id == "x"));
    let spec = field.spec.as_ref().expect("a spec");
    assert_eq!(spec[0], FStringPart::Literal(">".to_owned()));
    assert!(matches!(&spec[1], FStringPart::Field(_)));
    assert_eq!(spec[2], FStringPart::Literal(".2f".to_owned()));
    assert_eq!(parts[2], FStringPart::Literal("b".to_owned()));
    let FStringPart::Field(debug) = &parts[3] else {
        panic!("a field");
    };
    assert_eq!(debug.debug.as_deref(), Some("y = "));
    assert_eq!(debug.spec, None);
    let FStringPart::Field(empty) = &parts[4] else {
        panic!("a field");
    };
    assert_eq!(empty.spec, Some(Vec::new()));
    let dumped = dump(
        &parse(&file(source), PythonFloor::V3_12).unwrap(),
        &file(source),
    );
    for line in [
        "FString @1:1",
        "Literal \"a{\"",
        "Field !r @1:6",
        "spec",
        "Field debug \"y = \"",
    ] {
        assert!(dumped.contains(line), "{line} in\n{dumped}");
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
    for kind in ["AsyncFunctionDef", "TryStar", "Slice", "SetComp", "Starred"] {
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
fn classes_keep_their_bases_keywords_decorators_and_body() {
    let source =
        "@d\nclass C[T](B, metaclass=M):\n    x: int\n    def f(self) -> None:\n        pass\n";
    let parsed = parse(&file(source), PythonFloor::V3_12).unwrap();
    let Stmt::ClassDef {
        name,
        bases,
        keyword_count,
        body,
        decorators,
        type_parameter_count,
        ..
    } = &parsed.body[0]
    else {
        panic!("a class: {:?}", parsed.body[0]);
    };
    assert_eq!(name.name, "C");
    assert_eq!((bases.len(), *keyword_count, body.len()), (1, 1, 2));
    assert_eq!((decorators.len(), *type_parameter_count), (1, 1));
    let dumped = dump(&parsed, &file(source));
    for line in [
        "ClassDef C @1:1",
        "  decorator",
        "  type_params 1",
        "  base",
        "    Name B",
        "  keywords 1",
        "  body",
        "    AnnAssign @3:5",
    ] {
        assert!(dumped.contains(line), "{line} in\n{dumped}");
    }
    let plain = parse(&file("class D:\n    pass\n"), PythonFloor::V3_12).unwrap();
    assert_eq!(
        dump(&plain, &file("class D:\n    pass\n")),
        "Module\n  ClassDef D @1:1\n    body\n      Pass @2:5\n"
    );
    assert!(
        matches!(&plain.body[0], Stmt::ClassDef { bases, keyword_count: 0, .. } if bases.is_empty())
    );
}

#[test]
fn string_annotations_hold_the_expression_they_quote() {
    let source = "def f(a: \"Node | None\") -> 'int':\n    b: \"x(\" = 1\n";
    let parsed = parse(&file(source), PythonFloor::V3_12).unwrap();
    let dumped = dump(&parsed, &file(source));
    for line in [
        "Quoted @1:10",
        "BinOp BitOr @1:11",
        "Name Node @1:11",
        "Quoted @1:28",
    ] {
        assert!(dumped.contains(line), "{line} in\n{dumped}");
    }
    assert!(dumped.contains("Constant Str(\"x(\")"), "{dumped}");
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

#[test]
fn async_loops_and_with_statements_keep_their_kind() {
    let source = "async for x in y:\n    pass\nasync with z:\n    pass\n";
    let kinds: Vec<NodeKind> = body(source)
        .iter()
        .map(|statement| match statement {
            Stmt::Other { kind, .. } => *kind,
            other => panic!("converted in detail: {other:?}"),
        })
        .collect();
    assert_eq!(kinds, [NodeKind::AsyncFor, NodeKind::AsyncWith]);
}

#[test]
fn a_parameter_without_annotation_is_dumped_without_one() {
    let source = "def f(n):\n    pass\n";
    let dumped = dump(
        &parse(&file(source), PythonFloor::V3_12).unwrap(),
        &file(source),
    );
    let expected =
        "Module\n  FunctionDef f @1:1\n    Parameter n Normal @1:7\n    body\n      Pass @2:5\n";
    assert_eq!(dumped, expected);
}

#[test]
fn tuples_are_converted_with_their_elements() {
    for source in ["(1, x)\n", "1, x\n"] {
        let Expr::Tuple { elements, .. } = only_expression(source) else {
            panic!("not a tuple: {source:?}");
        };
        assert_eq!(elements.len(), 2, "{source:?}");
        assert!(matches!(elements[1], Expr::Name { ref id, .. } if id == "x"));
    }
    assert!(
        matches!(only_expression("()\n"), Expr::Tuple { ref elements, .. } if elements.is_empty())
    );
}

#[test]
fn lists_and_attributes_are_converted() {
    let Expr::List { elements, .. } = only_expression("[1, x]\n") else {
        panic!("not a list");
    };
    assert_eq!(elements.len(), 2);
    let Expr::Call { function, .. } = only_expression("items.append(1)\n") else {
        panic!("not a call");
    };
    let Expr::Attribute {
        value, attribute, ..
    } = *function
    else {
        panic!("not an attribute");
    };
    assert!(matches!(*value, Expr::Name { ref id, .. } if id == "items"));
    assert_eq!(attribute.name, "append");
}

#[test]
fn every_node_kind_is_dumped_at_its_depth_with_its_position() {
    let source = "@trace\nclass Box(Base, metaclass=M):\n    size: int\n\n    def put[T](self, item: int, /, *rest: int, flag: bool = True, **more: int) -> None:\n        self.items = [x * 2 for x in rest if x if not x]\n        pairs = {k: v for k, v in more.items()}\n        unique = {y async for y in rest}\n        text = f\"{item!r:>{size}.2f} {flag=}\"\n        part = rest[1:2:3]\n        same = {1, 2}\n        table = {\"a\": 1, **more}\n        print(a < b <= c, -item, item if flag else 0, x or y, lambda: 1)\n";
    let expected = "Module\n  ClassDef Box @1:1\n    decorator\n      Name trace @1:2\n    base\n      Name Base @2:11\n    keywords 1\n    body\n      AnnAssign @3:5\n        Name size @3:5\n        Name int @3:11\n      FunctionDef put @5:5\n        type_params 1\n        Parameter self PositionalOnly @5:16\n        Parameter item PositionalOnly @5:22\n          Name int @5:28\n        Parameter rest VarPositional @5:36\n          Name int @5:43\n        Parameter flag KeywordOnly @5:48\n          Name bool @5:54\n          default\n            Constant Bool(true) @5:61\n        Parameter more VarKeyword @5:67\n          Name int @5:75\n        returns\n          Constant None @5:83\n        body\n          Assign @6:9\n            Attribute items @6:9\n              Name self @6:9\n            ListComp @6:22\n              BinOp Mult @6:23\n                Name x @6:23\n                Constant Int(\"2\") @6:27\n              Comprehension @6:29\n                Name x @6:33\n                Name rest @6:38\n                Name x @6:46\n                UnaryOp Not @6:51\n                  Name x @6:55\n          Assign @7:9\n            Name pairs @7:9\n            DictComp @7:17\n              Name k @7:18\n              Name v @7:21\n              Comprehension @7:23\n                Tuple @7:27\n                  Name k @7:27\n                  Name v @7:30\n                Call @7:35\n                  Attribute items @7:35\n                    Name more @7:35\n          Assign @8:9\n            Name unique @8:9\n            SetComp @8:18\n              Name y @8:19\n              AsyncComprehension @8:21\n                Name y @8:31\n                Name rest @8:36\n          Assign @9:9\n            Name text @9:9\n            FString @9:16\n              Field !r @9:18\n                Name item @9:19\n                spec\n                  Literal \">\"\n                  Field @9:27\n                    Name size @9:28\n                  Literal \".2f\"\n              Literal \" \"\n              Field debug \"flag=\" @9:38\n                Name flag @9:39\n          Assign @10:9\n            Name part @10:9\n            Subscript @10:16\n              Name rest @10:16\n              Slice @10:21\n                Constant Int(\"1\") @10:21\n                Constant Int(\"2\") @10:23\n                Constant Int(\"3\") @10:25\n          Assign @11:9\n            Name same @11:9\n            Set @11:16\n              Constant Int(\"1\") @11:17\n              Constant Int(\"2\") @11:20\n          Assign @12:9\n            Name table @12:9\n            Dict @12:17\n              Constant Str(\"a\") @12:18\n              Constant Int(\"1\") @12:23\n              Unpack @12:28\n              Name more @12:28\n          Expr @13:9\n            Call @13:9\n              Name print @13:9\n              Compare [Lt, LtE] @13:15\n                Name a @13:15\n                Name b @13:19\n                Name c @13:24\n              UnaryOp USub @13:27\n                Name item @13:28\n              IfExp @13:34\n                Name flag @13:42\n                Name item @13:34\n                Constant Int(\"0\") @13:52\n              BoolOp Or @13:55\n                Name x @13:55\n                Name y @13:60\n              Lambda @13:63\n";
    let parsed = parse(&file(source), PythonFloor::V3_12).unwrap();
    assert_eq!(dump(&parsed, &file(source)), expected);
}

#[test]
fn each_form_of_raise_is_dumped_with_its_operands() {
    let source = "raise\nraise E\nraise E(1) from c\n";
    let expected = "Module\n  Raise @1:1\n  Raise @2:1\n    Name E @2:7\n  Raise from @3:1\n    Call @3:7\n      Name E @3:7\n      Constant Int(\"1\") @3:9\n    Name c @3:17\n";
    let parsed = parse(&file(source), PythonFloor::V3_12).unwrap();
    assert_eq!(dump(&parsed, &file(source)), expected);
}

#[test]
fn a_try_statement_keeps_its_clauses_in_order() {
    let source = "try:\n    pass\nexcept (A, B) as e:\n    raise\nexcept:\n    pass\nelse:\n    pass\nfinally:\n    pass\n";
    let expected = "Module\n  Try @1:1\n    body\n      Pass @2:5\n    except as e @3:1\n      Tuple @3:8\n        Name A @3:9\n        Name B @3:12\n      body\n        Raise @4:5\n    except @5:1\n      body\n        Pass @6:5\n    orelse\n      Pass @8:5\n    finally\n      Pass @10:5\n";
    let parsed = parse(&file(source), PythonFloor::V3_12).unwrap();
    assert_eq!(dump(&parsed, &file(source)), expected);
}

#[test]
fn assert_keeps_its_test_and_message() {
    let source = "assert x\nassert x, 'm'\n";
    let expected = "Module\n  Assert @1:1\n    Name x @1:8\n  Assert @2:1\n    Name x @2:8\n    Constant Str(\"m\") @2:11\n";
    let parsed = parse(&file(source), PythonFloor::V3_12).unwrap();
    assert_eq!(dump(&parsed, &file(source)), expected);
}

#[test]
fn a_with_statement_keeps_each_item_and_its_target() {
    let source = "with a() as (x, y), b:\n    pass\n";
    let expected = "Module\n  With @1:1\n    Call @1:6\n      Name a @1:6\n    as\n      Tuple @1:13\n        Name x @1:14\n        Name y @1:17\n    Name b @1:21\n    body\n      Pass @2:5\n";
    let parsed = parse(&file(source), PythonFloor::V3_12).unwrap();
    assert_eq!(dump(&parsed, &file(source)), expected);
}

#[test]
fn del_keeps_each_target() {
    let source = "del xs[0], d['k']\n";
    let expected = "Module\n  Delete @1:1\n    Subscript @1:5\n      Name xs @1:5\n      Constant Int(\"0\") @1:8\n    Subscript @1:12\n      Name d @1:12\n      Constant Str(\"k\") @1:14\n";
    let parsed = parse(&file(source), PythonFloor::V3_12).unwrap();
    assert_eq!(dump(&parsed, &file(source)), expected);
}

#[test]
fn every_statement_kind_is_dumped_at_its_depth_with_its_position() {
    let source = "@cache\ndef f(n: int) -> int:\n    total = 0\n    total += n\n    for i in range(n):\n        continue\n    else:\n        pass\n    while n:\n        break\n    else:\n        pass\n    if n:\n        return n\n    else:\n        return 0\nimport os\nfrom ..pkg import name\nprint(s[0], sep=\"-\", **options)\n";
    let expected = "Module\n  FunctionDef f @1:1\n    decorator\n      Name cache @1:2\n    Parameter n Normal @2:7\n      Name int @2:10\n    returns\n      Name int @2:18\n    body\n      Assign @3:5\n        Name total @3:5\n        Constant Int(\"0\") @3:13\n      AugAssign Add @4:5\n        Name total @4:5\n        Name n @4:14\n      For @5:5\n        Name i @5:9\n        Call @5:14\n          Name range @5:14\n          Name n @5:20\n        body\n          Continue @6:9\n        orelse\n          Pass @8:9\n      While @9:5\n        Name n @9:11\n        body\n          Break @10:9\n        orelse\n          Pass @12:9\n      If @13:5\n        Name n @13:8\n        body\n          Return @14:9\n            Name n @14:16\n        orelse\n          Return @16:9\n            Constant Int(\"0\") @16:16\n  Import os @17:1\n  ImportFrom ..pkg name @18:1\n  Expr @19:1\n    Call @19:1\n      Name print @19:1\n      Subscript @19:7\n        Name s @19:7\n        Constant Int(\"0\") @19:9\n      keyword sep @19:13\n        Constant Str(\"-\") @19:17\n      keyword ** @19:22\n        Name options @19:24\n";
    let parsed = parse(&file(source), PythonFloor::V3_12).unwrap();
    assert_eq!(dump(&parsed, &file(source)), expected);
}
