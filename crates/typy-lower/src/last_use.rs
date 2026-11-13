//! Finds the reads of a variable that are its last use and store its value, so lowering moves
//! the value out of the variable instead of counting it (MEM-09).
//!
//! A read is moved when four things hold. It is the value of an assignment, or an item a
//! display, `append`, `insert` or `add` stores. It is outside any comprehension, whose body
//! runs again. The statement reads the variable nowhere else, so no other copy of the value
//! is in use while the statement runs. No path from after the statement reads the variable
//! before assigning it again, counting the `except` clauses and `finally` blocks an exception
//! or a `return` would run. Liveness is computed backwards over the statements; a loop is
//! iterated until what is live at its start stops growing.

use std::collections::BTreeSet;

use typy_check::{AssignTarget, LoopSource, Method, SymbolId, TypedExpr, TypedExprKind, TypedStmt};
use typy_diagnostics::Span;

/// The variables live at a point.
type Live = BTreeSet<SymbolId>;

/// Returns each read, by its variable and its span, that moves its variable's value. Only
/// `candidates` are moved: the variables the function owns, whose slot can take a zero value.
pub(crate) fn last_uses(
    body: &[TypedStmt],
    candidates: &BTreeSet<SymbolId>,
) -> BTreeSet<(SymbolId, Span)> {
    let mut analysis = Analysis {
        candidates,
        moves: BTreeSet::new(),
        loops: Vec::new(),
        escape: Live::new(),
        marking: true,
    };
    analysis.block(body, &Live::new());
    analysis.moves
}

/// Where `break` and `continue` go in the innermost loop: what is live there.
struct LoopExits {
    after: Live,
    start: Live,
}

struct Analysis<'candidates> {
    candidates: &'candidates BTreeSet<SymbolId>,
    moves: BTreeSet<(SymbolId, Span)>,
    loops: Vec<LoopExits>,
    /// What is live wherever the code could leave the statements being analysed abruptly:
    /// an `except` clause or a `finally` block that an exception or a `return` would run.
    escape: Live,
    /// Whether moves are recorded; a loop's body is analysed without recording until what is
    /// live at its start is known.
    marking: bool,
}

impl Analysis<'_> {
    /// Returns what is live before `body` when `after` is live after it.
    fn block(&mut self, body: &[TypedStmt], after: &Live) -> Live {
        let mut live = after.clone();
        for statement in body.iter().rev() {
            live = self.statement(statement, &live);
        }
        live
    }

    #[allow(clippy::too_many_lines, reason = "one arm per statement kind")]
    fn statement(&mut self, statement: &TypedStmt, after: &Live) -> Live {
        let mut after: Live = after.union(&self.escape).copied().collect();
        match statement {
            TypedStmt::Assign { targets, value, .. } => {
                let mut reads = Vec::new();
                for target in targets {
                    target_reads(target, &mut reads);
                }
                expression_reads(value, &mut reads);
                if let ([single], TypedExprKind::Local(symbol)) = (&targets[..], &value.kind)
                    && !matches!(single, AssignTarget::Tuple(_) | AssignTarget::Slice { .. })
                {
                    self.consider(*symbol, value.span, &reads, &after);
                }
                self.stored_items(value, &reads, &after);
                for target in targets {
                    target_defines(target, &mut after);
                }
                self.with_reads(after, &reads)
            }
            TypedStmt::AugAssign { target, value, .. } => {
                let mut reads = Vec::new();
                target_reads(target, &mut reads);
                if let AssignTarget::Name(symbol) = target {
                    reads.push(*symbol);
                }
                expression_reads(value, &mut reads);
                self.stored_items(value, &reads, &after);
                self.with_reads(after, &reads)
            }
            TypedStmt::Expr { value, .. } => {
                let mut reads = Vec::new();
                expression_reads(value, &mut reads);
                self.stored_items(value, &reads, &after);
                self.with_reads(after, &reads)
            }
            TypedStmt::Return { value, .. } => {
                let mut reads = Vec::new();
                let escape = self.escape.clone();
                if let Some(value) = value {
                    expression_reads(value, &mut reads);
                    self.stored_items(value, &reads, &escape);
                }
                self.with_reads(escape, &reads)
            }
            TypedStmt::Raise {
                exception, cause, ..
            } => {
                let mut reads = Vec::new();
                if let Some(exception) = exception {
                    expression_reads(exception, &mut reads);
                }
                if let typy_check::RaiseCause::Exception(cause) = cause {
                    expression_reads(cause, &mut reads);
                }
                self.with_reads(self.escape.clone(), &reads)
            }
            TypedStmt::Break { .. } => {
                let exits = self.loops.last().map(|exits| exits.after.clone());
                exits
                    .unwrap_or_default()
                    .union(&self.escape)
                    .copied()
                    .collect()
            }
            TypedStmt::Continue { .. } => {
                let exits = self.loops.last().map(|exits| exits.start.clone());
                exits
                    .unwrap_or_default()
                    .union(&self.escape)
                    .copied()
                    .collect()
            }
            TypedStmt::If {
                test, body, orelse, ..
            } => {
                let mut live = self.block(body, &after);
                live.extend(self.block(orelse, &after));
                self.with_expression(live, test)
            }
            TypedStmt::While {
                test, body, orelse, ..
            } => {
                let exit = self.block(orelse, &after);
                self.fixed_point(&after, |analysis, start| {
                    let mut live = analysis.block(body, start);
                    live.extend(exit.iter().copied());
                    analysis.with_expression(live, test)
                })
            }
            TypedStmt::ForEach {
                target,
                source,
                body,
                orelse,
                ..
            } => {
                let exit = self.block(orelse, &after);
                let start = self.fixed_point(&after, |analysis, start| {
                    let mut live = analysis.block(body, start);
                    target_defines(target, &mut live);
                    let mut reads = Vec::new();
                    target_reads(target, &mut reads);
                    live.extend(
                        reads
                            .into_iter()
                            .filter(|read| analysis.candidates.contains(read)),
                    );
                    live.extend(exit.iter().copied());
                    live
                });
                let mut reads = Vec::new();
                source_reads(source, &mut reads);
                self.with_reads(start, &reads)
            }
            TypedStmt::ForRange {
                target,
                start: first,
                stop,
                step,
                body,
                orelse,
                ..
            } => {
                let exit = self.block(orelse, &after);
                let start = self.fixed_point(&after, |analysis, start| {
                    let mut live = analysis.block(body, start);
                    live.remove(target);
                    live.extend(exit.iter().copied());
                    live
                });
                let mut reads = Vec::new();
                for value in [first, stop, step] {
                    expression_reads(value, &mut reads);
                }
                self.with_reads(start, &reads)
            }
            TypedStmt::Try {
                body,
                handlers,
                orelse,
                finalbody,
                ..
            } => {
                let outer = self.escape.clone();
                let after_finally = self.block(finalbody, &after);
                // Inside the clauses and the `else` block, leaving abruptly runs `finally`.
                self.escape = outer.union(&after_finally).copied().collect();
                let mut handled = Live::new();
                for handler in handlers {
                    handled.extend(self.block(&handler.body, &after_finally));
                }
                let before_else = self.block(orelse, &after_finally);
                // Inside the body, an exception may also go to any clause.
                self.escape.extend(handled.iter().copied());
                let live = self.block(body, &before_else);
                self.escape = outer;
                live
            }
            TypedStmt::With {
                manager,
                target,
                body,
                ..
            } => {
                let outer = self.escape.clone();
                // An exception `__exit__` suppresses goes on after the `with`.
                self.escape.extend(after.iter().copied());
                let mut live = self.block(body, &after);
                self.escape = outer;
                if let Some(target) = target {
                    target_defines(target, &mut live);
                }
                self.with_expression(live, manager)
            }
            TypedStmt::Delete { targets, .. } => {
                let mut reads = Vec::new();
                for target in targets {
                    expression_reads(target, &mut reads);
                }
                self.with_reads(after, &reads)
            }
            TypedStmt::Assert {
                test, exception, ..
            } => {
                let mut reads = Vec::new();
                expression_reads(test, &mut reads);
                expression_reads(exception, &mut reads);
                self.with_reads(after, &reads)
            }
            TypedStmt::Import { bindings, .. } => {
                for binding in bindings {
                    after.remove(&binding.target());
                }
                after
            }
            TypedStmt::LazyImport { .. } | TypedStmt::Pass { .. } => after,
        }
    }

    /// Returns what is live at the start of a loop: iterates `start_of` from nothing live
    /// until it stops growing, without recording moves, then once more recording them.
    fn fixed_point(&mut self, after: &Live, start_of: impl Fn(&mut Self, &Live) -> Live) -> Live {
        let marking = std::mem::replace(&mut self.marking, false);
        let mut start = Live::new();
        loop {
            self.loops.push(LoopExits {
                after: after.clone(),
                start: start.clone(),
            });
            let next = start_of(self, &start);
            self.loops.pop();
            if next == start {
                break;
            }
            start = next;
        }
        self.marking = marking;
        self.loops.push(LoopExits {
            after: after.clone(),
            start: start.clone(),
        });
        let start = start_of(self, &start);
        self.loops.pop();
        start
    }

    /// Records a read of `symbol` at `span` as a move when the statement reads it only there
    /// and it is not live after the statement.
    fn consider(&mut self, symbol: SymbolId, span: Span, reads: &[SymbolId], after: &Live) {
        if self.marking
            && self.candidates.contains(&symbol)
            && !after.contains(&symbol)
            && reads.iter().filter(|read| **read == symbol).count() == 1
        {
            self.moves.insert((symbol, span));
        }
    }

    /// Considers each variable read as an item that a display, `append`, `insert` or `add`
    /// stores, outside any comprehension.
    fn stored_items(&mut self, value: &TypedExpr, reads: &[SymbolId], after: &Live) {
        let mut stored = Vec::new();
        stored_reads(value, &mut stored);
        for (symbol, span) in stored {
            self.consider(symbol, span, reads, after);
        }
    }

    /// Returns `after` with the variables of `reads` that are candidates added.
    fn with_reads(&self, mut after: Live, reads: &[SymbolId]) -> Live {
        after.extend(reads.iter().filter(|read| self.candidates.contains(read)));
        after
    }

    /// Returns `after` with the variables `expression` reads added.
    fn with_expression(&self, after: Live, expression: &TypedExpr) -> Live {
        let mut reads = Vec::new();
        expression_reads(expression, &mut reads);
        self.with_reads(after, &reads)
    }
}

/// Adds each variable `expression` reads, at any depth.
fn expression_reads(expression: &TypedExpr, reads: &mut Vec<SymbolId>) {
    if let TypedExprKind::Local(symbol) = &expression.kind {
        reads.push(*symbol);
    }
    for operand in expression.operands() {
        expression_reads(operand, reads);
    }
}

/// Adds each variable a loop source reads.
fn source_reads(source: &LoopSource, reads: &mut Vec<SymbolId>) {
    for expression in source.expressions() {
        expression_reads(expression, reads);
    }
}

/// Adds each variable an assignment target reads: the list, dict or instance it stores into,
/// and the index or key.
fn target_reads(target: &AssignTarget, reads: &mut Vec<SymbolId>) {
    match target {
        AssignTarget::Name(_) => {}
        AssignTarget::Tuple(targets) => {
            for each in targets {
                target_reads(each, reads);
            }
        }
        AssignTarget::Item { list, index } => {
            expression_reads(list, reads);
            expression_reads(index, reads);
        }
        AssignTarget::Slice {
            list,
            lower,
            upper,
            step,
        } => {
            expression_reads(list, reads);
            for bound in [lower, upper, step].into_iter().flatten() {
                expression_reads(bound, reads);
            }
        }
        AssignTarget::Attribute { object, .. } => expression_reads(object, reads),
        AssignTarget::Entry { dict, key } => {
            expression_reads(dict, reads);
            expression_reads(key, reads);
        }
    }
}

/// Removes from `live` each variable an assignment target binds.
fn target_defines(target: &AssignTarget, live: &mut Live) {
    match target {
        AssignTarget::Name(symbol) => {
            live.remove(symbol);
        }
        AssignTarget::Tuple(targets) => {
            for each in targets {
                target_defines(each, live);
            }
        }
        _ => {}
    }
}

/// Adds each variable read as an item that a display, `append`, `insert` or `add` stores,
/// with the read's span, outside any comprehension.
fn stored_reads(expression: &TypedExpr, stored: &mut Vec<(SymbolId, Span)>) {
    let local = |item: &TypedExpr| match &item.kind {
        TypedExprKind::Local(symbol) => Some((*symbol, item.span)),
        _ => None,
    };
    match &expression.kind {
        // A comprehension's body runs once for each value, so nothing in it is moved.
        TypedExprKind::Comprehension { .. } => return,
        TypedExprKind::List(items) | TypedExprKind::Set(items) | TypedExprKind::Tuple(items) => {
            stored.extend(items.iter().filter_map(local));
        }
        TypedExprKind::Dict { values, .. } => {
            stored.extend(values.iter().filter_map(local));
        }
        TypedExprKind::Method {
            method: method @ (Method::ListAppend | Method::ListInsert | Method::SetAdd),
            arguments,
            ..
        } => {
            // The item is the first parameter, and the second of `insert`.
            let item = usize::from(*method == Method::ListInsert);
            stored.extend(
                arguments
                    .iter()
                    .find(|argument| argument.parameter == item)
                    .and_then(|argument| local(&argument.value)),
            );
        }
        _ => {}
    }
    for operand in expression.operands() {
        stored_reads(operand, stored);
    }
}

/// Returns whether a variable of type `ty` can be moved: its slot's zero value is a constant,
/// which a view of a dict's is not.
pub(crate) fn movable_type(ty: &typy_check::Type) -> bool {
    use typy_check::Type;
    match ty {
        Type::DictView(..) => false,
        Type::Tuple(items) => items.iter().all(movable_type),
        Type::Optional(inner) => movable_type(inner),
        _ => true,
    }
}

/// Returns the variables an `except` clause binds, which leaving the clause releases, so they
/// are never moved.
pub(crate) fn handler_names(body: &[TypedStmt]) -> BTreeSet<SymbolId> {
    let mut names = BTreeSet::new();
    let mut pending: Vec<&[TypedStmt]> = vec![body];
    while let Some(statements) = pending.pop() {
        for statement in statements {
            match statement {
                TypedStmt::Try {
                    body,
                    handlers,
                    orelse,
                    finalbody,
                    ..
                } => {
                    names.extend(handlers.iter().filter_map(|handler| handler.name));
                    pending.extend(handlers.iter().map(|handler| handler.body.as_slice()));
                    pending.extend([body.as_slice(), orelse, finalbody]);
                }
                TypedStmt::If { body, orelse, .. }
                | TypedStmt::While { body, orelse, .. }
                | TypedStmt::ForEach { body, orelse, .. }
                | TypedStmt::ForRange { body, orelse, .. } => {
                    pending.extend([body.as_slice(), orelse]);
                }
                TypedStmt::With { body, .. } => pending.push(body),
                _ => {}
            }
        }
    }
    names
}

#[cfg(test)]
mod tests {
    use super::movable_type;
    use typy_check::{Type, View};

    #[test]
    fn a_type_holding_a_view_of_a_dict_is_not_moved() {
        let view = Type::DictView(View::Keys, Box::new(Type::Str), Box::new(Type::Int));
        for (ty, movable) in [
            (Type::Str, true),
            (Type::List(Box::new(Type::Int)), true),
            (Type::Tuple(vec![Type::Str, Type::Int]), true),
            (Type::Optional(Box::new(Type::Str)), true),
            (view.clone(), false),
            (Type::Tuple(vec![Type::Int, view.clone()]), false),
            (Type::Optional(Box::new(view)), false),
        ] {
            assert_eq!(movable_type(&ty), movable, "{ty:?}");
        }
    }
}
