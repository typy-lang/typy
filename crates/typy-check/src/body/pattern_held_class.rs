//! A class pattern whose class is held in a value, `case kind(size=1):` (CLS-18, ADR-0084):
//! the subject is tested as `isinstance(subject, kind)` is, when the program runs, and narrows
//! to the class the value's type names, whose attributes keyword sub-patterns match.
//! Positional sub-patterns read the `__match_args__` of the class the value holds, which only
//! the run knows, and are planned for 0.14 (CLS-40).

use typy_diagnostics::{Diagnostic, DiagnosticCode, RuleId, Span};
use typy_source::syntax::{Expr, Identifier, Pattern};

use super::BodyChecker;
use super::flow::test_facts;
use crate::typed::{TypedExpr, TypedPattern};
use crate::types::Type;

impl BodyChecker<'_> {
    /// Returns the steps of a class pattern whose class `held` is held in a value: the test,
    /// then each keyword sub-pattern against the attribute it names.
    pub(super) fn held_class_pattern(
        &mut self,
        whole: &Pattern,
        held: TypedExpr,
        (patterns, keywords): (&[Pattern], &[(Identifier, Pattern)]),
        subject: &Expr,
        span: Span,
    ) -> TypedPattern {
        let mut steps = Vec::new();
        let refusal = if matches!(held.ty, Type::Tuple(_) | Type::TupleOf(_)) {
            Some(
                Diagnostic::new(
                    DiagnosticCode::error(401),
                    held.span,
                    "a class pattern names one class, and this is a tuple of classes: CPython \
                     raises `TypeError`",
                )
                .with_rule(RuleId::CST_06),
            )
        } else {
            patterns.first().map(|first| {
                Diagnostic::new(
                    DiagnosticCode::error(201),
                    first.span(),
                    "a positional sub-pattern of a class pattern whose class is held in a value \
                     is planned for TyPy 0.14 (CLS-40)",
                )
                .with_rule(RuleId::CLS_40)
                .with_help("name the attribute, as in `case kind(size=1):`")
            })
        };
        if let Some(diagnostic) = refusal {
            let error = self.error(diagnostic, span);
            self.bind_after_error(whole, &error);
            return TypedPattern::All(steps);
        }
        let value = self.expression(subject);
        let test = self.isinstance_test(value, &[], span);
        let test = self.with_held_classes(test, vec![held]);
        self.add_facts(test_facts(&test).0);
        steps.push(TypedPattern::Test(test));
        for (attribute, pattern) in keywords {
            let value = Expr::Attribute {
                value: Box::new(subject.clone()),
                attribute: attribute.clone(),
                span,
            };
            self.part_pattern(pattern, &value, &mut steps);
        }
        TypedPattern::All(steps)
    }
}
