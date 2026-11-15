//! Checks mapping patterns, `{"key": pattern, **rest}` (CST-06). The subject is tested as
//! `isinstance(subject, dict)`, each key as `key in subject`, each value is matched as
//! `subject[key]`, and `rest` takes a new dict of the other items.

use typy_diagnostics::Span;
use typy_source::syntax::{CompareOperator, Comprehension, Expr, Identifier, Pattern};

use super::BodyChecker;
use super::flow::test_facts;
use crate::typed::TypedPattern;
use crate::types::TypeObjectOf;

impl BodyChecker<'_> {
    /// Returns the steps of a mapping pattern: the class test, then for each key its test and
    /// its value's pattern, then the rest.
    pub(super) fn mapping_pattern(
        &mut self,
        whole: &Pattern,
        keys: &[Expr],
        patterns: &[Pattern],
        rest: Option<&Identifier>,
        subject: &Expr,
        span: Span,
    ) -> TypedPattern {
        let mut steps = Vec::new();
        if !self.subject_class_test(
            whole,
            subject,
            &[TypeObjectOf::Dict],
            "a mapping pattern",
            span,
            &mut steps,
        ) {
            return TypedPattern::All(steps);
        }
        for (key, pattern) in keys.iter().zip(patterns) {
            let contains = Expr::Compare {
                left: Box::new(key.clone()),
                operators: vec![CompareOperator::In],
                comparators: vec![subject.clone()],
                span,
            };
            let test = self.expression(&contains);
            self.add_facts(test_facts(&test).0);
            steps.push(TypedPattern::Test(test));
            let value = Expr::Subscript {
                value: Box::new(subject.clone()),
                index: Box::new(key.clone()),
                span,
            };
            self.part_pattern(pattern, &value, &mut steps);
        }
        if let Some(rest) = rest {
            let others = others(keys, subject, rest.span);
            let value = self.expression(&others);
            let target = Expr::Name {
                id: rest.name.clone(),
                span: rest.span,
            };
            if let Some(symbol) = self.assign(&target, value.ty.clone(), &value) {
                steps.push(TypedPattern::Bind(symbol, value));
            }
        }
        TypedPattern::All(steps)
    }
}

/// Returns `{k: v for k, v in subject.items() if k not in (keys)}`, whose names no program
/// can write.
fn others(keys: &[Expr], subject: &Expr, span: Span) -> Expr {
    let name = |id: &str| Expr::Name {
        id: id.to_owned(),
        span,
    };
    let items = Expr::Call {
        function: Box::new(Expr::Attribute {
            value: Box::new(subject.clone()),
            attribute: Identifier {
                name: "items".to_owned(),
                span,
            },
            span,
        }),
        arguments: Vec::new(),
        keywords: Vec::new(),
        span,
    };
    let ifs = if keys.is_empty() {
        Vec::new()
    } else {
        vec![Expr::Compare {
            left: Box::new(name("<rest key>")),
            operators: vec![CompareOperator::NotIn],
            comparators: vec![Expr::Tuple {
                elements: keys.to_vec(),
                span,
            }],
            span,
        }]
    };
    Expr::DictComp {
        key: Box::new(name("<rest key>")),
        value: Box::new(name("<rest value>")),
        generators: vec![Comprehension {
            target: Expr::Tuple {
                elements: vec![name("<rest key>"), name("<rest value>")],
                span,
            },
            iter: items,
            ifs,
            is_async: false,
            span,
        }],
        span,
    }
}
