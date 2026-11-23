//! Values written where a literal type is declared (TYP-28, ADR-0110). An expression's type
//! is never a literal type, so what a value is known to be comes from the value itself: a
//! constant, or a variable, an attribute or a function's result declared, or narrowed, to a
//! literal type.

use typy_diagnostics::{Diagnostic, DiagnosticCode, RuleId};
use typy_source::syntax::{CompareOperator, UnaryOperator};

use super::BodyChecker;
use super::flow::narrowable_variable;
use crate::literal_type::LiteralValue;
use crate::typed::{
    LiteralOutcome, LiteralTest, SymbolId, TypedConstant, TypedExpr, TypedExprKind,
};
use crate::types::Type;

impl BodyChecker<'_> {
    /// Returns `value`, or the error type with E0431 when it is not known to be one of the
    /// values the literal types of `expected` take.
    pub(super) fn checked_literal(&mut self, value: TypedExpr, expected: &Type) -> TypedExpr {
        if value.ty.has_error() {
            return value;
        }
        let found = self.literal_of(&value).unwrap_or_else(|| value.ty.clone());
        if found.takes_literals_of(expected) {
            return value;
        }
        let span = value.span;
        let found = match &found {
            Type::Literal(value) => format!("`{value}`"),
            other => other.with_article(),
        };
        self.error(
            Diagnostic::new(
                DiagnosticCode::error(431),
                span,
                format!("this is {found}, and {expected} is wanted here"),
            )
            .with_rule(RuleId::TYP_28)
            .with_help(
                "write one of the constants it names, or test the value first, as \
                 `if mode == \"r\":` narrows `mode` to `Literal['r']`",
            ),
            span,
        )
    }

    /// Returns the literal type a value is known to have: a constant's own, or the declared
    /// or narrowed literal type of the variable, the attribute or the function's result it
    /// reads; `None` when none is known.
    pub(super) fn literal_of(&self, value: &TypedExpr) -> Option<Type> {
        let found = match &value.kind {
            TypedExprKind::Constant(constant) => {
                Type::literal(LiteralValue::of_constant(constant)?)
            }
            TypedExprKind::Unary {
                operator: UnaryOperator::USub,
                operand,
            } => match &operand.kind {
                TypedExprKind::Constant(constant @ TypedConstant::Int(_)) => {
                    Type::literal(LiteralValue::of_constant(constant)?.negated()?)
                }
                _ => return None,
            },
            TypedExprKind::Local(symbol) | TypedExprKind::Named { target: symbol, .. } => {
                self.variable_literal(*symbol)?
            }
            TypedExprKind::ModuleVariable(symbol) => self
                .variable_literal(*symbol)
                .unwrap_or_else(|| self.program.symbols[symbol.0 as usize].ty.clone()),
            // The value an optional holds, once a test showed it is not `None`.
            TypedExprKind::Unwrap(inner) => match self.literal_of(inner)? {
                Type::Optional(held) => *held,
                other => other,
            },
            TypedExprKind::Call { function, .. } => {
                self.program.signatures[function.0 as usize].returns.clone()
            }
            TypedExprKind::Attribute { object, index } => {
                self.program.attribute_ty(object.ty.class_id()?, *index)?
            }
            TypedExprKind::IfExp { body, orelse, .. } => Type::union(vec![
                self.literal_of(body).unwrap_or_else(|| body.ty.clone()),
                self.literal_of(orelse).unwrap_or_else(|| orelse.ty.clone()),
            ]),
            _ => return None,
        };
        found.has_literal().then_some(found)
    }

    /// Returns the literal type a variable is known to have where it is read: the one a test
    /// narrowed it to, or the one it is declared with.
    fn variable_literal(&self, symbol: SymbolId) -> Option<Type> {
        let name = self.program.symbols[symbol.0 as usize].name.clone();
        if let Some(narrowed) = self.narrowed_type(symbol, &name)
            && narrowed.has_literal()
        {
            return Some(narrowed);
        }
        self.declared
            .get(&symbol)
            .filter(|declared| declared.has_literal())
            .cloned()
    }
}

impl BodyChecker<'_> {
    /// Returns what `left operator right` shows of a variable compared with constants: `==`
    /// and `!=` with one constant, `in` and `not in` with a display of them. A variable of
    /// the constants' base type narrows to the literal types of the constants it may equal
    /// when the test holds, and one declared of literal types to the others when it fails. A
    /// narrowing never changes how the value is held, beyond the value of an optional once
    /// it is known not to be `None`.
    pub(super) fn literal_test(
        &mut self,
        operator: CompareOperator,
        left: &TypedExpr,
        right: &TypedExpr,
    ) -> Option<LiteralTest> {
        let (variable, constants, equal) = match operator {
            CompareOperator::Eq | CompareOperator::NotEq => {
                let equal = operator == CompareOperator::Eq;
                match (self.constant_values(left), self.constant_values(right)) {
                    (None, Some(constants)) => (left, constants, equal),
                    (Some(constants), None) => (right, constants, equal),
                    _ => return None,
                }
            }
            CompareOperator::In | CompareOperator::NotIn => {
                let (TypedExprKind::Tuple(items)
                | TypedExprKind::List(items)
                | TypedExprKind::Set(items)) = &right.kind
                else {
                    return None;
                };
                let mut constants = Vec::with_capacity(items.len());
                for item in items {
                    constants.extend(self.constant_values(item)?);
                }
                (left, constants, operator == CompareOperator::In)
            }
            _ => return None,
        };
        let symbol = narrowable_variable(variable)?;
        let name = self.program.symbols[symbol.0 as usize].name.clone();
        if self.changed_by_calls(&name) {
            return None;
        }
        let (base, optional) = match &variable.ty {
            Type::Optional(inner) => (inner.as_ref(), true),
            other => (other, false),
        };
        let known = self
            .literal_of(variable)
            .and_then(|known| known.literal_values());
        // A variable of literal types narrows to its own values, held alike; any other to the
        // constants, which must be of its own type.
        if constants.is_empty()
            || (known.is_none() && constants.iter().any(|constant| constant.base() != *base))
        {
            return None;
        }
        let (equal_values, other_values) = match &known {
            Some((values, _)) => values
                .iter()
                .cloned()
                .partition(|value| constants.iter().any(|constant| value.equals(constant))),
            None => (constants, Vec::new()),
        };
        let of = |values: Vec<LiteralValue>| {
            (!values.is_empty())
                .then(|| Type::union(values.into_iter().map(Type::literal).collect()))
        };
        // With the values known, a test that no value, or every value, passes is decided.
        let some_equal = !equal_values.is_empty();
        let some_other = known.is_none() || !other_values.is_empty() || optional;
        let equal_type = of(equal_values);
        // The values other than the constants are known only for a variable of literal types.
        let other_type = of(other_values).map(|ty| if optional { Type::optional(ty) } else { ty });
        let equal_position = equal_type.map(|ty| self.narrowing_index(ty));
        let other_position = other_type.map(|ty| self.narrowing_index(ty));
        // Equal to a constant, an optional is not `None`.
        let when_equal = LiteralOutcome {
            narrowed: equal_position,
            not_none: optional,
            reached: some_equal,
        };
        let when_other = LiteralOutcome {
            narrowed: other_position,
            not_none: false,
            reached: some_other,
        };
        let (holds, fails) = if equal {
            (when_equal, when_other)
        } else {
            (when_other, when_equal)
        };
        Some(LiteralTest {
            variable: symbol,
            holds,
            fails,
        })
    }

    /// Returns the literal values a constant, or a `Literal` constant read, is known to be,
    /// or `None` for any other value.
    fn constant_values(&self, value: &TypedExpr) -> Option<Vec<LiteralValue>> {
        if !matches!(
            value.kind,
            TypedExprKind::Constant(_) | TypedExprKind::Unary { .. }
        ) {
            return None;
        }
        match self.literal_of(value)? {
            Type::Literal(value) => Some(vec![*value]),
            _ => None,
        }
    }
}
