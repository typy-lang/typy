//! What is known on every path to a point of a body: which variables are assigned (TYP-12),
//! and which optional variables are not `None` there (TYP-13).

use std::collections::BTreeSet;

use crate::typed::{SymbolId, TypedConstant, TypedExpr, TypedExprKind};
use crate::types::Type;
use typy_source::syntax::{BoolOperator, CompareOperator, UnaryOperator};

/// One thing known of a variable at a point.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(super) enum Fact {
    /// The variable is assigned on every path.
    Assigned(SymbolId),
    /// The variable, of type `T | None`, is not `None` on every path.
    NotNone(SymbolId),
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

/// Returns the optional variable an expression reads, if it reads one.
fn optional_variable(expression: &TypedExpr) -> Option<SymbolId> {
    match &expression.kind {
        TypedExprKind::Local(symbol) if matches!(expression.ty, Type::Optional(_)) => Some(*symbol),
        TypedExprKind::Unwrap(inner) => optional_variable(inner),
        _ => None,
    }
}

fn is_none(expression: &TypedExpr) -> bool {
    matches!(
        expression.kind,
        TypedExprKind::Constant(TypedConstant::None)
    )
}

/// Returns the facts a test gives when it holds and when it fails: `x is not None` and a
/// truth test of `x` show `x` is not `None` when they hold, and `x is None` when it fails.
pub(super) fn test_facts(test: &TypedExpr) -> (BTreeSet<Fact>, BTreeSet<Fact>) {
    let none = BTreeSet::new;
    match &test.kind {
        TypedExprKind::Compare {
            left,
            operators,
            comparators,
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
