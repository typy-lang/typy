//! Borrowed parameters and moved returns (MEM-04). A parameter the function never assigns
//! keeps the reference its caller holds, so passing an argument counts nothing; the caller
//! holds a reference of its own for the call to every argument that is not a variable or a
//! constant, because the callee could otherwise free it by changing the container it was read
//! from. A `return` of a variable hands its slot's reference to the caller.

use std::collections::BTreeSet;

use typy_check::{AssignTarget, SymbolId, TypedExpr, TypedExprKind, TypedStmt};

/// Adds the variables an assignment target assigns, its hidden ones included.
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
        AssignTarget::Lazy { target: inner, .. } => target(inner, found),
        AssignTarget::Converted {
            holder,
            target: inner,
            ..
        } => {
            found.insert(*holder);
            target(inner, found);
        }
        AssignTarget::Item { .. }
        | AssignTarget::Slice { .. }
        | AssignTarget::Entry { .. }
        | AssignTarget::Attribute { .. }
        | AssignTarget::CData { .. }
        | AssignTarget::Property { .. } => {}
    }
}

/// Returns the variables a body assigns, in any statement at any depth.
pub(crate) fn assigned_symbols(body: &[TypedStmt]) -> BTreeSet<SymbolId> {
    fn statements(body: &[TypedStmt], found: &mut BTreeSet<SymbolId>) {
        for statement in body {
            match statement {
                TypedStmt::Assign { targets, .. } => {
                    for each in targets {
                        target(each, found);
                    }
                }
                TypedStmt::AugAssign { target: each, .. } => target(each, found),
                TypedStmt::Import { bindings, .. } => {
                    found.extend(bindings.iter().map(typy_check::ImportBinding::target));
                }
                TypedStmt::LazyImport { targets, .. } => found.extend(targets.iter().copied()),
                TypedStmt::FunctionDef { target, .. } => {
                    found.insert(*target);
                }
                TypedStmt::Match { subject, cases, .. } => {
                    found.extend(subject.iter().map(|(symbol, _)| *symbol));
                    for case in cases {
                        found.extend(case.pattern.bound());
                        statements(&case.body, found);
                    }
                }
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
                TypedStmt::With {
                    target: each, body, ..
                } => {
                    if let Some(each) = each {
                        target(each, found);
                    }
                    statements(body, found);
                }
                TypedStmt::Try {
                    body,
                    handlers,
                    orelse,
                    finalbody,
                    ..
                } => {
                    statements(body, found);
                    for handler in handlers {
                        found.extend(handler.name);
                        statements(&handler.body, found);
                    }
                    statements(orelse, found);
                    statements(finalbody, found);
                }
                TypedStmt::Yield {
                    received: Some(received),
                    ..
                } => statements(std::slice::from_ref(received.as_ref()), found),
                TypedStmt::Expr { .. }
                | TypedStmt::Yield { .. }
                | TypedStmt::Delete { .. }
                | TypedStmt::Assert { .. }
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
        | TypedExprKind::TypeObject(_)
        | TypedExprKind::FunctionValue(_)
        // The running function and its captures, which never change, live while it runs.
        | TypedExprKind::RunningFunction
        | TypedExprKind::Capture(_)
        // A cell lives as long as its slot or the function object that captures it.
        | TypedExprKind::CellObject(_)
        | TypedExprKind::CapturedCellObject(_) => true,
        TypedExprKind::Wrap(inner)
        | TypedExprKind::Unwrap(inner)
        | TypedExprKind::AsIterable(inner) => outlives_call(inner),
        _ => false,
    }
}

/// Returns true when evaluating `expression` runs no code of the program and frees nothing:
/// a variable, a constant, or a field, an item of a list or a tuple, or an operation of the
/// built-in types, of such operands. Anything else may run a method, which may free what an
/// operand evaluated before it borrows (MEM-04, NFR-07).
pub(crate) fn is_quiet(expression: &TypedExpr) -> bool {
    match &expression.kind {
        TypedExprKind::Local(_)
        | TypedExprKind::ModuleVariable(_)
        | TypedExprKind::Constant(_)
        | TypedExprKind::ModuleName
        | TypedExprKind::TypeObject(_)
        | TypedExprKind::TypeAliasObject(_)
        | TypedExprKind::FunctionValue(_)
        | TypedExprKind::RunningFunction
        | TypedExprKind::Capture(_)
        | TypedExprKind::CellRead(_)
        | TypedExprKind::CellObject(_)
        | TypedExprKind::CapturedCell(_)
        | TypedExprKind::CapturedCellObject(_)
        | TypedExprKind::NotImplemented => true,
        TypedExprKind::Attribute {
            object: operand, ..
        }
        | TypedExprKind::TupleItem { tuple: operand, .. }
        | TypedExprKind::CaptureOf {
            closure: operand, ..
        }
        | TypedExprKind::Wrap(operand)
        | TypedExprKind::Unwrap(operand)
        | TypedExprKind::Upcast(operand)
        | TypedExprKind::Downcast(operand)
        | TypedExprKind::AsIterable(operand)
        | TypedExprKind::Unary { operand, .. } => is_quiet(operand),
        TypedExprKind::ListItem {
            list: left,
            index: right,
        }
        | TypedExprKind::Binary { left, right, .. } => is_quiet(left) && is_quiet(right),
        _ => false,
    }
}

impl crate::function::FunctionLowerer<'_> {
    /// Lowers `operand`, which other operands follow: when one of `later` may run code, a
    /// borrowed value that code could free, read from an object, a container or a module
    /// variable, gets a reference of its own, which the operation releases with its other
    /// operands (MEM-04, NFR-07).
    pub(crate) fn operand_before(
        &mut self,
        operand: &TypedExpr,
        later: &[&TypedExpr],
        span: typy_diagnostics::Span,
    ) -> crate::ownership::Lowered {
        let lowered = self.expression(operand);
        if later.iter().all(|each| is_quiet(each)) {
            return lowered;
        }
        self.kept_for_call(operand, lowered, span)
    }

    /// Lowers `operands` in order, each that a later one may free kept with a reference of its
    /// own, as `operand_before` keeps one.
    pub(crate) fn operands_in_order(
        &mut self,
        operands: &[&TypedExpr],
        span: typy_diagnostics::Span,
    ) -> Vec<crate::ownership::Lowered> {
        (0..operands.len())
            .map(|position| {
                self.operand_before(operands[position], &operands[position + 1..], span)
            })
            .collect()
    }

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
