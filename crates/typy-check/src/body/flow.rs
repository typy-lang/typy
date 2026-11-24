//! What is known on every path to a point of a body: which variables are assigned (TYP-12),
//! and which optional variables are not `None` there (TYP-13).

use std::collections::BTreeSet;

use crate::typed::{LiteralTest, SymbolId, TypedConstant, TypedExpr, TypedExprKind, TypedPattern};
use crate::types::Type;
use typy_source::syntax::{BoolOperator, CompareOperator, UnaryOperator};

/// One thing known of a variable at a point.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(super) enum Fact {
    /// The variable is assigned on every path.
    Assigned(SymbolId),
    /// The variable, of type `T | None`, is not `None` on every path.
    NotNone(SymbolId),
    /// The attribute at this position of the instance `__init__` builds is assigned on every
    /// path (CLS-01).
    Attribute(usize),
    /// The variable holds a value of the type at this position of the function's table of
    /// narrowed types on every path, as an `isinstance` test showed (TYP-07).
    Narrowed(SymbolId, u32),
}

/// What holds on every path to a point; `None` when the point cannot be reached.
pub(super) type Assigned = Option<BTreeSet<Fact>>;

/// Returns what holds where two paths join: what holds on both.
pub(super) fn meet(left: Assigned, right: Assigned) -> Assigned {
    match (left, right) {
        (None, other) | (other, None) => other,
        (Some(left), Some(right)) => Some(left.intersection(&right).copied().collect()),
    }
}

/// Returns the variable an expression reads, also through what an earlier test showed of it:
/// the value it holds when it is not `None`, or an instance of a derived class (TYP-07). An
/// assignment expression is its variable, which holds its value (EXP-13).
pub(super) fn narrowable_variable(expression: &TypedExpr) -> Option<SymbolId> {
    match &expression.kind {
        TypedExprKind::Local(symbol)
        | TypedExprKind::ModuleVariable(symbol)
        | TypedExprKind::Named { target: symbol, .. } => Some(*symbol),
        TypedExprKind::Unwrap(inner) | TypedExprKind::Downcast(inner) => narrowable_variable(inner),
        _ => None,
    }
}

fn optional_variable(expression: &TypedExpr) -> Option<SymbolId> {
    match &expression.kind {
        TypedExprKind::Local(symbol)
        | TypedExprKind::ModuleVariable(symbol)
        | TypedExprKind::Named { target: symbol, .. }
            if matches!(expression.ty, Type::Optional(_)) =>
        {
            Some(*symbol)
        }
        TypedExprKind::Unwrap(inner) => optional_variable(inner),
        // A read an `isinstance` test narrowed to a `T | None` (TYP-14).
        TypedExprKind::Downcast(inner) if matches!(expression.ty, Type::Optional(_)) => {
            narrowable_variable(inner)
        }
        _ => None,
    }
}

fn is_none(expression: &TypedExpr) -> bool {
    matches!(
        expression.kind,
        TypedExprKind::Constant(TypedConstant::None)
    )
}

/// Returns whether a test may hold and whether it may fail when the program runs, as the
/// types decide it: `True` and `False` decide themselves, an `isinstance` test of a value no
/// tested class takes never holds, and one every value passes never fails (TYP-34,
/// ADR-0111). A value's static type holds for each of its values, so a branch the test rules
/// out never runs. With `literals`, a comparison of a variable of literal types with
/// constants decides too, as the check of an overloaded function's body reads it (TYP-29).
pub(super) fn test_reach(test: &TypedExpr, literals: bool) -> (bool, bool) {
    match &test.kind {
        TypedExprKind::Compare {
            narrowing: Some(narrowing),
            ..
        } if literals => (narrowing.holds.reached, narrowing.fails.reached),
        TypedExprKind::Constant(TypedConstant::Bool(value)) => (*value, !*value),
        TypedExprKind::IsInstance(isinstance) => {
            let optional = matches!(isinstance.value.ty, Type::Optional(_));
            let may_hold = isinstance.always
                || isinstance.none_matches
                || !isinstance.checks.is_empty()
                || !isinstance.held.is_empty();
            let may_fail = !(isinstance.always && (!optional || isinstance.none_matches));
            // A `TypeGuard` call may be false for a value of its type (TYP-31).
            let inexact = isinstance.guard.as_ref().is_some_and(|guard| !guard.exact);
            (may_hold, may_fail || inexact)
        }
        TypedExprKind::Unary {
            operator: UnaryOperator::Not,
            operand,
        } => {
            let (may_hold, may_fail) = test_reach(operand, literals);
            (may_fail, may_hold)
        }
        TypedExprKind::BoolOp { operator, values } => {
            let reach: Vec<(bool, bool)> = values
                .iter()
                .map(|value| test_reach(value, literals))
                .collect();
            match operator {
                BoolOperator::And => (
                    reach.iter().all(|(may_hold, _)| *may_hold),
                    reach.iter().any(|(_, may_fail)| *may_fail),
                ),
                BoolOperator::Or => (
                    reach.iter().any(|(may_hold, _)| *may_hold),
                    reach.iter().all(|(_, may_fail)| *may_fail),
                ),
            }
        }
        _ => (true, true),
    }
}

/// Returns whether a pattern may match, as the types decide its tests (TYP-34).
pub(super) fn pattern_reach(pattern: &TypedPattern, literals: bool) -> bool {
    match pattern {
        TypedPattern::Test(test) => test_reach(test, literals).0,
        TypedPattern::Bind(..) => true,
        TypedPattern::All(steps) => steps.iter().all(|step| pattern_reach(step, literals)),
        TypedPattern::Any(alternatives) => alternatives
            .iter()
            .any(|alternative| pattern_reach(alternative, literals)),
    }
}

/// Returns the facts a comparison of a variable with constants gives when it holds and when
/// it fails: the literal types it narrows the variable to, and that it is not `None`.
fn literal_facts(narrowing: &LiteralTest) -> (BTreeSet<Fact>, BTreeSet<Fact>) {
    let facts = |position: Option<u32>, not_none: bool| {
        let mut facts = BTreeSet::new();
        if let Some(position) = position {
            facts.insert(Fact::Narrowed(narrowing.variable, position));
        }
        if not_none {
            facts.insert(Fact::NotNone(narrowing.variable));
        }
        facts
    };
    (
        facts(narrowing.holds.narrowed, narrowing.holds.not_none),
        facts(narrowing.fails.narrowed, narrowing.fails.not_none),
    )
}

/// Returns the facts a test gives when it holds and when it fails: `x is not None` and a
/// truth test of `x` show `x` is not `None` when they hold, and `x is None` when it fails.
pub(super) fn test_facts(test: &TypedExpr) -> (BTreeSet<Fact>, BTreeSet<Fact>) {
    let none = BTreeSet::new;
    match &test.kind {
        // A comparison with constants narrows the variable to literal types (TYP-28).
        TypedExprKind::Compare {
            narrowing: Some(narrowing),
            ..
        } => literal_facts(narrowing),
        TypedExprKind::Compare {
            left,
            operators,
            comparators,
            ..
        } if operators.len() == 1 => {
            let right = &comparators[0];
            let variable = match (is_none(left), is_none(right)) {
                (false, true) => optional_variable(left),
                (true, false) => optional_variable(right),
                _ => None,
            };
            let Some(variable) = variable else {
                return (none(), none());
            };
            let fact = BTreeSet::from([Fact::NotNone(variable)]);
            match operators[0] {
                CompareOperator::IsNot => (fact, none()),
                CompareOperator::Is => (none(), fact),
                _ => (none(), none()),
            }
        }
        TypedExprKind::IsInstance(isinstance) => {
            let Some(symbol) = narrowable_variable(&isinstance.value) else {
                return (none(), none());
            };
            let mut holds = none();
            if let Some(narrowed) = isinstance.narrowed {
                holds.insert(Fact::Narrowed(symbol, narrowed));
                if matches!(isinstance.value.ty, Type::Optional(_)) {
                    holds.insert(Fact::NotNone(symbol));
                }
            }
            let mut fails: BTreeSet<Fact> = isinstance
                .otherwise
                .map(|otherwise| BTreeSet::from([Fact::Narrowed(symbol, otherwise)]))
                .unwrap_or_default();
            // A test that `None` passes shows an optional is not `None` where it fails; a
            // `TypeGuard` call that fails shows nothing (TYP-31).
            let exact = isinstance.guard.as_ref().is_none_or(|guard| guard.exact);
            if exact && isinstance.none_matches && matches!(isinstance.value.ty, Type::Optional(_))
            {
                fails.insert(Fact::NotNone(symbol));
            }
            (holds, fails)
        }
        TypedExprKind::Unary {
            operator: UnaryOperator::Not,
            operand,
        } => {
            let (holds, fails) = test_facts(operand);
            (fails, holds)
        }
        TypedExprKind::BoolOp { operator, values } => {
            let facts: Vec<_> = values.iter().map(test_facts).collect();
            // `a and b` holds when every operand holds; it fails when one fails after the ones
            // before it held. `or` is the same with holding and failing exchanged.
            let (decides, passes): (Vec<_>, Vec<_>) = match operator {
                BoolOperator::And => facts
                    .into_iter()
                    .map(|(holds, fails)| (fails, holds))
                    .unzip(),
                BoolOperator::Or => facts.into_iter().unzip(),
            };
            let all: BTreeSet<Fact> = passes.iter().flatten().copied().collect();
            let mut before = BTreeSet::new();
            let mut any: Option<BTreeSet<Fact>> = None;
            for (decide, pass) in decides.into_iter().zip(passes) {
                let here: BTreeSet<Fact> = before.union(&decide).copied().collect();
                any = Some(match any {
                    None => here,
                    Some(any) => any.intersection(&here).copied().collect(),
                });
                before.extend(pass);
            }
            let any = any.unwrap_or_default();
            match operator {
                BoolOperator::And => (all, any),
                BoolOperator::Or => (any, all),
            }
        }
        _ => match optional_variable(test) {
            Some(variable) => (BTreeSet::from([Fact::NotNone(variable)]), none()),
            None => (none(), none()),
        },
    }
}
