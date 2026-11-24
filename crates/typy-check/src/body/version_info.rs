//! `sys.version_info` (LIB-13, LIB-16, ADR-0029): a comparison of `sys.version_info` with a
//! tuple of `int`s, an item at a constant index and its named fields give constants, those of
//! CPython 3.15.0, whose behaviour TyPy follows. Held as a value, it is one object, which a
//! tuple's operations read as the tuple of its items.

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
    /// Returns `value` as the tuple of its items when it is `sys.version_info`, which an index,
    /// a slice, a comparison, `len`, a loop and unpacking read; any other value as it is
    /// (LIB-16).
    pub(super) fn version_tuple(value: TypedExpr) -> TypedExpr {
        if value.ty != Type::VersionInfo {
            return value;
        }
        let span = value.span;
        let items: Vec<TypedExpr> = ITEMS
            .iter()
            .map(|item| {
                let (constant, ty) = match item {
                    Item::Int(number) => (TypedConstant::Int(number.to_string()), Type::Int),
                    Item::Str(text) => (TypedConstant::Str((*text).to_owned()), Type::Str),
                };
                Self::typed(TypedExprKind::Constant(constant), ty, span)
            })
            .collect();
        let ty = Type::Tuple(items.iter().map(|item| item.ty.clone()).collect());
        let tuple = Self::typed(TypedExprKind::Tuple(items), ty, span);
        Self::version_read(value, tuple)
    }

    /// Returns the read of `sys.version_info` held in `value` that gives `result`.
    fn version_read(value: TypedExpr, result: TypedExpr) -> TypedExpr {
        let (ty, span) = (result.ty.clone(), value.span);
        Self::typed(
            TypedExprKind::VersionRead {
                value: Box::new(value),
                result: Box::new(result),
            },
            ty,
            span,
        )
    }

    /// Returns `left <operator> right` made when compiling, when one operand is
    /// `sys.version_info` held as a value and the other a tuple of `int` literals; the value
    /// is still evaluated (LIB-16).
    pub(super) fn version_comparison(
        operator: CompareOperator,
        left: &TypedExpr,
        right: &TypedExpr,
        span: typy_diagnostics::Span,
    ) -> Option<TypedExpr> {
        let held = |operand: &TypedExpr| match &operand.kind {
            TypedExprKind::VersionRead { value, .. } => Some(value.as_ref().clone()),
            _ => None,
        };
        let ints = |operand: &TypedExpr| match &operand.kind {
            TypedExprKind::Tuple(items) => items
                .iter()
                .map(|item| match &item.kind {
                    TypedExprKind::Constant(TypedConstant::Int(digits)) => digits.parse().ok(),
                    _ => None,
                })
                .collect::<Option<Vec<i64>>>(),
            _ => None,
        };
        let (value, truth) = if let (Some(value), Some(other)) = (held(left), ints(right)) {
            (value, compare(operator, &other, false)?)
        } else if let (Some(other), Some(value)) = (ints(left), held(right)) {
            (value, compare(operator, &other, true)?)
        } else {
            return None;
        };
        let answer = Self::typed(
            TypedExprKind::Constant(TypedConstant::Bool(truth)),
            Type::Bool,
            span,
        );
        Some(Self::version_read(value, answer))
    }

    /// Checks `value.attribute` of `sys.version_info` held as a value: one of its named
    /// fields.
    pub(super) fn version_info_attribute(
        &mut self,
        value: TypedExpr,
        attribute: &typy_source::syntax::Identifier,
        span: typy_diagnostics::Span,
    ) -> TypedExpr {
        let name = attribute.name.clone();
        if let Some(field) = Self::version_info_field(value, &name, span) {
            return field;
        }
        self.error(
            typy_diagnostics::Diagnostic::new(
                typy_diagnostics::DiagnosticCode::error(412),
                attribute.span,
                format!(
                    "`sys.version_info` has no attribute `{name}`; it has `major`, `minor`, \
                     `micro`, `releaselevel` and `serial`"
                ),
            )
            .with_rule(typy_diagnostics::RuleId::LIB_16),
            span,
        )
    }

    /// Returns `value.attribute` of `sys.version_info` held as a value: one of its named
    /// fields, or `None` for another attribute.
    fn version_info_field(
        value: TypedExpr,
        attribute: &str,
        span: typy_diagnostics::Span,
    ) -> Option<TypedExpr> {
        let index = FIELDS.iter().position(|field| *field == attribute)?;
        let tuple = Self::version_tuple(value);
        let ty = match ITEMS[index] {
            Item::Int(_) => Type::Int,
            Item::Str(_) => Type::Str,
        };
        Some(Self::typed(
            TypedExprKind::TupleItem {
                tuple: Box::new(tuple),
                index,
            },
            ty,
            span,
        ))
    }

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
mod tests;
