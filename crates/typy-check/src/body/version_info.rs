//! `sys.version_info` where a program reads it (LIB-13, ADR-0029): a comparison with a tuple of
//! `int`s, an item at a constant index and its named fields give constants, those of CPython
//! 3.15.0, whose behaviour TyPy follows. Any other use of it as a value is planned (LIB-16).

use std::cmp::Ordering;

use typy_source::syntax::{CompareOperator, Constant, Expr, UnaryOperator};

use super::BodyChecker;
use crate::typed::{TypedConstant, TypedExpr, TypedExprKind};
use crate::types::Type;

/// The version TyPy reports, `(3, 15, 0, 'final', 0)`.
const MAJOR: i64 = 3;
const MINOR: i64 = 15;
const MICRO: i64 = 0;

/// An item of `sys.version_info`.
#[derive(Clone, Copy)]
enum Item {
    Int(i64),
    Str(&'static str),
}

const ITEMS: [Item; 5] = [
    Item::Int(MAJOR),
    Item::Int(MINOR),
    Item::Int(MICRO),
    Item::Str("final"),
    Item::Int(0),
];

const FIELDS: [&str; 5] = ["major", "minor", "micro", "releaselevel", "serial"];

impl BodyChecker<'_> {
    /// Returns the constant a use of `sys.version_info` gives, when the expression is one TyPy
    /// folds.
    pub(super) fn version_info_use(&mut self, expression: &Expr) -> Option<TypedExpr> {
        let span = expression.span();
        let item = match expression {
            Expr::Attribute {
                value, attribute, ..
            } if self.is_version_info(value) => {
                let index = FIELDS.iter().position(|field| *field == attribute.name)?;
                ITEMS[index]
            }
            Expr::Subscript { value, index, .. } if self.is_version_info(value) => {
                let position = constant_int(index)?;
                let position = if position < 0 { position + 5 } else { position };
                ITEMS[usize::try_from(position)
                    .ok()
                    .filter(|position| *position < 5)?]
            }
            Expr::Compare {
                left,
                operators,
                comparators,
                ..
            } => {
                let ([operator], [right]) = (operators.as_slice(), comparators.as_slice()) else {
                    return None;
                };
                let truth = if self.is_version_info(left) {
                    compare(*operator, &int_tuple(right)?, false)?
                } else if self.is_version_info(right) {
                    compare(*operator, &int_tuple(left)?, true)?
                } else {
                    return None;
                };
                return Some(Self::typed(
                    TypedExprKind::Constant(TypedConstant::Bool(truth)),
                    Type::Bool,
                    span,
                ));
            }
            _ => return None,
        };
        let (constant, ty) = match item {
            Item::Int(value) => (TypedConstant::Int(value.to_string()), Type::Int),
            Item::Str(text) => (TypedConstant::Str(text.to_owned()), Type::Str),
        };
        Some(Self::typed(TypedExprKind::Constant(constant), ty, span))
    }

    /// Returns true for `sys.version_info`, read through a name of the `sys` module.
    fn is_version_info(&self, expression: &Expr) -> bool {
        matches!(expression, Expr::Attribute { value, attribute, .. }
            if attribute.name == "version_info"
                && self.module_of(value).is_some_and(|module| module == "sys"))
    }
}

/// Returns the value of an `int` constant, negated or not, that fits an `i64`.
fn constant_int(expression: &Expr) -> Option<i64> {
    match expression {
        Expr::Constant {
            value: Constant::Int(digits),
            ..
        } => digits.parse().ok(),
        Expr::UnaryOp {
            operator: UnaryOperator::USub,
            operand,
            ..
        } => constant_int(operand).and_then(i64::checked_neg),
        _ => None,
    }
}

/// Returns the items of a tuple display of `int` constants.
fn int_tuple(expression: &Expr) -> Option<Vec<i64>> {
    let Expr::Tuple { elements, .. } = expression else {
        return None;
    };
    elements.iter().map(constant_int).collect()
}

/// Returns `version_info <operator> other`, or `other <operator> version_info` when
/// `reversed`, as CPython compares tuples; `None` where CPython would compare `'final'` with
/// an `int`, which raises `TypeError`.
fn compare(operator: CompareOperator, other: &[i64], reversed: bool) -> Option<bool> {
    let mut order = None;
    let mut equal = true;
    for (item, value) in ITEMS.iter().zip(other) {
        match item {
            Item::Int(own) if own == value => {}
            Item::Int(own) => {
                order = Some(own.cmp(value));
                equal = false;
                break;
            }
            Item::Str(_) => {
                equal = false;
                order = None;
                // Equality is false here; an order would raise.
                if !matches!(operator, CompareOperator::Eq | CompareOperator::NotEq) {
                    return None;
                }
                break;
            }
        }
    }
    let order = match order {
        Some(order) => order,
        None if equal => ITEMS.len().cmp(&other.len()),
        None => Ordering::Equal,
    };
    let order = if reversed { order.reverse() } else { order };
    let same = equal && ITEMS.len() == other.len();
    Some(match operator {
        CompareOperator::Eq => same,
        CompareOperator::NotEq => !same,
        CompareOperator::Lt => order == Ordering::Less,
        CompareOperator::LtE => order != Ordering::Greater,
        CompareOperator::Gt => order == Ordering::Greater,
        CompareOperator::GtE => order != Ordering::Less,
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::{CompareOperator, compare};

    #[test]
    fn version_info_compares_as_cpython_compares_tuples() {
        use CompareOperator::{Eq, Gt, GtE, Lt, LtE, NotEq};
        let cases: &[(CompareOperator, &[i64], bool, Option<bool>)] = &[
            (GtE, &[3, 12], false, Some(true)),
            (GtE, &[3, 15], false, Some(true)),
            (Gt, &[3, 15], false, Some(true)),
            (Lt, &[3, 16], false, Some(true)),
            (LtE, &[3, 15, 0], false, Some(false)),
            (Eq, &[3, 15, 0], false, Some(false)),
            (NotEq, &[3, 15], false, Some(true)),
            (Lt, &[3, 12], true, Some(true)),
            (GtE, &[4], true, Some(true)),
            (GtE, &[], false, Some(true)),
            (GtE, &[3, 15, 0, 0], false, None),
            (Eq, &[3, 15, 0, 0], false, Some(false)),
            (Gt, &[3, 14, 0, 0], false, Some(true)),
        ];
        for (operator, other, reversed, expected) in cases {
            assert_eq!(
                compare(*operator, other, *reversed),
                *expected,
                "{operator:?} {other:?} {reversed}"
            );
        }
    }
}
