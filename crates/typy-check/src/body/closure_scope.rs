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

/// The parameter of a generator expression's function: the iterator of its first iterable.
pub(super) const FIRST_ITERATOR: &str = ".0";

/// The name of a generator expression's function, as CPython names it.
pub(super) const GENERATOR_EXPRESSION: &str = "<genexpr>";

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
            Expr::Starred { value, .. } => self.target_after(value, in_loop, after),
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
            Expr::Yield { value, .. } => {
                if let Some(value) = value {
                    self.expression(value, in_loop);
                }
            }
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
            Expr::GeneratorExp {
                element,
                generators,
                span,
            } => self.generator_expression(element, generators, *span, in_loop),
            Expr::Attribute { value, .. }
            | Expr::YieldFrom { value, .. }
            | Expr::Starred { value, .. } => {
                self.expression(value, in_loop);
            }
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

    /// Notes a generator expression as a nested function of `.0`, the iterator of its first
    /// iterable, which it reads where it runs, as CPython does (EXP-15).
    fn generator_expression(
        &mut self,
        element: &Expr,
        generators: &[Comprehension],
        span: Span,
        in_loop: bool,
    ) {
        if let Some(first) = generators.first() {
            self.expression(&first.iter, in_loop);
        }
        let mut inner = Self::default();
        inner.bind(FIRST_ITERATOR, 0, false);
        for (position, generator) in generators.iter().enumerate() {
            if position > 0 {
                inner.expression(&generator.iter, true);
            }
            inner.target(&generator.target, true);
            inner.expressions(&generator.ifs, true);
        }
        inner.expression(element, true);
        self.nested.push(NestedDef {
            span,
            name: GENERATOR_EXPRESSION.to_owned(),
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
        Expr::Starred { value, .. } => target_names(value),
        _ => Vec::new(),
    }
}

#[cfg(test)]
mod tests;
