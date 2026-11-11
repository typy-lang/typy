//! Finds the functions that are part of a cycle of calls, which check the stack on entry (FUN-02).

use std::collections::{BTreeMap, BTreeSet};

use typy_check::{
    AssignTarget, FunctionId, LoopSource, RaiseCause, TypedExpr, TypedExprKind, TypedProgram,
    TypedStmt,
};

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

#[allow(clippy::too_many_lines, reason = "one arm per statement kind")]
fn statements(body: &[TypedStmt], callees: &mut BTreeSet<FunctionId>) {
    for statement in body {
        match statement {
            TypedStmt::Assign { targets, value, .. } => {
                expression(value, callees);
                for target in targets {
                    assign_target(target, callees);
                }
            }
            TypedStmt::AugAssign { target, value, .. } => {
                assign_target(target, callees);
                expression(value, callees);
            }
            TypedStmt::Expr { value, .. } => {
                expression(value, callees);
            }
            TypedStmt::ForEach {
                target,
                source,
                body,
                orelse,
                ..
            } => {
                loop_source(source, callees);
                assign_target(target, callees);
                statements(body, callees);
                statements(orelse, callees);
            }
            TypedStmt::Return {
                value: Some(value), ..
            } => expression(value, callees),
            TypedStmt::With {
                manager,
                enter,
                exit,
                target,
                body,
                ..
            } => {
                expression(manager, callees);
                callees.insert(*enter);
                callees.insert(*exit);
                if let Some(target) = target {
                    assign_target(target, callees);
                }
                statements(body, callees);
            }
            TypedStmt::Delete { targets, .. } => {
                for target in targets {
                    expression(target, callees);
                }
            }
            TypedStmt::Assert {
                test, exception, ..
            } => {
                expression(test, callees);
                expression(exception, callees);
            }
            TypedStmt::Try {
                body,
                handlers,
                orelse,
                finalbody,
                ..
            } => {
                statements(body, callees);
                for handler in handlers {
                    statements(&handler.body, callees);
                }
                statements(orelse, callees);
                statements(finalbody, callees);
            }
            TypedStmt::Raise {
                exception, cause, ..
            } => {
                let cause = match cause {
                    RaiseCause::Exception(cause) => Some(cause),
                    RaiseCause::Absent | RaiseCause::None => None,
                };
                for value in exception.iter().chain(cause) {
                    expression(value, callees);
                }
            }
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

/// Finds calls in an assignment target: the list and the index of an item target.
fn assign_target(target: &AssignTarget, callees: &mut BTreeSet<FunctionId>) {
    match target {
        AssignTarget::Name(_) => {}
        AssignTarget::Tuple(targets) => {
            for target in targets {
                assign_target(target, callees);
            }
        }
        AssignTarget::Slice {
            list,
            lower,
            upper,
            step,
        } => {
            expression(list, callees);
            for bound in [lower, upper, step].into_iter().flatten() {
                expression(bound, callees);
            }
        }
        AssignTarget::Item { list, index } => {
            expression(list, callees);
            expression(index, callees);
        }
        AssignTarget::Entry { dict, key } => {
            expression(dict, callees);
            expression(key, callees);
        }
        AssignTarget::Attribute { object, .. } => expression(object, callees),
    }
}

#[allow(clippy::too_many_lines, reason = "one arm per expression kind")]
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
        TypedExprKind::Wrap(operand)
        | TypedExprKind::Unwrap(operand)
        | TypedExprKind::Unary { operand, .. }
        | TypedExprKind::TupleOf(operand)
        | TypedExprKind::TupleItem { tuple: operand, .. }
        | TypedExprKind::TupleSlice { tuple: operand, .. } => {
            expression(operand, callees);
        }
        TypedExprKind::ListItem { list, index }
        | TypedExprKind::DictItem {
            dict: list,
            key: index,
        } => {
            expression(list, callees);
            expression(index, callees);
        }
        TypedExprKind::Comprehension {
            element,
            value,
            generators,
        } => {
            expression(element, callees);
            if let Some(value) = value {
                expression(value, callees);
            }
            for generator in generators {
                assign_target(&generator.target, callees);
                protocol_calls(&generator.source, callees);
                for value in generator
                    .source
                    .expressions()
                    .into_iter()
                    .chain(&generator.conditions)
                {
                    expression(value, callees);
                }
            }
        }
        TypedExprKind::Dict { keys, values } => {
            for value in keys.iter().chain(values) {
                expression(value, callees);
            }
        }
        TypedExprKind::ListOf(source)
        | TypedExprKind::SetOf(source)
        | TypedExprKind::DictOf(source) => {
            loop_source(source, callees);
        }
        TypedExprKind::DictWith { source, entries } => {
            if let Some(source) = source {
                loop_source(source, callees);
            }
            for (_, value) in entries {
                expression(value, callees);
            }
        }
        TypedExprKind::DictFromKeys { source, value } => {
            loop_source(source, callees);
            if let Some(value) = value {
                expression(value, callees);
            }
        }
        TypedExprKind::Slice {
            value,
            lower,
            upper,
            step,
        } => {
            expression(value, callees);
            for bound in [lower, upper, step].into_iter().flatten() {
                expression(bound, callees);
            }
        }
        TypedExprKind::Method {
            receiver,
            arguments,
            ..
        } => {
            expression(receiver, callees);
            for argument in arguments {
                expression(&argument.value, callees);
            }
        }
        TypedExprKind::IterableBuiltin {
            iterable, option, ..
        } => {
            expression(iterable, callees);
            if let Some(option) = option {
                expression(option, callees);
            }
        }
        TypedExprKind::BoolOp { values, .. }
        | TypedExprKind::Tuple(values)
        | TypedExprKind::List(values)
        | TypedExprKind::Set(values)
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
        TypedExprKind::NewException {
            arguments, init, ..
        } => {
            for argument in arguments {
                expression(argument, callees);
            }
            if let Some(init) = init {
                callees.insert(init.function);
                for argument in &init.arguments {
                    expression(&argument.value, callees);
                }
            }
        }
        TypedExprKind::Iter(source) => expression(source, callees),
        TypedExprKind::Next {
            iterator,
            method,
            default,
        } => {
            callees.extend(*method);
            expression(iterator, callees);
            if let Some(default) = default {
                expression(default, callees);
            }
        }
        TypedExprKind::SetArguments {
            exception,
            arguments,
        } => {
            expression(exception, callees);
            for argument in arguments {
                expression(argument, callees);
            }
        }
        TypedExprKind::New {
            init, arguments, ..
        } => {
            callees.extend(*init);
            for argument in arguments {
                expression(&argument.value, callees);
            }
        }
        TypedExprKind::Attribute { object, .. }
        | TypedExprKind::TypeOf(object)
        | TypedExprKind::TypeName(object) => expression(object, callees),
        TypedExprKind::FString(parts) => fstring(parts, callees),
        TypedExprKind::TypeObject(_)
        | TypedExprKind::Constant(_)
        | TypedExprKind::Local(_)
        | TypedExprKind::ModuleName => {}
    }
}

/// Finds calls in the fields of an f-string and of its specifications.
fn fstring(parts: &[typy_check::FStringPart], callees: &mut BTreeSet<FunctionId>) {
    for part in parts {
        if let typy_check::FStringPart::Field(field) = part {
            expression(&field.value, callees);
            if let Some(spec) = &field.spec {
                fstring(spec, callees);
            }
        }
    }
}

/// Adds the calls a loop source makes: its expressions', and a class's `__iter__` and
/// `__next__`.
fn loop_source(source: &LoopSource, callees: &mut BTreeSet<FunctionId>) {
    for value in source.expressions() {
        expression(value, callees);
    }
    protocol_calls(source, callees);
}

/// Adds the `__iter__` and `__next__` of every class a loop source iterates over.
fn protocol_calls(source: &LoopSource, callees: &mut BTreeSet<FunctionId>) {
    match source {
        LoopSource::Protocol { iter, next, .. } => {
            callees.insert(*iter);
            callees.insert(*next);
        }
        LoopSource::Enumerate { source, .. } => protocol_calls(source, callees),
        LoopSource::Zip { sources, .. } => {
            for source in sources {
                protocol_calls(source, callees);
            }
        }
        LoopSource::Items(_) | LoopSource::Reversed(_) | LoopSource::Range { .. } => {}
    }
}
