//! Lowers the set operators and comparisons of `dict.keys()` and `dict.items()`, and the
//! identity comparison of `dict.values()` (DAT-16). The runtime reads each operand by its type
//! ID, so a view, a set and a list pass as their addresses.

use typy_check::{Type, View};
use typy_diagnostics::Span;
use typy_source::syntax::{BinaryOperator, CompareOperator};
use typy_tyir::{Comparison, Constant, Operation, RuntimeFunction, TyirType, ValueId};

use crate::function::FunctionLowerer;
use crate::ownership::Lowered;

/// Returns the item type of a set or a view of keys or items, as the checker types it.
fn item_of(ty: &Type) -> Option<Type> {
    match ty {
        Type::Set(item) => Some(item.as_ref().clone()),
        Type::DictView(View::Keys, key, _) => Some(key.as_ref().clone()),
        Type::DictView(View::Items, key, value) => Some(Type::Tuple(vec![
            key.as_ref().clone(),
            value.as_ref().clone(),
        ])),
        _ => None,
    }
}

impl FunctionLowerer<'_> {
    /// Returns `left op right` for `|`, `&`, `-` or `^` with a view on a side: a new set of the
    /// result's item type, which the runtime fills by CPython's steps. The operands are not
    /// released.
    pub(crate) fn view_combine(
        &mut self,
        operator: BinaryOperator,
        left: ValueId,
        right: ValueId,
        result: &Type,
        span: Span,
    ) -> Lowered {
        let Type::Set(item) = result else {
            unreachable!("the checker types a view operator as a set")
        };
        let item = self.lowered_type(item);
        let set = self.new_set(item, span);
        let code = match operator {
            BinaryOperator::BitOr => 0,
            BinaryOperator::BitAnd => 1,
            BinaryOperator::Sub => 2,
            _ => 3,
        };
        let code = self.constant(Constant::Int(code.to_string()), span);
        let left = self.address(left, span);
        let right = self.address(right, span);
        self.call_runtime(
            RuntimeFunction::DictViewCombine,
            vec![left, right, code, set.value],
            span,
        );
        set
    }

    /// Returns `left op right` for a comparison with a view on a side, or `None` for any other
    /// comparison. Views of values compare by identity; any other view compares by its items
    /// with a set or another view. The operands are not released.
    pub(crate) fn compare_views(
        &mut self,
        operator: CompareOperator,
        (left, left_type): (Lowered, &Type),
        (right, right_type): (Lowered, &Type),
        span: Span,
    ) -> Option<ValueId> {
        if !matches!(left_type, Type::DictView(..)) && !matches!(right_type, Type::DictView(..)) {
            return None;
        }
        let comparison = match operator {
            CompareOperator::Eq => Comparison::Equal,
            CompareOperator::NotEq => Comparison::NotEqual,
            CompareOperator::Lt => Comparison::Less,
            CompareOperator::LtE => Comparison::LessEqual,
            CompareOperator::Gt => Comparison::Greater,
            CompareOperator::GtE => Comparison::GreaterEqual,
            _ => return None,
        };
        let Some(item) = item_of(left_type).or_else(|| item_of(right_type)) else {
            let same = self.identical(left, right, span);
            if comparison == Comparison::Equal {
                return Some(same);
            }
            return Some(
                self.builder
                    .push_value(Operation::BoolNot(same), TyirType::Bool, span),
            );
        };
        let item = self.lowered_type(&item);
        let (kind, descriptor) = self.item_layout(item, span);
        let code = self.constant(Constant::Int(comparison.code().to_string()), span);
        let left = self.address(left.value, span);
        let right = self.address(right.value, span);
        let holds = self.call_runtime(
            RuntimeFunction::DictViewCompare,
            vec![left, right, code, kind, descriptor],
            span,
        );
        Some(holds.value)
    }
}
