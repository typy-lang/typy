//! Finds the functions that are part of a cycle of calls, which check the stack on entry (FUN-02).

use std::collections::{BTreeMap, BTreeSet};

use typy_check::{FunctionId, TypedExpr, TypedExprKind, TypedProgram, TypedStmt};

/// Returns the functions that can call themselves, directly or through others.
pub(crate) fn recursive_functions(program: &TypedProgram) -> BTreeSet<FunctionId> {
    let calls: BTreeMap<FunctionId, BTreeSet<FunctionId>> = program
        .functions
        .iter()
        .map(|function| {
            let mut callees = BTreeSet::new();
            statements(&function.body, &mut callees);
            (function.id, callees)
        })
        .collect();
    calls
        .keys()
        .copied()
        .filter(|start| {
            let mut seen = BTreeSet::new();
            let mut pending: Vec<FunctionId> = calls[start].iter().copied().collect();
            while let Some(next) = pending.pop() {
                if next == *start {
                    return true;
                }
                if seen.insert(next) {
                    pending.extend(calls.get(&next).into_iter().flatten().copied());
                }
            }
            false
        })
        .collect()
}

fn statements(body: &[TypedStmt], callees: &mut BTreeSet<FunctionId>) {
    for statement in body {
        match statement {
            TypedStmt::Assign { value, .. }
            | TypedStmt::AugAssign { value, .. }
            | TypedStmt::Expr { value, .. } => {
                expression(value, callees);
            }
            TypedStmt::Return {
                value: Some(value), ..
            } => expression(value, callees),
            TypedStmt::If {
                test, body, orelse, ..
            }
            | TypedStmt::While {
                test, body, orelse, ..
            } => {
                expression(test, callees);
                statements(body, callees);
                statements(orelse, callees);
            }
            TypedStmt::ForRange {
                start,
                stop,
                step,
                body,
                orelse,
                ..
            } => {
                for value in [start, stop, step] {
                    expression(value, callees);
                }
                statements(body, callees);
                statements(orelse, callees);
            }
            TypedStmt::Return { value: None, .. }
            | TypedStmt::Pass { .. }
            | TypedStmt::Break { .. }
            | TypedStmt::Continue { .. } => {}
        }
    }
}

fn expression(expression_value: &TypedExpr, callees: &mut BTreeSet<FunctionId>) {
    match &expression_value.kind {
        TypedExprKind::Call {
            function,
            arguments,
        } => {
            callees.insert(*function);
            for argument in arguments {
                expression(&argument.value, callees);
            }
        }
        TypedExprKind::Binary { left, right, .. } => {
            expression(left, callees);
            expression(right, callees);
        }
        TypedExprKind::Unary { operand, .. } | TypedExprKind::TupleItem { tuple: operand, .. } => {
            expression(operand, callees);
        }
        TypedExprKind::BoolOp { values, .. }
        | TypedExprKind::Tuple(values)
        | TypedExprKind::Builtin {
            arguments: values, ..
        } => {
            for value in values {
                expression(value, callees);
            }
        }
        TypedExprKind::Compare {
            left, comparators, ..
        } => {
            expression(left, callees);
            for comparator in comparators {
                expression(comparator, callees);
            }
        }
        TypedExprKind::IfExp { test, body, orelse } => {
            for value in [test, body, orelse] {
                expression(value, callees);
            }
        }
        TypedExprKind::Print { values, sep, end } => {
            for value in values.iter().chain(sep.as_deref()).chain(end.as_deref()) {
                expression(value, callees);
            }
        }
        TypedExprKind::Index { value, index } => {
            expression(value, callees);
            expression(index, callees);
        }
        TypedExprKind::Constant(_) | TypedExprKind::Local(_) | TypedExprKind::ModuleName => {}
    }
}
