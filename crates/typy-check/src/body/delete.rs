//! Checks `del`: of list items, list slices and dict keys (SMT-05).

use typy_diagnostics::{Diagnostic, DiagnosticCode, RuleId, Span};
use typy_source::syntax::Expr;

use super::BodyChecker;
use crate::typed::{TypedExpr, TypedExprKind, TypedStmt};
use crate::types::Type;

impl BodyChecker<'_> {
    /// Checks `del target, ...`; a tuple or list of targets deletes each in order.
    pub(super) fn delete_statement(&mut self, targets: &[Expr], span: Span) -> TypedStmt {
        let mut checked = Vec::new();
        for target in targets {
            self.delete_target(target, &mut checked);
        }
        TypedStmt::Delete {
            targets: checked,
            span,
        }
    }

    /// Checks one target: a list item, a list slice or a dict key. A name or an attribute is
    /// refused, since TyPy keeps every variable and attribute bound once assigned.
    fn delete_target(&mut self, target: &Expr, checked: &mut Vec<TypedExpr>) {
        if let Expr::Tuple { elements, .. } | Expr::List { elements, .. } = target {
            for element in elements {
                self.delete_target(element, checked);
            }
            return;
        }
        let refused = |what: &str| {
            Diagnostic::new(
                DiagnosticCode::error(421),
                target.span(),
                format!(
                    "`del` removes a list item, a list slice or a dict key, and this is {what}"
                ),
            )
            .with_rule(RuleId::SMT_05)
        };
        match target {
            Expr::Name { id, .. } => {
                self.diagnostics
                    .push(refused(&format!("the variable `{id}`")));
                return;
            }
            Expr::Subscript { .. } => {}
            // Python's grammar leaves an attribute as the only other target of `del`.
            _ => {
                self.diagnostics.push(refused("an attribute"));
                return;
            }
        }
        let value = self.expression(target);
        let removable = match &value.kind {
            TypedExprKind::ListItem { list, .. } => matches!(list.ty, Type::List(_)),
            TypedExprKind::Slice {
                value: sequence, ..
            } => matches!(sequence.ty, Type::List(_)),
            TypedExprKind::DictItem { .. } => true,
            _ => value.ty.has_error(),
        };
        if removable {
            checked.push(value);
            return;
        }
        let container = match &value.kind {
            TypedExprKind::ListItem { list: sequence, .. }
            | TypedExprKind::Slice {
                value: sequence, ..
            }
            | TypedExprKind::Index {
                value: sequence, ..
            } => format!("part of {}", sequence.ty.with_article()),
            TypedExprKind::TupleItem { tuple, .. } | TypedExprKind::TupleSlice { tuple, .. } => {
                format!("part of {}", tuple.ty.with_article())
            }
            _ => "not a target".to_owned(),
        };
        self.diagnostics.push(refused(&container));
    }
}
