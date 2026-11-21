//! Finds the functions that are part of a cycle of calls, which check the stack on entry (FUN-02).

use std::collections::{BTreeMap, BTreeSet};

use typy_check::{
    AssignTarget, ClassId, FunctionId, LoopSource, RaiseCause, TypedExpr, TypedExprKind,
    TypedProgram, TypedStmt,
};

/// Stands among a function's callees for a call through a function value, whose code is known
/// only when the program runs (FUN-05).
const THROUGH_VALUE: FunctionId = FunctionId(u32::MAX);

/// Returns the functions that can call themselves, directly or through others. A cycle that
/// passes through a function value has a call through a value in it, so each function that
/// calls through one checks the stack, and a cycle of values raises `RecursionError` too.
pub(crate) fn recursive_functions(program: &TypedProgram) -> BTreeSet<FunctionId> {
    let overrides = overrides(program);
    let calls: BTreeMap<FunctionId, BTreeSet<FunctionId>> = program
        .functions
        .iter()
        .map(|function| {
            let mut callees = BTreeSet::new();
            statements(&function.body, &mut callees);
            // A call of a method may run any method that overrides it (ADR-0021).
            let reached: Vec<FunctionId> = callees
                .iter()
                .flat_map(|callee| overrides.get(callee).into_iter().flatten().copied())
                .collect();
            callees.extend(reached);
            (function.id, callees)
        })
        .collect();
    calls
        .keys()
        .copied()
        .filter(|start| {
            if calls[start].contains(&THROUGH_VALUE) {
                return true;
            }
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

/// Returns, for each method, the methods of the same name of the classes derived from its
/// class.
fn overrides(program: &TypedProgram) -> BTreeMap<FunctionId, Vec<FunctionId>> {
    let derives_from = |class: ClassId, base: ClassId| {
        let mut current = program.class(class).base;
        while let Some(each) = current {
            if each == base {
                return true;
            }
            current = program.class(each).base;
        }
        false
    };
    let mut found: BTreeMap<FunctionId, Vec<FunctionId>> = BTreeMap::new();
    for class in &program.classes {
        for (name, method) in &class.methods {
            let below = program
                .classes
                .iter()
                .filter(|other| derives_from(other.id, class.id))
                .flat_map(|other| &other.methods)
                .filter(|(each, _)| each == name)
                .map(|(_, id)| *id);
            found.entry(*method).or_default().extend(below);
        }
    }
    found
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
            TypedStmt::Expr { value, .. }
            | TypedStmt::Return {
                value: Some(value), ..
            } => expression(value, callees),
            TypedStmt::Yield {
                value, received, ..
            } => {
                expression(value, callees);
                if let Some(received) = received {
                    statements(std::slice::from_ref(received.as_ref()), callees);
                }
            }
            TypedStmt::YieldFrom {
                source,
                item,
                received,
                ..
            } => {
                expression(source, callees);
                expression(item, callees);
                if let Some(received) = received {
                    statements(std::slice::from_ref(received.as_ref()), callees);
                }
            }
            TypedStmt::FunctionDef { captures, .. } => {
                for capture in captures {
                    expression(capture, callees);
                }
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
            TypedStmt::With {
                manager,
                protocol,
                target,
                body,
                ..
            } => {
                expression(manager, callees);
                if let typy_check::WithProtocol::Methods { enter, exit } = protocol {
                    callees.insert(*enter);
                    callees.insert(*exit);
                }
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
            TypedStmt::Match { subject, cases, .. } => {
                if let Some((_, value)) = subject {
                    expression(value, callees);
                }
                for case in cases {
                    for each in case.pattern.expressions().into_iter().chain(&case.guard) {
                        expression(each, callees);
                    }
                    statements(&case.body, callees);
                }
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
            // Another module's lazy import a name needs is a call.
            TypedStmt::Import { pending, .. } => callees.extend(pending.iter().copied()),
            // A module's code runs once and never recursively, so an import adds no edge.
            TypedStmt::Return { value: None, .. }
            | TypedStmt::LazyImport { .. }
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
        AssignTarget::Lazy { reify, target } => {
            callees.insert(*reify);
            assign_target(target, callees);
        }
        AssignTarget::Tuple(_) | AssignTarget::Unpack { .. } => {
            for nested in target.nested() {
                assign_target(nested, callees);
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
        AssignTarget::Property { object, setter, .. } => {
            callees.insert(*setter);
            expression(object, callees);
        }
        AssignTarget::Converted { value, target, .. } => {
            expression(value, callees);
            assign_target(target, callees);
        }
        AssignTarget::CData { object, place } => {
            expression(object, callees);
            if let typy_check::CDataPlace::Item(index) = place {
                expression(index, callees);
            }
        }
    }
}

#[allow(clippy::too_many_lines, reason = "one arm per expression kind")]
fn expression(expression_value: &TypedExpr, callees: &mut BTreeSet<FunctionId>) {
    match &expression_value.kind {
        TypedExprKind::Call {
            function,
            arguments,
            ..
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
        TypedExprKind::Lambda { captures, .. } => {
            for capture in captures {
                expression(capture, callees);
            }
        }
        TypedExprKind::CallNested {
            function,
            closure,
            arguments,
        } => {
            callees.insert(*function);
            expression(closure, callees);
            for argument in arguments {
                expression(&argument.value, callees);
            }
        }
        TypedExprKind::CallValue { callee, arguments } => {
            callees.insert(THROUGH_VALUE);
            expression(callee, callees);
            for argument in arguments {
                expression(argument, callees);
            }
        }
        // The key of each item is called through its function value.
        TypedExprKind::Keyed { .. } => {
            callees.insert(THROUGH_VALUE);
            for part in expression_value.operands() {
                expression(part, callees);
            }
        }
        TypedExprKind::IsInstance(test) => expression(&test.value, callees),
        TypedExprKind::EachMember(each) => {
            expression(&each.value, callees);
            for (_, body) in &each.arms {
                expression(body, callees);
            }
        }
        TypedExprKind::OperatorCall(call) => {
            callees.extend(call.forward);
            callees.extend(call.reflected);
            callees.extend(call.in_place);
            callees.extend(call.reflected_below.iter().map(|(_, method)| *method));
            expression(&call.left, callees);
            expression(&call.right, callees);
        }
        TypedExprKind::Wrap(operand)
        | TypedExprKind::CheckedLength(operand)
        | TypedExprKind::Upcast(operand)
        | TypedExprKind::Downcast(operand)
        | TypedExprKind::Unwrap(operand)
        | TypedExprKind::Unary { operand, .. }
        | TypedExprKind::TupleOf(operand)
        | TypedExprKind::Raise(operand)
        | TypedExprKind::Hash(operand)
        | TypedExprKind::TupleItem { tuple: operand, .. }
        | TypedExprKind::TupleSlice { tuple: operand, .. }
        | TypedExprKind::SliceBound { slice: operand, .. } => {
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
        TypedExprKind::Spread { parts, .. } => {
            for part in parts {
                match part {
                    typy_check::SpreadPart::Item(item) => expression(item, callees),
                    typy_check::SpreadPart::Each(source) => loop_source(source, callees),
                }
            }
        }
        TypedExprKind::DictSpread(parts) => {
            for part in parts {
                match part {
                    typy_check::DictPart::Entry(key, value) => {
                        expression(key, callees);
                        expression(value, callees);
                    }
                    typy_check::DictPart::Merge(dict) => expression(dict, callees),
                }
            }
        }
        TypedExprKind::ListOf(source)
        | TypedExprKind::SetOf(source)
        | TypedExprKind::DictOf(source) => {
            loop_source(source, callees);
        }
        TypedExprKind::IterContains {
            value,
            source,
            test,
            ..
        } => {
            expression(value, callees);
            loop_source(source, callees);
            expression(test, callees);
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
        TypedExprKind::Print(call) => {
            for value in call
                .values
                .iter()
                .chain(&call.sep)
                .chain(&call.end)
                .chain(&call.file)
                .chain(&call.flush)
            {
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
        | TypedExprKind::TypeName(object)
        | TypedExprKind::TypeAliasPart { alias: object, .. }
        | TypedExprKind::BoundMethod { object, .. }
        | TypedExprKind::ChosenFunction { object, .. } => expression(object, callees),
        TypedExprKind::VersionRead { value, result } => {
            expression(value, callees);
            expression(result, callees);
        }
        TypedExprKind::FString(parts) => fstring(parts, callees),
        TypedExprKind::TypeObject(_)
        | TypedExprKind::Constant(_)
        | TypedExprKind::Local(_)
        | TypedExprKind::ModuleVariable(_)
        | TypedExprKind::ModuleAttribute { .. }
        | TypedExprKind::Submodule(_)
        | TypedExprKind::ModuleName
        | TypedExprKind::NotImplemented
        | TypedExprKind::ClassVariable(_)
        | TypedExprKind::TypeAliasObject(_)
        | TypedExprKind::FunctionValue(_)
        | TypedExprKind::Capture(_)
        | TypedExprKind::RunningFunction
        | TypedExprKind::Received
        | TypedExprKind::DelegatedItem
        | TypedExprKind::DelegatedResult
        | TypedExprKind::CellRead(_)
        | TypedExprKind::CellObject(_)
        | TypedExprKind::CapturedCell(_)
        | TypedExprKind::CapturedCellObject(_)
        | TypedExprKind::ForeignLibrary(_)
        | TypedExprKind::SysValue(_) => {}
        TypedExprKind::AsIterable(inner)
        | TypedExprKind::AsIterator(inner)
        | TypedExprKind::Named { value: inner, .. }
        | TypedExprKind::CaptureOf { closure: inner, .. } => {
            expression(inner, callees);
        }
        TypedExprKind::PercentFormat { values, .. } => expression(values, callees),
        TypedExprKind::Open { .. } => {
            for part in expression_value.operands() {
                expression(part, callees);
            }
        }
        TypedExprKind::ExceptionArguments { exception, .. }
        | TypedExprKind::FileAttribute {
            file: exception, ..
        } => expression(exception, callees),
        TypedExprKind::PercentDynamic { format, values }
        | TypedExprKind::Sequence {
            first: format,
            then: values,
        } => {
            expression(format, callees);
            expression(values, callees);
        }
        TypedExprKind::ExceptionPart {
            exception,
            operation,
            ..
        } => {
            expression(exception, callees);
            if let typy_check::PartOperation::Equal(other) = operation {
                expression(other, callees);
            }
        }
        TypedExprKind::SysExit(value)
        | TypedExprKind::Input(value)
        | TypedExprKind::RecursionLimit(value) => {
            if let Some(value) = value {
                expression(value, callees);
            }
        }
        TypedExprKind::MathCall { arguments, .. }
        | TypedExprKind::ForeignCall { arguments, .. } => {
            for argument in arguments {
                expression(argument, callees);
            }
        }
        TypedExprKind::CData(cdata) => {
            for part in cdata.expressions() {
                expression(part, callees);
            }
        }
        TypedExprKind::Lazy { reify, value } => {
            callees.insert(*reify);
            expression(value, callees);
        }
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
