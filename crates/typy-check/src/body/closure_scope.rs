//! Finds, from a function's source alone, the variables each nested function captures and
//! which of them live in a cell (FUN-04, ADR-0061), as CPython's symbol table finds free and
//! cell variables. A variable is copied when the nested function is made when one binding
//! gives its value, outside every loop and before each nested function that captures it;
//! any other captured variable lives in a cell, which every function that captures it shares.
//! Nested functions that call one another by name form a group, each of which holds what any
//! of them captures, so none holds another and they form no reference cycle. A variable that
//! a nested function names `nonlocal` lives in a cell (SMT-09).

use std::collections::{BTreeMap, BTreeSet};

use typy_diagnostics::Span;
use typy_source::syntax::{Comprehension, ExceptHandler, Expr, FStringPart, Parameter, Stmt};

/// What a function's nested functions capture of its own variables, and of those it captures
/// itself.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct ClosureScope {
    /// For each nested `def` of the function, by the span of its statement, the variables it
    /// reads from the function, directly or through the functions nested in it, by name.
    pub(crate) captures: BTreeMap<Span, BTreeSet<String>>,
    /// The function's own variables that live in a cell.
    pub(crate) cells: BTreeSet<String>,
    /// For each nested `def` of a group, by the span of its statement, the names of the
    /// group's functions, which a call by name inside it runs with the running function
    /// object, since each holds the captures of the whole group.
    pub(crate) groups: BTreeMap<Span, BTreeSet<String>>,
    /// The function's own variables that a nested function assigns through `nonlocal`, which
    /// any call may change, so no test narrows them.
    pub(crate) rebound: BTreeSet<String>,
}

impl ClosureScope {
    /// Returns the scope of the function of `parameters` and `body`, nested in functions whose
    /// variables `enclosing` names.
    pub(crate) fn of(parameters: &[&str], body: &[Stmt], enclosing: &BTreeSet<String>) -> Self {
        let names = Names::of(parameters, body);
        let own = names.bound_names();
        let available: BTreeSet<String> = own
            .iter()
            .cloned()
            .chain(names.free().intersection(enclosing).cloned())
            .collect();
        let mut scope = Self::default();
        for group in names.groups(body) {
            let members: BTreeSet<String> =
                group.iter().map(|nested| nested.name.clone()).collect();
            // A call by name among the group passes the running function object, so each
            // holds, in one order, what any of them captures, and none holds another.
            let captured: BTreeSet<String> = group
                .iter()
                .flat_map(|nested| nested.free.intersection(&available))
                .filter(|name| !members.contains(*name))
                .cloned()
                .collect();
            for nested in group {
                scope.captures.insert(nested.span, captured.clone());
                scope.groups.insert(nested.span, members.clone());
            }
        }
        for nested in &names.nested {
            for name in nested.assigned.intersection(&own) {
                scope.rebound.insert(name.clone());
                scope.cells.insert(name.clone());
            }
            // A nested function's own name reads the running function object, since only its
            // `def` binds it, so it is no capture.
            let captured = scope
                .captures
                .entry(nested.span)
                .or_insert_with(|| {
                    nested
                        .free
                        .intersection(&available)
                        .filter(|each| **each != nested.name)
                        .cloned()
                        .collect()
                })
                .clone();
            for name in captured.intersection(&own) {
                if !names.copied(name, nested.span.start().0) {
                    scope.cells.insert(name.clone());
                }
            }
        }
        scope
    }
}

impl ClosureScope {
    /// Returns the scope of a module's body, whose own variables are globals that every
    /// function reads as they are: only a comprehension's variable that a `lambda` in it
    /// reads is captured, in a cell.
    pub(crate) fn of_module(statements: &[&Stmt]) -> Self {
        let mut names = Names::default();
        for statement in statements {
            names.statement(statement, false);
        }
        let cells = names.comprehension_cells;
        let captures = names
            .nested
            .iter()
            .map(|nested| {
                (
                    nested.span,
                    nested.free.intersection(&cells).cloned().collect(),
                )
            })
            .collect();
        Self {
            captures,
            cells,
            ..Self::default()
        }
    }
}

/// One nested `def` of a function: where it is, its name, the names it reads of the
/// functions around it, those of them it reads other than by calling them by name, or
/// through the functions nested in it, and those it or a function nested in it assigns
/// through `nonlocal`.
struct NestedDef {
    span: Span,
    name: String,
    free: BTreeSet<String>,
    values: BTreeSet<String>,
    assigned: BTreeSet<String>,
}

/// One binding of a variable: where it is, whether a loop runs it again, and whether it
/// gives a value, which an annotation alone, such as `x: int`, does not.
#[derive(Clone, Copy, Debug)]
struct Binding {
    offset: u32,
    in_loop: bool,
    gives_value: bool,
}

/// The names one function binds and reads, and the free variables of its nested functions.
#[derive(Default)]
struct Names {
    bindings: BTreeMap<String, Vec<Binding>>,
    globals: BTreeSet<String>,
    nonlocals: BTreeSet<String>,
    reads: BTreeSet<String>,
    /// The names read other than by calling them by name.
    values: BTreeSet<String>,
    /// Each nested `def` and `lambda`, in source order.
    nested: Vec<NestedDef>,
    /// The variables of comprehensions that a `lambda` inside them reads, which live in a
    /// cell each run of the comprehension makes anew, as CPython 3.12 to 3.15 keep them.
    comprehension_cells: BTreeSet<String>,
}

impl Names {
    /// Returns the names the function of `parameters` and `body` binds and reads.
    fn of(parameters: &[&str], body: &[Stmt]) -> Self {
        let mut names = Self::default();
        for parameter in parameters {
            names.bind(parameter, 0, false);
        }
        names.statements(body, false);
        names
    }

    fn bind(&mut self, name: &str, offset: u32, in_loop: bool) {
        self.bindings
            .entry(name.to_owned())
            .or_default()
            .push(Binding {
                offset,
                in_loop,
                gives_value: true,
            });
    }

    /// Notes an annotation alone, which makes `name` the function's variable, as CPython's
    /// symbol table reads it, and gives it no value.
    fn declare(&mut self, name: &str, offset: u32) {
        self.bindings
            .entry(name.to_owned())
            .or_default()
            .push(Binding {
                offset,
                in_loop: false,
                gives_value: false,
            });
    }

    /// Returns the names this function binds, less those it declares `global` or `nonlocal`,
    /// with the variables of its comprehensions that a `lambda` captures.
    fn bound_names(&self) -> BTreeSet<String> {
        self.bindings
            .keys()
            .filter(|name| !self.globals.contains(*name) && !self.nonlocals.contains(*name))
            .chain(&self.comprehension_cells)
            .cloned()
            .collect()
    }

    /// Returns the names this function, or a function nested in it, assigns through
    /// `nonlocal` among those of the functions around it.
    fn assigned_free(&self) -> BTreeSet<String> {
        let own = self.bound_names();
        self.nonlocals
            .iter()
            .chain(self.nested.iter().flat_map(|nested| &nested.assigned))
            .filter(|name| !own.contains(*name) && !self.globals.contains(*name))
            .cloned()
            .collect()
    }

    /// Returns the names read, here or in a nested function, that this function neither
    /// binds nor declares `global`.
    fn free(&self) -> BTreeSet<String> {
        self.unbound(&self.reads)
    }

    /// Returns the free names read here other than by calling them by name, or read in a
    /// nested function.
    fn value_free(&self) -> BTreeSet<String> {
        self.unbound(&self.values)
    }

    /// Returns the names of `read`, those this function declares `nonlocal` and those the
    /// nested functions read, that this function neither binds nor declares `global`.
    fn unbound(&self, read: &BTreeSet<String>) -> BTreeSet<String> {
        let own = self.bound_names();
        read.iter()
            .chain(&self.nonlocals)
            .chain(self.nested.iter().flat_map(|nested| &nested.free))
            .filter(|name| !own.contains(*name) && !self.globals.contains(*name))
            .cloned()
            .collect()
    }

    /// Returns true when only one binding binds `name`, which no function declares `global`
    /// or assigns through `nonlocal`.
    fn bound_once(&self, name: &str) -> bool {
        !self.globals.contains(name)
            && !self
                .nested
                .iter()
                .any(|nested| nested.assigned.contains(name))
            && self.bindings.get(name).is_some_and(|all| all.len() == 1)
    }

    /// Returns the groups of nested functions of `body` that call one another by name.
    fn groups(&self, body: &[Stmt]) -> Vec<Vec<&NestedDef>> {
        let mut lists = Vec::new();
        statement_lists(body, &mut lists);
        lists
            .into_iter()
            .flat_map(|list| self.groups_in(list))
            .collect()
    }

    /// Returns the groups among the `def`s of one statement list: the functions, each bound
    /// only by its `def`, that reach one another through the names they read.
    fn groups_in(&self, statements: &[Stmt]) -> Vec<Vec<&NestedDef>> {
        let candidates: Vec<(usize, &NestedDef)> = statements
            .iter()
            .enumerate()
            .filter_map(|(index, statement)| {
                let Stmt::FunctionDef { span, .. } = statement else {
                    return None;
                };
                let nested = self.nested.iter().find(|nested| nested.span == *span)?;
                self.bound_once(&nested.name).then_some((index, nested))
            })
            .collect();
        let mut groups: Vec<Vec<(usize, &NestedDef)>> = Vec::new();
        for (index, nested) in &candidates {
            if groups
                .iter()
                .flatten()
                .any(|(_, member)| member.span == nested.span)
            {
                continue;
            }
            let reached = reach(nested, &candidates);
            let mut group = vec![(*index, *nested)];
            group.extend(candidates.iter().filter(|(_, other)| {
                other.span != nested.span
                    && reached.contains(&other.name)
                    && reach(other, &candidates).contains(&nested.name)
            }));
            if group.len() > 1 {
                groups.push(group);
            }
        }
        groups
            .into_iter()
            .filter(|group| self.shares_environment(statements, group))
            .map(|group| group.into_iter().map(|(_, nested)| nested).collect())
            .collect()
    }

    /// Returns true when the functions of `group`, at their positions in `statements`, may share
    /// their captures: each names the others only to call them, only `def`s run between the
    /// first `def` of the group and the last, and no other nested function defined before the
    /// last reads one of them, so none of them runs before each `def` of the group has run.
    fn shares_environment(&self, statements: &[Stmt], group: &[(usize, &NestedDef)]) -> bool {
        let members: BTreeSet<&str> = group
            .iter()
            .map(|(_, nested)| nested.name.as_str())
            .collect();
        let named =
            |names: &BTreeSet<String>| names.iter().any(|name| members.contains(name.as_str()));
        let first = group
            .iter()
            .map(|(index, _)| *index)
            .min()
            .unwrap_or_default();
        let last = group
            .iter()
            .map(|(index, _)| *index)
            .max()
            .unwrap_or_default();
        let end = statements
            .get(last)
            .map_or(0, |statement| statement.span().start().0);
        !group.iter().any(|(_, nested)| named(&nested.values))
            && statements[first..=last]
                .iter()
                .all(|statement| matches!(statement, Stmt::FunctionDef { .. }))
            && !self.nested.iter().any(|nested| {
                !members.contains(nested.name.as_str())
                    && nested.span.start().0 < end
                    && named(&nested.free)
            })
    }

    /// Returns true when one binding, outside every loop and before `offset`, the position of
    /// a nested function that captures `name`, gives its value.
    fn copied(&self, name: &str, offset: u32) -> bool {
        let mut valued = self
            .bindings
            .get(name)
            .into_iter()
            .flatten()
            .filter(|binding| binding.gives_value);
        matches!(
            (valued.next(), valued.next()),
            (Some(binding), None) if !binding.in_loop && binding.offset < offset
        )
    }

    fn statements(&mut self, statements: &[Stmt], in_loop: bool) {
        for statement in statements {
            self.statement(statement, in_loop);
        }
    }

    #[allow(clippy::too_many_lines, reason = "one arm per statement kind")]
    fn statement(&mut self, statement: &Stmt, in_loop: bool) {
        match statement {
            Stmt::FunctionDef {
                name,
                parameters,
                returns,
                body,
                decorators,
                span,
                ..
            } => {
                // Annotations, defaults and decorators are read where the `def` runs.
                for parameter in parameters {
                    for expression in parameter.annotation.iter().chain(&parameter.default) {
                        self.expression(expression, in_loop);
                    }
                }
                for expression in returns.iter().chain(decorators) {
                    self.expression(expression, in_loop);
                }
                let names: Vec<&str> = parameters
                    .iter()
                    .map(|parameter| parameter.name.name.as_str())
                    .collect();
                let inner = Self::of(&names, body);
                self.nested.push(NestedDef {
                    span: *span,
                    name: name.name.clone(),
                    free: inner.free(),
                    values: inner.value_free(),
                    assigned: inner.assigned_free(),
                });
                self.bind(&name.name, span.start().0, in_loop);
            }
            Stmt::ClassDef {
                name, bases, span, ..
            } => {
                for base in bases {
                    self.expression(base, in_loop);
                }
                self.bind(&name.name, span.start().0, in_loop);
            }
            Stmt::Return { value, .. } => {
                if let Some(value) = value {
                    self.expression(value, in_loop);
                }
            }
            Stmt::Assign { targets, value, .. } => {
                self.expression(value, in_loop);
                for target in targets {
                    self.assigned_from(target, value, in_loop);
                }
            }
            Stmt::AugAssign { target, value, .. } => {
                self.expression(value, in_loop);
                // `x += 1` reads `x`, then binds it again.
                self.expression(target, in_loop);
                self.assigned_from(target, value, in_loop);
            }
            Stmt::AnnAssign { target, value, .. } => {
                if let Some(value) = value {
                    self.expression(value, in_loop);
                    self.assigned_from(target, value, in_loop);
                } else if let Expr::Name { id, span } = target {
                    self.declare(id, span.start().0);
                }
            }
            Stmt::For {
                target,
                iter,
                body,
                orelse,
                ..
            } => {
                self.expression(iter, in_loop);
                self.target(target, true);
                self.statements(body, true);
                self.statements(orelse, in_loop);
            }
            Stmt::While {
                test, body, orelse, ..
            } => {
                self.expression(test, true);
                self.statements(body, true);
                self.statements(orelse, in_loop);
            }
            Stmt::If {
                test, body, orelse, ..
            } => {
                self.expression(test, in_loop);
                self.statements(body, in_loop);
                self.statements(orelse, in_loop);
            }
            Stmt::Match { subject, cases, .. } => {
                self.expression(subject, in_loop);
                for case in cases {
                    for name in case.pattern.bound_names() {
                        self.bind(&name.name, name.span.start().0, in_loop);
                    }
                    if let Some(guard) = &case.guard {
                        self.expression(guard, in_loop);
                    }
                    self.statements(&case.body, in_loop);
                }
            }
            Stmt::Import { names, span, .. } | Stmt::ImportFrom { names, span, .. } => {
                for alias in names {
                    let bound = alias.as_name.as_ref().unwrap_or(&alias.name);
                    let first = bound.name.split('.').next().unwrap_or_default();
                    self.bind(first, span.start().0, in_loop);
                }
            }
            Stmt::Expr { value, .. } => self.expression(value, in_loop),
            Stmt::Try {
                body,
                handlers,
                orelse,
                finalbody,
                ..
            } => {
                self.statements(body, in_loop);
                for handler in handlers {
                    self.handler(handler, in_loop);
                }
                self.statements(orelse, in_loop);
                self.statements(finalbody, in_loop);
            }
            Stmt::With { items, body, .. } => {
                for item in items {
                    self.expression(&item.context, in_loop);
                    if let Some(target) = &item.target {
                        self.target(target, in_loop);
                    }
                }
                self.statements(body, in_loop);
            }
            Stmt::Delete { targets, .. } => {
                // `del x` unbinds `x`, which a copy could not show.
                for target in targets {
                    self.expression(target, in_loop);
                    self.target(target, in_loop);
                }
            }
            Stmt::Assert { test, message, .. } => {
                self.expression(test, in_loop);
                if let Some(message) = message {
                    self.expression(message, in_loop);
                }
            }
            Stmt::Global { names, .. } => {
                self.globals
                    .extend(names.iter().map(|name| name.name.clone()));
            }
            Stmt::Nonlocal { names, .. } => {
                self.nonlocals
                    .extend(names.iter().map(|name| name.name.clone()));
            }
            Stmt::Raise {
                exception, cause, ..
            } => {
                for expression in exception.iter().chain(cause) {
                    self.expression(expression, in_loop);
                }
            }
            Stmt::TypeAlias { name, span, .. } => {
                self.bind(&name.name, span.start().0, in_loop);
            }
            Stmt::Pass { .. } | Stmt::Break { .. } | Stmt::Continue { .. } | Stmt::Other { .. } => {
            }
        }
    }

    fn handler(&mut self, handler: &ExceptHandler, in_loop: bool) {
        if let Some(class) = &handler.class {
            self.expression(class, in_loop);
        }
        // `except E as name` binds the name, then deletes it at the end of the clause.
        if let Some(name) = &handler.name {
            self.bind(&name.name, name.span.start().0, in_loop);
            self.bind(&name.name, name.span.start().0, in_loop);
        }
        self.statements(&handler.body, in_loop);
    }

    /// Notes the names an assignment target binds, and reads the parts of an item or an
    /// attribute it assigns.
    fn target(&mut self, target: &Expr, in_loop: bool) {
        self.target_after(target, in_loop, None);
    }

    /// Notes the names a target binds once `value` is evaluated, so a `lambda` in the value
    /// comes before the binding and reads the variable it assigns through a cell.
    fn assigned_from(&mut self, target: &Expr, value: &Expr, in_loop: bool) {
        self.target_after(target, in_loop, Some(value.span().end().0));
    }

    fn target_after(&mut self, target: &Expr, in_loop: bool, after: Option<u32>) {
        match target {
            Expr::Name { id, span } => self.bind(id, after.unwrap_or(span.start().0), in_loop),
            Expr::Tuple { elements, .. } | Expr::List { elements, .. } => {
                for element in elements {
                    self.target_after(element, in_loop, after);
                }
            }
            Expr::Subscript { value, index, .. } => {
                self.expression(value, in_loop);
                self.expression(index, in_loop);
            }
            Expr::Attribute { value, .. } => self.expression(value, in_loop),
            other => self.expression(other, in_loop),
        }
    }

    fn expressions<'expression>(
        &mut self,
        expressions: impl IntoIterator<Item = &'expression Expr>,
        in_loop: bool,
    ) {
        for expression in expressions {
            self.expression(expression, in_loop);
        }
    }

    fn expression(&mut self, expression: &Expr, in_loop: bool) {
        match expression {
            Expr::Name { id, .. } => {
                self.reads.insert(id.clone());
                self.values.insert(id.clone());
            }
            Expr::BoolOp { values, .. } => self.expressions(values, in_loop),
            Expr::BinOp { left, right, .. } => self.expressions([&**left, &**right], in_loop),
            Expr::UnaryOp { operand, .. } => self.expression(operand, in_loop),
            Expr::IfExp {
                test, body, orelse, ..
            } => self.expressions([&**test, &**body, &**orelse], in_loop),
            Expr::Named { target, value, .. } => {
                self.expression(value, in_loop);
                self.bind(&target.name, value.span().end().0, in_loop);
            }
            Expr::Lambda {
                parameters,
                body,
                span,
            } => self.lambda(parameters, body, *span, in_loop),
            Expr::Compare {
                left, comparators, ..
            } => {
                self.expression(left, in_loop);
                self.expressions(comparators, in_loop);
            }
            Expr::Call {
                function,
                arguments,
                keywords,
                ..
            } => {
                // A call by name reads the name only to call it.
                if let Expr::Name { id, .. } = &**function {
                    self.reads.insert(id.clone());
                } else {
                    self.expression(function, in_loop);
                }
                self.expressions(arguments, in_loop);
                self.expressions(keywords.iter().map(|keyword| &keyword.value), in_loop);
            }
            Expr::Subscript { value, index, .. } => {
                self.expressions([&**value, &**index], in_loop);
            }
            Expr::FString { parts, .. } => {
                for part in parts {
                    self.fstring_part(part, in_loop);
                }
            }
            Expr::Tuple { elements, .. }
            | Expr::List { elements, .. }
            | Expr::Set { elements, .. } => {
                self.expressions(elements, in_loop);
            }
            Expr::Dict { keys, values, .. } => {
                self.expressions(keys.iter().flatten(), in_loop);
                self.expressions(values, in_loop);
            }
            Expr::ListComp {
                element,
                generators,
                ..
            }
            | Expr::SetComp {
                element,
                generators,
                ..
            } => self.comprehension(&[&**element], generators, in_loop),
            Expr::DictComp {
                key,
                value,
                generators,
                ..
            } => self.comprehension(&[&**key, &**value], generators, in_loop),
            Expr::Attribute { value, .. } => self.expression(value, in_loop),
            Expr::Slice {
                lower, upper, step, ..
            } => self.expressions(
                [lower, upper, step]
                    .into_iter()
                    .flatten()
                    .map(|part| &**part),
                in_loop,
            ),
            Expr::Constant { .. } | Expr::Quoted { .. } | Expr::Other { .. } => {}
        }
    }

    /// Notes a `lambda` as a nested function of its parameters and its body; its defaults are
    /// read where it runs.
    fn lambda(&mut self, parameters: &[Parameter], body: &Expr, span: Span, in_loop: bool) {
        for default in parameters
            .iter()
            .filter_map(|parameter| parameter.default.as_ref())
        {
            self.expression(default, in_loop);
        }
        let mut inner = Self::default();
        for parameter in parameters {
            inner.bind(&parameter.name.name, 0, false);
        }
        inner.expression(body, false);
        self.nested.push(NestedDef {
            span,
            name: "<lambda>".to_owned(),
            free: inner.free(),
            values: inner.value_free(),
            assigned: inner.assigned_free(),
        });
    }

    fn fstring_part(&mut self, part: &FStringPart, in_loop: bool) {
        if let FStringPart::Field(field) = part {
            self.expression(&field.value, in_loop);
            for part in field.spec.iter().flatten() {
                self.fstring_part(part, in_loop);
            }
        }
    }

    /// Reads a comprehension: its targets are its own, so only the names it reads that no
    /// clause binds are the function's; an assignment expression in it binds the function's
    /// variable, as in CPython.
    fn comprehension(&mut self, results: &[&Expr], generators: &[Comprehension], in_loop: bool) {
        let mut inner = Self::default();
        for generator in generators {
            inner.expression(&generator.iter, true);
            inner.target(&generator.target, true);
            inner.expressions(&generator.ifs, true);
        }
        inner.expressions(results.iter().copied(), true);
        let own: BTreeSet<String> = generators
            .iter()
            .flat_map(|generator| target_names(&generator.target))
            .collect();
        for (name, bindings) in inner.bindings {
            if own.contains(&name) {
                continue;
            }
            for binding in bindings {
                self.bind(&name, binding.offset, in_loop || binding.in_loop);
            }
        }
        self.reads
            .extend(inner.reads.into_iter().filter(|name| !own.contains(name)));
        self.values
            .extend(inner.values.into_iter().filter(|name| !own.contains(name)));
        // A `lambda` that reads the comprehension's variable shares it through a cell.
        self.comprehension_cells.extend(
            inner
                .nested
                .iter()
                .flat_map(|nested| nested.free.intersection(&own))
                .cloned(),
        );
        self.comprehension_cells.extend(inner.comprehension_cells);
        self.nested.extend(inner.nested);
    }
}

/// Returns the names that `nested` reaches among `candidates`: those it reads, and those
/// they reach in turn.
fn reach(nested: &NestedDef, candidates: &[(usize, &NestedDef)]) -> BTreeSet<String> {
    let mut reached = BTreeSet::new();
    let mut next = vec![nested];
    while let Some(current) = next.pop() {
        for (_, other) in candidates {
            if current.free.contains(&other.name) && reached.insert(other.name.clone()) {
                next.push(other);
            }
        }
    }
    reached
}

/// Adds `body` and each statement list in it, outside nested functions and classes, to
/// `lists`.
fn statement_lists<'tree>(body: &'tree [Stmt], lists: &mut Vec<&'tree [Stmt]>) {
    lists.push(body);
    for statement in body {
        match statement {
            Stmt::For { body, orelse, .. }
            | Stmt::While { body, orelse, .. }
            | Stmt::If { body, orelse, .. } => {
                statement_lists(body, lists);
                statement_lists(orelse, lists);
            }
            Stmt::Try {
                body,
                handlers,
                orelse,
                finalbody,
                ..
            } => {
                statement_lists(body, lists);
                for handler in handlers {
                    statement_lists(&handler.body, lists);
                }
                statement_lists(orelse, lists);
                statement_lists(finalbody, lists);
            }
            Stmt::With { body, .. } => statement_lists(body, lists),
            Stmt::Match { cases, .. } => {
                for case in cases {
                    statement_lists(&case.body, lists);
                }
            }
            _ => {}
        }
    }
}

/// Returns the names an assignment target binds.
fn target_names(target: &Expr) -> Vec<String> {
    match target {
        Expr::Name { id, .. } => vec![id.clone()],
        Expr::Tuple { elements, .. } | Expr::List { elements, .. } => {
            elements.iter().flat_map(target_names).collect()
        }
        _ => Vec::new(),
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use typy_diagnostics::FileId;
    use typy_source::syntax::Stmt;
    use typy_source::{PythonFloor, SourceFile, parse};

    use super::ClosureScope;

    /// Returns, for the one function `source` defines, the captures of each of its nested
    /// functions in source order, and its cell variables.
    fn scope(source: &str) -> (Vec<Vec<String>>, Vec<String>) {
        let file = SourceFile::from_text(FileId(0), "app.py".into(), source);
        let tree = parse(&file, PythonFloor::V3_12).expect("the source parses");
        let Some(Stmt::FunctionDef {
            parameters, body, ..
        }) = tree.body.first()
        else {
            panic!("the source defines a function first");
        };
        let names: Vec<&str> = parameters
            .iter()
            .map(|parameter| parameter.name.name.as_str())
            .collect();
        let found = ClosureScope::of(&names, body, &BTreeSet::new());
        let captures = found
            .captures
            .values()
            .map(|names| names.iter().cloned().collect())
            .collect();
        (captures, found.cells.into_iter().collect())
    }

    /// Returns, for the one function `source` defines, the group of each of its nested
    /// functions in source order, empty for one in none.
    fn groups(source: &str) -> Vec<Vec<String>> {
        let file = SourceFile::from_text(FileId(0), "app.py".into(), source);
        let tree = parse(&file, PythonFloor::V3_12).expect("the source parses");
        let Some(Stmt::FunctionDef { body, .. }) = tree.body.first() else {
            panic!("the source defines a function first");
        };
        let found = ClosureScope::of(&[], body, &BTreeSet::new());
        found
            .captures
            .keys()
            .map(|span| {
                found
                    .groups
                    .get(span)
                    .map(|names| names.iter().cloned().collect())
                    .unwrap_or_default()
            })
            .collect()
    }

    const EVEN_ODD: &str = "def f(n: int) -> bool:\n    limit = n\n    \
                            def even(k: int) -> bool:\n        return k == 0 or odd(k - 1)\n    \
                            def odd(k: int) -> bool:\n        return k < limit and even(k - 1)\n    \
                            return even(n)\n";

    #[test]
    fn fun_04_functions_that_call_one_another_by_name_share_their_captures() {
        let both = vec!["even".to_owned(), "odd".to_owned()];
        assert_eq!(groups(EVEN_ODD), [both.clone(), both]);
        // Each holds what either captures, and neither holds the other.
        let (captures, cells) = scope(EVEN_ODD);
        assert_eq!(
            captures,
            [vec!["limit".to_owned()], vec!["limit".to_owned()]]
        );
        assert_eq!(cells, Vec::<String>::new());
    }

    #[test]
    fn fun_04_a_cycle_of_three_with_another_def_between_is_one_group() {
        let found = groups(
            "def f() -> None:\n    def a() -> int:\n        return b()\n    \
             def other() -> int:\n        return 1\n    def b() -> int:\n        return c()\n    \
             def c() -> int:\n        return a()\n",
        );
        let all = vec!["a".to_owned(), "b".to_owned(), "c".to_owned()];
        assert_eq!(found, [all.clone(), Vec::new(), all.clone(), all]);
    }

    #[test]
    fn fun_04_functions_that_may_run_before_the_group_is_made_form_no_group() {
        for source in [
            // A function of the group read as a value.
            "def f() -> None:\n    def a() -> int:\n        return b()\n    \
             def b() -> int:\n        print(a)\n        return a()\n",
            // A function of the group read by a function nested in another.
            "def f() -> None:\n    def a() -> int:\n        return b()\n    \
             def b() -> int:\n        def inner() -> int:\n            return a()\n        \
             return inner()\n",
            // A statement between the `def`s, which may leave the block.
            "def f() -> None:\n    def a() -> int:\n        return b()\n    print(1)\n    \
             def b() -> int:\n        return a()\n",
            // Another function, defined before the group is complete, that reads one of it.
            "def f() -> None:\n    def early() -> int:\n        return a()\n    \
             def a() -> int:\n        return b()\n    def b() -> int:\n        return a()\n",
            // A function bound again.
            "def f() -> None:\n    def a() -> int:\n        return b()\n    \
             def b() -> int:\n        return a()\n    b = a\n",
            // A function declared global in another.
            "def f() -> None:\n    def a() -> int:\n        global b\n        return b()\n    \
             def b() -> int:\n        return a()\n",
        ] {
            assert!(groups(source).iter().all(Vec::is_empty), "{source}");
        }
    }

    #[test]
    fn fun_04_a_variable_assigned_after_the_group_lives_in_a_cell_both_hold() {
        let (captures, cells) = scope(
            "def f() -> None:\n    def a() -> int:\n        return b()\n    \
             def b() -> int:\n        return a() + late\n    late = 1\n",
        );
        assert_eq!(captures, [vec!["late".to_owned()], vec!["late".to_owned()]]);
        assert_eq!(cells, ["late"]);
    }

    #[test]
    fn fun_04_a_variable_bound_once_before_the_def_is_copied() {
        let (captures, cells) = scope(
            "def outer(n: int) -> None:\n    total = n + 1\n    def inner() -> int:\n        \
             return total + n + len([n])\n",
        );
        assert_eq!(captures, [vec!["n".to_owned(), "total".to_owned()]]);
        assert_eq!(cells, Vec::<String>::new());
    }

    #[test]
    fn fun_04_a_variable_a_loop_binds_or_bound_again_or_after_the_def_lives_in_a_cell() {
        for (source, cell) in [
            (
                "def f() -> None:\n    for i in range(3):\n        def g() -> int:\n            \
                 return i\n",
                "i",
            ),
            (
                "def f() -> None:\n    x = 1\n    def g() -> int:\n        return x\n    x = 2\n",
                "x",
            ),
            (
                "def f() -> None:\n    def g() -> int:\n        return x\n    x = 1\n",
                "x",
            ),
            (
                "def f(x: int) -> None:\n    x += 1\n    def g() -> int:\n        return x\n",
                "x",
            ),
            (
                "def f() -> None:\n    while True:\n        x = 1\n        break\n    \
                 def g() -> int:\n        return x\n",
                "x",
            ),
            (
                "def f() -> None:\n    try:\n        pass\n    except ValueError as e:\n        \
                 def g() -> str:\n            return str(e)\n",
                "e",
            ),
            (
                "def f() -> None:\n    y = [z := 1 for _ in range(2)]\n    \
                 def g() -> int:\n        return z\n",
                "z",
            ),
        ] {
            let (_, cells) = scope(source);
            assert_eq!(cells, [cell.to_owned()], "{source}");
        }
    }

    #[test]
    fn fun_04_only_the_functions_own_variables_are_captured() {
        // A global, a name the inner function binds, a comprehension's own target and a
        // module name are not captures; a name a deeper function reads passes through.
        let (captures, cells) = scope(
            "def f() -> None:\n    global g\n    g = 1\n    a = 1\n    b = 2\n    \
             def inner() -> None:\n        b = 3\n        print(g, b, [a for a in range(2)])\n        \
             def deeper() -> int:\n            return a\n",
        );
        assert_eq!(captures, [vec!["a".to_owned()]]);
        assert_eq!(cells, Vec::<String>::new());
    }

    /// Returns, for the one function `source` defines, its cell variables and the variables
    /// its nested functions assign through `nonlocal`.
    fn rebound(source: &str) -> (Vec<String>, Vec<String>) {
        let file = SourceFile::from_text(FileId(0), "app.py".into(), source);
        let tree = parse(&file, PythonFloor::V3_12).expect("the source parses");
        let Some(Stmt::FunctionDef { body, .. }) = tree.body.first() else {
            panic!("the source defines a function first");
        };
        let found = ClosureScope::of(&[], body, &BTreeSet::new());
        (
            found.cells.into_iter().collect(),
            found.rebound.into_iter().collect(),
        )
    }

    #[test]
    fn smt_09_a_variable_a_nested_function_assigns_through_nonlocal_lives_in_a_cell() {
        for source in [
            // Bound once before the `def`, which would copy it, and assigned in the function.
            "def f() -> None:\n    count = 0\n    def bump() -> None:\n        nonlocal count\n        \
             count += 1\n",
            // Assigned by a function nested two deep, through one that never names it.
            "def f() -> None:\n    count = 0\n    def middle() -> None:\n        \
             def inner() -> None:\n            nonlocal count\n            count = 2\n        \
             inner()\n",
            // Declared by an annotation alone, which binds it as CPython's symbol table does.
            "def f() -> None:\n    count: int\n    def bump() -> None:\n        nonlocal count\n        \
             count = 1\n",
        ] {
            let (cells, rebound) = rebound(source);
            assert_eq!(cells, ["count"], "{source}");
            assert_eq!(rebound, ["count"], "{source}");
        }
    }

    #[test]
    fn smt_09_a_variable_named_nonlocal_passes_through_the_function_in_between() {
        let file = SourceFile::from_text(
            FileId(0),
            "app.py".into(),
            "def middle() -> None:\n    def inner() -> None:\n        nonlocal far\n        far = 1\n",
        );
        let tree = parse(&file, PythonFloor::V3_12).expect("the source parses");
        let Some(Stmt::FunctionDef { body, .. }) = tree.body.first() else {
            panic!("the source defines a function");
        };
        let enclosing: BTreeSet<String> = ["far".to_owned()].into();
        let found = ClosureScope::of(&[], body, &enclosing);
        let captures: Vec<Vec<String>> = found
            .captures
            .values()
            .map(|names| names.iter().cloned().collect())
            .collect();
        assert_eq!(captures, [vec!["far".to_owned()]]);
        // `far` is the enclosing function's, so this function neither keeps nor rebinds it.
        assert!(found.cells.is_empty() && found.rebound.is_empty());
    }

    #[test]
    fn smt_09_an_annotation_alone_gives_no_value_so_one_assignment_is_still_copied() {
        let (captures, cells) = scope(
            "def f() -> None:\n    total: int\n    total = 1\n    def g() -> int:\n        \
             return total\n",
        );
        assert_eq!(captures, [vec!["total".to_owned()]]);
        assert_eq!(cells, Vec::<String>::new());
    }

    #[test]
    fn smt_09_a_function_another_assigns_through_nonlocal_is_in_no_group() {
        let found = groups(
            "def f() -> None:\n    def a() -> int:\n        return b()\n    \
             def b() -> int:\n        nonlocal a\n        return a()\n",
        );
        assert!(found.iter().all(Vec::is_empty), "{found:?}");
    }

    #[test]
    fn exp_14_a_lambda_captures_as_a_nested_function_does() {
        // A variable bound once before the `lambda` is copied; a loop variable is shared.
        let (captures, cells) = scope(
            "def f(step: int) -> None:\n    add = lambda x: x + step\n    for i in range(3):\n        \
             show = lambda: i\n",
        );
        assert_eq!(captures, [vec!["step".to_owned()], vec!["i".to_owned()]]);
        assert_eq!(cells, ["i"]);
    }

    #[test]
    fn exp_14_a_lambda_in_the_value_of_its_own_assignment_shares_the_variable() {
        let (captures, cells) =
            scope("def f() -> None:\n    fact = lambda n: 1 if n == 0 else n * fact(n - 1)\n");
        assert_eq!(captures, [vec!["fact".to_owned()]]);
        assert_eq!(cells, ["fact"]);
    }

    #[test]
    fn exp_14_a_comprehension_variable_a_lambda_reads_lives_in_a_cell() {
        let (captures, cells) =
            scope("def f() -> None:\n    made = [lambda: i * k for i in range(3)]\n    k = 2\n");
        assert_eq!(captures, [vec!["i".to_owned(), "k".to_owned()]]);
        assert_eq!(cells, ["i", "k"]);
    }

    #[test]
    fn exp_14_names_a_lambda_binds_are_its_own() {
        let (captures, cells) =
            scope("def f(x: int) -> None:\n    g = lambda y: (z := y + x) + z\n");
        assert_eq!(captures, [vec!["x".to_owned()]]);
        assert_eq!(cells, Vec::<String>::new());
    }

    #[test]
    fn smt_09_nonlocal_under_a_function_that_declares_the_name_global_rebinds_nothing_here() {
        let (cells, rebound) = rebound(
            "def f() -> None:\n    x = 1\n    def g() -> None:\n        global x\n        \
             def h() -> None:\n            nonlocal x\n            x = 2\n",
        );
        assert_eq!(
            (cells, rebound),
            (Vec::<String>::new(), Vec::<String>::new())
        );
    }

    #[test]
    fn fun_04_a_function_declared_global_or_read_by_a_later_def_keeps_its_group_rules() {
        // A name declared `global` is the module's, so its `def`s form no group.
        let global = groups(
            "def f() -> None:\n    global a\n    def a() -> int:\n        return b()\n    \
             def b() -> int:\n        return a()\n",
        );
        assert!(global.iter().all(Vec::is_empty), "{global:?}");
        // A `def` after the group that reads one of it runs once the group is complete.
        let later = groups(
            "def f() -> None:\n    def a() -> int:\n        return b()\n    \
             def b() -> int:\n        return a()\n    def last() -> int:\n        return a()\n",
        );
        let both = vec!["a".to_owned(), "b".to_owned()];
        assert_eq!(later, [both.clone(), both, Vec::new()]);
    }

    #[test]
    fn fun_04_an_annotated_attribute_binds_no_variable() {
        let (captures, cells) = scope(
            "def f(box: Box) -> None:\n    box.size: int\n    def g() -> Box:\n        return box\n",
        );
        assert_eq!(captures, [vec!["box".to_owned()]]);
        assert_eq!(cells, Vec::<String>::new());
    }

    #[test]
    fn fun_04_an_assignment_expression_in_a_comprehension_in_a_loop_binds_in_the_loop() {
        let (_, cells) = scope(
            "def f() -> None:\n    for _ in range(2):\n        xs = [y := 1 for _ in range(2)]\n    \
             def g() -> int:\n        return y\n",
        );
        assert_eq!(cells, ["y"]);
    }

    #[test]
    fn fun_04_a_name_a_nested_function_declares_global_is_no_capture() {
        let (captures, cells) = scope(
            "def f() -> None:\n    g = 1\n    def inner() -> int:\n        global g\n        \
             return g\n    g = 2\n",
        );
        assert_eq!(captures, [Vec::<String>::new()]);
        assert_eq!(cells, Vec::<String>::new());
    }

    #[test]
    fn fun_04_the_enclosing_functions_variables_pass_through_to_a_nested_function() {
        let file = SourceFile::from_text(
            FileId(0),
            "app.py".into(),
            "def middle() -> None:\n    def inner() -> int:\n        return far + near\n    near = 1\n",
        );
        let tree = parse(&file, PythonFloor::V3_12).expect("the source parses");
        let Some(Stmt::FunctionDef { body, .. }) = tree.body.first() else {
            panic!("the source defines a function");
        };
        let enclosing: BTreeSet<String> = ["far".to_owned()].into();
        let found = ClosureScope::of(&[], body, &enclosing);
        let captures: Vec<Vec<String>> = found
            .captures
            .values()
            .map(|names| names.iter().cloned().collect())
            .collect();
        assert_eq!(captures, [vec!["far".to_owned(), "near".to_owned()]]);
        // `far` is the enclosing function's, so only `near` is this function's cell.
        assert_eq!(found.cells.into_iter().collect::<Vec<_>>(), ["near"]);
    }
}
