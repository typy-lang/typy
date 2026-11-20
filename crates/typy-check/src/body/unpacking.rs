//! Checks unpacking targets that take the values of an iterable (SMT-02, EXP-17): `a, b =
//! items` and `first, *rest = items`, whose number of values the program learns as it runs,
//! and a starred target over a tuple of a fixed number of items.

use typy_diagnostics::{Diagnostic, DiagnosticCode, RuleId};
use typy_source::syntax::Expr;

use super::BodyChecker;
use crate::typed::{AssignTarget, TypedExpr};
use crate::types::Type;

/// What checking an unpacking target gives.
pub(super) enum Unpacking {
    /// The target, or `None` when the checks refused it.
    Checked(Option<AssignTarget>),
    /// Nothing yet: the target unpacks a tuple item by item, or the value cannot be unpacked.
    ItemByItem,
}

impl BodyChecker<'_> {
    /// Returns the target of `elements`, with a starred one at `starred`, for a value of type
    /// `ty`, when it unpacks an iterable or a starred target takes some items of a tuple, or
    /// `ItemByItem` for a tuple of as many items as targets, which the caller unpacks, and
    /// for a value that cannot be unpacked, which it reports.
    pub(super) fn unpacking_target(
        &mut self,
        elements: &[Expr],
        starred: Option<usize>,
        ty: &Type,
        value: &TypedExpr,
    ) -> Unpacking {
        let (before, rest, after) = match starred {
            Some(index) => (
                &elements[..index],
                Some(&elements[index]),
                &elements[index + 1..],
            ),
            None => (elements, None, &elements[..0]),
        };
        let rest = rest.map(|starred| match starred {
            Expr::Starred { value, .. } => value.as_ref(),
            other => other,
        });
        let fixed = before.len() + after.len();
        let (before_types, middle, after_types) = match ty {
            // A tuple of as many items unpacks item by item, and one of another number is the
            // caller's to report.
            Type::Tuple(_) if rest.is_none() => return Unpacking::ItemByItem,
            Type::Tuple(items) if items.len() >= fixed => {
                let middle = &items[before.len()..items.len() - after.len()];
                let Some(middle) = self.middle_type(items, middle, value) else {
                    return Unpacking::Checked(None);
                };
                (
                    items[..before.len()].to_vec(),
                    middle,
                    items[items.len() - after.len()..].to_vec(),
                )
            }
            Type::Tuple(items) => {
                self.too_few_items(items.len(), fixed, value);
                return Unpacking::Checked(None);
            }
            Type::Error => (
                vec![Type::Error; before.len()],
                Type::Error,
                vec![Type::Error; after.len()],
            ),
            other => {
                let Some(item) = iteration_item(other) else {
                    return Unpacking::ItemByItem;
                };
                (
                    vec![item.clone(); before.len()],
                    item.clone(),
                    vec![item; after.len()],
                )
            }
        };
        let before: Option<Vec<AssignTarget>> = before
            .iter()
            .zip(before_types)
            .map(|(target, item)| self.assign_target(target, item, value))
            .collect();
        let rest = rest.map(|target| {
            self.assign_target(target, Type::List(Box::new(middle.clone())), value)
                .map(Box::new)
        });
        let after: Option<Vec<AssignTarget>> = after
            .iter()
            .zip(after_types)
            .map(|(target, item)| self.assign_target(target, item, value))
            .collect();
        let (Some(before), Some(after)) = (before, after) else {
            return Unpacking::Checked(None);
        };
        let rest = match rest {
            Some(Some(rest)) => Some(rest),
            Some(None) => return Unpacking::Checked(None),
            None => None,
        };
        Unpacking::Checked(Some(AssignTarget::Unpack {
            before,
            rest,
            after,
            item: middle,
        }))
    }

    /// Returns the type of the items a starred target takes of a tuple of a fixed number of
    /// items: theirs when they share one, or that of every item of the tuple when it takes
    /// none; reports items of several types.
    fn middle_type(&mut self, items: &[Type], middle: &[Type], value: &TypedExpr) -> Option<Type> {
        let shared = |types: &[Type]| {
            let first = types.first()?;
            types
                .iter()
                .all(|each| each == first)
                .then(|| first.clone())
        };
        if let Some(shared) =
            shared(middle).or_else(|| middle.is_empty().then(|| shared(items)).flatten())
        {
            return Some(shared);
        }
        self.diagnostics.push(
            Diagnostic::new(
                DiagnosticCode::error(308),
                value.span,
                if middle.is_empty() {
                    "the starred target takes no items of this tuple, whose items differ, so its \
                     list has no item type"
                        .to_owned()
                } else {
                    format!(
                        "the starred target takes items of types {}, which a list of one type \
                         cannot hold",
                        listed(middle)
                    )
                },
            )
            .with_rule(RuleId::EXP_17),
        );
        None
    }

    /// Reports a tuple of fewer items than the targets outside the starred one.
    fn too_few_items(&mut self, items: usize, fixed: usize, value: &TypedExpr) {
        self.diagnostics.push(
            Diagnostic::new(
                DiagnosticCode::error(308),
                value.span,
                format!("this unpacks {items} values into at least {fixed} targets"),
            )
            .with_rule(RuleId::SMT_02),
        );
    }
}

/// Returns the type of the values an iterable of type `ty` gives, for the types an unpacking
/// target takes the values of as the program runs.
fn iteration_item(ty: &Type) -> Option<Type> {
    Some(match ty {
        Type::List(item)
        | Type::TupleOf(item)
        | Type::Set(item)
        | Type::Dict(item, _)
        | Type::Iterator(item)
        | Type::Iterable(item) => item.as_ref().clone(),
        Type::Generator { yields, .. } => yields.as_ref().clone(),
        Type::DictView(view, key, value) => view.item(key, value),
        Type::Str => Type::Str,
        Type::Bytes | Type::Range => Type::Int,
        // An instance of an iterator or iterable class gives its class's items (TYP-20).
        Type::Instance(_) => return ty.iterable_item(),
        _ => return None,
    })
}

/// Returns the types as a list in words: `int and str`, or `int, str and float`.
fn listed(types: &[Type]) -> String {
    let names: Vec<String> = types.iter().map(ToString::to_string).collect();
    match names.split_last() {
        Some((last, rest)) if !rest.is_empty() => format!("{} and {last}", rest.join(", ")),
        _ => names.join(""),
    }
}
