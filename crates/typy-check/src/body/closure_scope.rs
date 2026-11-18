//! Finds, from a function's source alone, the variables each nested function captures and
//! which of them live in a cell (FUN-04, ADR-0061), as CPython's symbol table finds free and
//! cell variables. A variable is copied when the nested function is made when one binding
//! gives its value, outside every loop and before each nested function that captures it;
//! any other captured variable lives in a cell, which every function that captures it shares.

use std::collections::{BTreeMap, BTreeSet};

use typy_diagnostics::Span;
use typy_source::syntax::{Comprehension, ExceptHandler, Expr, FStringPart, Stmt};

/// What a function's nested functions capture of its own variables, and of those it captures
/// itself.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct ClosureScope {
    /// For each nested `def` of the function, by the span of its statement, the variables it
    /// reads from the function, directly or through the functions nested in it, by name.
    pub(crate) captures: BTreeMap<Span, BTreeSet<String>>,
    /// The function's own variables that live in a cell.
    pub(crate) cells: BTreeSet<String>,
}

impl ClosureScope {
    /// Returns the scope of the function of `parameters` and `body`, nested in functions whose
    /// variables `enclosing` names.
    pub(crate) fn of(parameters: &[&str], body: &[Stmt], enclosing: &BTreeSet<String>) -> Self {
        let mut names = Names::default();
        for parameter in parameters {
            names.bind(parameter, 0, false);
        }
        names.statements(body, false);
        let own = names.bound_names();
        let available: BTreeSet<String> = own
            .iter()
            .cloned()
            .chain(names.free().intersection(enclosing).cloned())
            .collect();
        let mut scope = Self::default();
        for (span, name, free) in &names.nested {
            // A nested function's own name reads the running function object, since only its
            // `def` binds it, so it is no capture.
            let captured: BTreeSet<String> = free
                .intersection(&available)
                .filter(|each| *each != name)
                .cloned()
                .collect();
            for name in captured.intersection(&own) {
                if !names.copied(name, span.start().0) {
                    scope.cells.insert(name.clone());
                }
            }
            scope.captures.insert(*span, captured);
        }
        scope
    }
}

/// Returns the names the function of `parameters` and `body` reads, directly or through its
/// nested functions, that it does not bind: its free variables, and the module's names.
fn free_names(parameters: &[&str], body: &[Stmt]) -> BTreeSet<String> {
    let mut names = Names::default();
    for parameter in parameters {
        names.bind(parameter, 0, false);
    }
    names.statements(body, false);
    names.free()
}

/// One binding of a variable: where it is, and whether a loop runs it again.
#[derive(Clone, Copy, Debug)]
struct Binding {
    offset: u32,
    in_loop: bool,
}

/// The names one function binds and reads, and the free variables of its nested functions.
#[derive(Default)]
struct Names {
    bindings: BTreeMap<String, Vec<Binding>>,
    globals: BTreeSet<String>,
    reads: BTreeSet<String>,
    /// Each nested `def`, by its span, with its name and its free variables.
    nested: Vec<(Span, String, BTreeSet<String>)>,
}

impl Names {
    fn bind(&mut self, name: &str, offset: u32, in_loop: bool) {
        self.bindings
            .entry(name.to_owned())
            .or_default()
            .push(Binding { offset, in_loop });
    }

    fn bound_names(&self) -> BTreeSet<String> {
        self.bindings
            .keys()
            .filter(|name| !self.globals.contains(*name))
            .cloned()
            .collect()
    }

    /// Returns the names read, here or in a nested function, that this function does not
    /// bind.
    fn free(&self) -> BTreeSet<String> {
        let own = self.bound_names();
        self.reads
            .iter()
            .chain(self.nested.iter().flat_map(|(_, _, free)| free))
            .filter(|name| !own.contains(*name))
            .cloned()
            .collect()
    }

    /// Returns true when one binding, outside every loop and before `offset`, the position of
    /// a nested function that captures `name`, gives its value.
    fn copied(&self, name: &str, offset: u32) -> bool {
        matches!(
            self.bindings.get(name).map(Vec::as_slice),
            Some([binding]) if !binding.in_loop && binding.offset < offset
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
                self.nested
                    .push((*span, name.name.clone(), free_names(&names, body)));
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
                    self.target(target, in_loop);
                }
            }
            Stmt::AugAssign { target, value, .. } => {
                self.expression(value, in_loop);
                // `x += 1` reads `x`, then binds it again.
                self.expression(target, in_loop);
                self.target(target, in_loop);
            }
            Stmt::AnnAssign { target, value, .. } => {
                if let Some(value) = value {
                    self.expression(value, in_loop);
                    self.target(target, in_loop);
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
        match target {
            Expr::Name { id, span } => self.bind(id, span.start().0, in_loop),
            Expr::Tuple { elements, .. } | Expr::List { elements, .. } => {
                for element in elements {
                    self.target(element, in_loop);
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
            }
            Expr::BoolOp { values, .. } => self.expressions(values, in_loop),
            Expr::BinOp { left, right, .. } => self.expressions([&**left, &**right], in_loop),
            Expr::UnaryOp { operand, .. } => self.expression(operand, in_loop),
            Expr::IfExp {
                test, body, orelse, ..
            } => self.expressions([&**test, &**body, &**orelse], in_loop),
            Expr::Named { target, value, .. } => {
                self.expression(value, in_loop);
                self.bind(&target.name, target.span.start().0, in_loop);
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
                self.expression(function, in_loop);
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
        self.nested.extend(inner.nested);
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
