//! Tests of a pattern's span, the names it binds and the expressions it holds.

use typy_diagnostics::FileId;

use super::Pattern;
use crate::syntax::statement::Stmt;
use crate::{PythonFloor, SourceFile, parse};

/// Returns the pattern of each case of the one `match` statement of `source`, with the text
/// its span covers.
fn patterns(source: &str) -> Vec<(Pattern, String)> {
    let file = SourceFile::from_text(FileId(0), "app.py".into(), source);
    let tree = parse(&file, PythonFloor::V3_12).unwrap();
    let Some(Stmt::Match { cases, .. }) = tree.body.into_iter().next() else {
        panic!("the source is one match statement");
    };
    cases
        .into_iter()
        .map(|case| {
            let span = case.pattern.span();
            let start = span.start().0 as usize;
            let end = span.end().0 as usize;
            (case.pattern, source[start..end].to_owned())
        })
        .collect()
}

#[test]
fn each_kind_of_pattern_has_the_span_of_its_text_and_its_names_and_expressions() {
    let source = "\
match subject:
    case 1:
        pass
    case None:
        pass
    case [first, *rest]:
        pass
    case {\"key\": value, **others}:
        pass
    case Point(x, y=0) as point:
        pass
    case 1 | 2:
        pass
    case _:
        pass
";
    let found = patterns(source);
    let texts: Vec<&str> = found.iter().map(|(_, text)| text.as_str()).collect();
    assert_eq!(
        texts,
        [
            "1",
            "None",
            "[first, *rest]",
            "{\"key\": value, **others}",
            "Point(x, y=0) as point",
            "1 | 2",
            "_"
        ]
    );
    let names: Vec<Vec<String>> = found
        .iter()
        .map(|(pattern, _)| {
            pattern
                .bound_names()
                .into_iter()
                .map(|name| name.name.clone())
                .collect()
        })
        .collect();
    assert_eq!(
        names,
        [
            vec![],
            vec![],
            vec!["first".to_owned(), "rest".to_owned()],
            vec!["value".to_owned(), "others".to_owned()],
            vec!["x".to_owned(), "point".to_owned()],
            vec![],
            vec![],
        ]
    );
    let expressions: Vec<usize> = found
        .iter()
        .map(|(pattern, _)| pattern.expressions().len())
        .collect();
    assert_eq!(expressions, [1, 0, 0, 1, 2, 2, 0]);
}

/// Collects the span of `pattern`, and of every pattern and expression inside it.
fn every_span(pattern: &Pattern, found: &mut Vec<typy_diagnostics::Span>) {
    found.push(pattern.span());
    match pattern {
        Pattern::Sequence { patterns, .. } | Pattern::Or { patterns, .. } => {
            for each in patterns {
                every_span(each, found);
            }
        }
        Pattern::Mapping { patterns, keys, .. } => {
            found.extend(keys.iter().map(crate::syntax::expression::Expr::span));
            for each in patterns {
                every_span(each, found);
            }
        }
        Pattern::Class {
            class,
            patterns,
            keywords,
            ..
        } => {
            found.push(class.span());
            for each in patterns.iter().chain(keywords.iter().map(|(_, each)| each)) {
                every_span(each, found);
            }
        }
        Pattern::As { pattern, .. } => {
            if let Some(inner) = pattern {
                every_span(inner, found);
            }
        }
        Pattern::Value { value, .. } => found.push(value.span()),
        Pattern::Singleton { .. } | Pattern::Star { .. } => {}
    }
}

#[test]
fn relocating_a_match_moves_every_pattern_to_the_new_place() {
    let source = "\
match subject:
    case [1, *rest] | {\"k\": None, **others}:
        pass
    case Point(x, y=_) as point:
        pass
";
    let file = SourceFile::from_text(FileId(0), "app.py".into(), source);
    let mut tree = parse(&file, PythonFloor::V3_12).unwrap();
    let place = typy_diagnostics::Span::empty(FileId(0), typy_diagnostics::TextOffset(3));
    tree.body[0].relocate(place);
    let Stmt::Match { cases, .. } = &tree.body[0] else {
        panic!("the source is one match statement");
    };
    let mut spans = Vec::new();
    for case in cases {
        every_span(&case.pattern, &mut spans);
    }
    assert!(spans.len() > 12, "{spans:?}");
    assert!(spans.iter().all(|span| *span == place), "{spans:?}");
}
