//! Checks sequence patterns, `[first, *rest]` and `(x, y)` (CST-06). The subject is tested as
//! `isinstance(subject, (list, tuple, range))` and by its length, and each item is matched as
//! `subject[i]`; a `str` or `bytes` subject is not a sequence to a pattern, as in CPython.

use typy_diagnostics::{Diagnostic, DiagnosticCode, RuleId, Span};
use typy_source::syntax::{CompareOperator, Constant, Expr, Identifier, Pattern, UnaryOperator};

use super::BodyChecker;
use super::flow::test_facts;
use crate::builtin::Builtin;
use crate::typed::{TypedConstant, TypedExpr, TypedExprKind, TypedPattern};
use crate::types::{Type, TypeObjectOf};

impl BodyChecker<'_> {
    /// Returns the steps of a sequence pattern: the class test, the length test, then the
    /// items in order.
    pub(super) fn sequence_pattern(
        &mut self,
        whole: &Pattern,
        patterns: &[Pattern],
        subject: &Expr,
        span: Span,
    ) -> TypedPattern {
        let mut steps = Vec::new();
        if !self.subject_class_test(
            whole,
            subject,
            &[TypeObjectOf::List, TypeObjectOf::Tuple, TypeObjectOf::Range],
            "a sequence pattern",
            span,
            &mut steps,
        ) {
            return TypedPattern::All(steps);
        }
        let star = patterns
            .iter()
            .position(|each| matches!(each, Pattern::Star { .. }));
        let fixed = patterns.len() - usize::from(star.is_some());
        // A named tuple matches as the tuple of its fields (CLS-27).
        let value = self.tuple_expression(subject);
        let known_length = match &value.ty {
            Type::Tuple(items) => Some(items.len()),
            Type::List(_) | Type::TupleOf(_) | Type::Range => None,
            other => {
                let error = self.error(
                    Diagnostic::new(
                        DiagnosticCode::error(201),
                        span,
                        format!(
                            "a sequence pattern on {} is planned for TyPy 1.0",
                            other.with_article()
                        ),
                    )
                    .with_rule(RuleId::CST_06),
                    span,
                );
                self.bind_after_error(whole, &error);
                return TypedPattern::All(steps);
            }
        };
        match known_length {
            Some(length) if length == fixed || (star.is_some() && length > fixed) => {}
            Some(_) => {
                let error = self.error(
                    Diagnostic::new(
                        DiagnosticCode::error(407),
                        span,
                        format!(
                            "a sequence pattern of {fixed} items never matches {}",
                            value.ty.with_article()
                        ),
                    )
                    .with_rule(RuleId::CST_06),
                    span,
                );
                self.bind_after_error(whole, &error);
                return TypedPattern::All(steps);
            }
            // Every length is at least zero, so `[*rest]` needs no test.
            None if star.is_some() && fixed == 0 => {}
            None => {
                let operator = if star.is_some() {
                    CompareOperator::GtE
                } else {
                    CompareOperator::Eq
                };
                let test = self.length_test(subject, operator, fixed, span);
                steps.push(TypedPattern::Test(test));
            }
        }
        let after = star.map_or(0, |star| patterns.len() - star - 1);
        for (position, pattern) in patterns.iter().enumerate() {
            let index = match star {
                Some(star) if position == star => {
                    if let Pattern::Star {
                        name: Some(name), ..
                    } = pattern
                    {
                        steps.extend(self.bind_rest(
                            name,
                            subject,
                            &value.ty,
                            (star, after),
                            known_length,
                        ));
                    }
                    continue;
                }
                Some(star) if position > star => {
                    let from_end = patterns.len() - position;
                    known_length.map_or_else(
                        || negative(from_end, span),
                        |length| whole_number(length - from_end, span),
                    )
                }
                _ => whole_number(position, span),
            };
            let item = Expr::Subscript {
                value: Box::new(subject.clone()),
                index: Box::new(index),
                span,
            };
            self.part_pattern(pattern, &item, &mut steps);
        }
        TypedPattern::All(steps)
    }

    /// Adds the test that `subject` is an instance of one of `classes` to `steps`, unless its
    /// type says it always is, and narrows the subject. Returns false, after reporting it,
    /// when no value of the subject's type is.
    pub(super) fn subject_class_test(
        &mut self,
        whole: &Pattern,
        subject: &Expr,
        classes: &[TypeObjectOf],
        what: &str,
        span: Span,
        steps: &mut Vec<TypedPattern>,
    ) -> bool {
        let value = self.expression(subject);
        let ty = value.ty.clone();
        if ty == Type::Error {
            self.bind_after_error(whole, &value);
            return false;
        }
        let test = self.isinstance_test(value, classes, span);
        let always = matches!(&test.kind, TypedExprKind::IsInstance(test) if test.always);
        // A union's members each answer, so the pattern never matches only when none may.
        let never = matches!(&test.kind,
            TypedExprKind::IsInstance(test) if !test.always && test.checks.is_empty()
                && test.members.iter().all(|(always, checks)| !always && checks.is_empty()));
        if never {
            let error = self.error(
                Diagnostic::new(
                    DiagnosticCode::error(407),
                    span,
                    format!("{what} never matches {}", ty.with_article()),
                )
                .with_rule(RuleId::CST_06),
                span,
            );
            self.bind_after_error(whole, &error);
            return false;
        }
        if !always || matches!(ty, Type::Optional(_)) {
            self.add_facts(test_facts(&test).0);
            steps.push(TypedPattern::Test(test));
        }
        true
    }

    /// Returns the test `len(subject) <operator> length`, made with the built-in `len` even
    /// where the program defines a function of that name.
    fn length_test(
        &mut self,
        subject: &Expr,
        operator: CompareOperator,
        length: usize,
        span: Span,
    ) -> TypedExpr {
        let measured = self.call_builtin(Builtin::Len, std::slice::from_ref(subject), &[], span);
        let length = Self::typed(
            TypedExprKind::Constant(TypedConstant::Int(length.to_string())),
            Type::Int,
            span,
        );
        Self::typed(
            TypedExprKind::Compare {
                left: Box::new(measured),
                operators: vec![operator],
                comparators: vec![length],
            },
            Type::Bool,
            span,
        )
    }

    /// Binds the starred name of a sequence pattern to a list of the items between the
    /// fixed ones, as CPython gives a list whatever the subject's sequence type, and returns
    /// the step.
    fn bind_rest(
        &mut self,
        name: &Identifier,
        subject: &Expr,
        ty: &Type,
        (before, after): (usize, usize),
        known_length: Option<usize>,
    ) -> Option<TypedPattern> {
        let span = name.span;
        if let (Type::Tuple(items), Some(length)) = (ty, known_length) {
            let value = self.tuple_rest(subject, items, before..length - after, span);
            let target = Expr::Name {
                id: name.name.clone(),
                span,
            };
            return self
                .assign(&target, value.ty.clone(), &value)
                .map(|symbol| TypedPattern::Bind(symbol, value));
        }
        let upper = match (after, known_length) {
            (0, _) => None,
            (after, Some(length)) => Some(Box::new(whole_number(length - after, span))),
            (after, None) => Some(Box::new(negative(after, span))),
        };
        let slice = Expr::Subscript {
            value: Box::new(subject.clone()),
            index: Box::new(Expr::Slice {
                lower: Some(Box::new(whole_number(before, span))),
                upper,
                step: None,
                span,
            }),
            span,
        };
        let value = if matches!(ty, Type::List(_)) {
            self.expression(&slice)
        } else {
            self.call_builtin(Builtin::List, std::slice::from_ref(&slice), &[], span)
        };
        let target = Expr::Name {
            id: name.name.clone(),
            span,
        };
        self.assign(&target, value.ty.clone(), &value)
            .map(|symbol| TypedPattern::Bind(symbol, value))
    }
}

impl BodyChecker<'_> {
    /// Returns the list `[subject[i] for i in range]` for a tuple of known items, a list of
    /// the union of their types, or of all the tuple's item types when the range is empty.
    fn tuple_rest(
        &mut self,
        subject: &Expr,
        items: &[Type],
        range: std::ops::Range<usize>,
        span: Span,
    ) -> TypedExpr {
        let taken = &items[range.clone()];
        let item = Type::union(if taken.is_empty() { items } else { taken }.to_vec());
        let display = Expr::List {
            elements: range
                .map(|index| Expr::Subscript {
                    value: Box::new(subject.clone()),
                    index: Box::new(whole_number(index, span)),
                    span,
                })
                .collect(),
            span,
        };
        self.expression_expecting(&display, Some(&Type::List(Box::new(item))))
    }
}

/// Returns the literal `number`.
pub(super) fn whole_number(number: usize, span: Span) -> Expr {
    Expr::Constant {
        value: Constant::Int(number.to_string()),
        span,
    }
}

/// Returns the literal `-number`.
fn negative(number: usize, span: Span) -> Expr {
    Expr::UnaryOp {
        operator: UnaryOperator::USub,
        operand: Box::new(whole_number(number, span)),
        span,
    }
}
