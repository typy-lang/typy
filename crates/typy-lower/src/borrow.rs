//! Borrowed parameters and moved returns (MEM-04). A parameter the function never assigns
//! keeps the reference its caller holds, so passing an argument counts nothing; the caller
//! holds a reference of its own for the call to every argument that is not a variable or a
//! constant, because the callee could otherwise free it by changing the container it was read
//! from. A `return` of a variable hands its slot's reference to the caller.

use std::collections::BTreeSet;

use typy_check::{AssignTarget, SymbolId, TypedExpr, TypedExprKind, TypedStmt};

/// Returns the variables a body assigns, in any statement at any depth.
pub(crate) fn assigned_symbols(body: &[TypedStmt]) -> BTreeSet<SymbolId> {
    fn target(assigned: &AssignTarget, found: &mut BTreeSet<SymbolId>) {
        match assigned {
            AssignTarget::Name(symbol) => {
                found.insert(*symbol);
            }
            AssignTarget::Tuple(targets) => {
                for each in targets {
                    target(each, found);
                }
            }
            AssignTarget::Item { .. }
            | AssignTarget::Entry { .. }
            | AssignTarget::Attribute { .. } => {}
        }
    }
    fn statements(body: &[TypedStmt], found: &mut BTreeSet<SymbolId>) {
        for statement in body {
            match statement {
                TypedStmt::Assign { targets, .. } => {
                    for each in targets {
                        target(each, found);
                    }
                }
                TypedStmt::AugAssign { target: each, .. } => target(each, found),
                TypedStmt::If { body, orelse, .. } | TypedStmt::While { body, orelse, .. } => {
                    statements(body, found);
                    statements(orelse, found);
                }
                TypedStmt::ForEach {
                    target: each,
                    body,
                    orelse,
                    ..
                } => {
                    target(each, found);
                    statements(body, found);
                    statements(orelse, found);
                }
                TypedStmt::ForRange {
                    target: symbol,
                    body,
                    orelse,
                    ..
                } => {
                    found.insert(*symbol);
                    statements(body, found);
                    statements(orelse, found);
                }
                TypedStmt::Expr { .. }
                | TypedStmt::Return { .. }
                | TypedStmt::Raise { .. }
                | TypedStmt::Pass { .. }
                | TypedStmt::Break { .. }
                | TypedStmt::Continue { .. } => {}
            }
        }
    }
    let mut found = BTreeSet::new();
    statements(body, &mut found);
    found
}

/// Returns true when a value stays alive for a call without a reference of the caller's
/// own: a variable, which no callee can assign, a constant, or one of them wrapped as an
/// optional value or unwrapped from one.
pub(crate) fn outlives_call(expression: &TypedExpr) -> bool {
    match &expression.kind {
        TypedExprKind::Local(_)
        | TypedExprKind::Constant(_)
        | TypedExprKind::ModuleName
        | TypedExprKind::TypeObject(_) => true,
        TypedExprKind::Wrap(inner) | TypedExprKind::Unwrap(inner) => outlives_call(inner),
        _ => false,
    }
}

impl crate::function::FunctionLowerer<'_> {
    /// Returns an argument the caller keeps alive for the call: a borrowed value read from a
    /// container gets a reference of the caller's own, released after the call.
    pub(crate) fn kept_for_call(
        &mut self,
        expression: &TypedExpr,
        lowered: crate::ownership::Lowered,
        span: typy_diagnostics::Span,
    ) -> crate::ownership::Lowered {
        if lowered.owned || !self.counted(lowered.ty) || outlives_call(expression) {
            return lowered;
        }
        let value = self.take_reference(lowered, span);
        self.produced(value, lowered.ty)
    }
}
