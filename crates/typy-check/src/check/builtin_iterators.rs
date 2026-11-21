//! `map` and `filter` (LIB-23, ADR-0089): a module that calls them, and binds neither name
//! itself, gets a generic iterator class for each form it calls, written as Python source the
//! checker reads as the program's own: `map` with each number of iterables it is given,
//! `filter` with a function and `filter(None, ...)`. Each class's name holds a space, so no
//! program names it, and so do the names its annotations read `Callable` and `Iterator` by,
//! which imports of the same kind bind. The checker makes an instance of a copy of one for
//! each call (`body/map_filter.rs`), and lowering writes it as `map` or `filter`.

use std::collections::BTreeSet;
use std::fmt::Write as _;

use camino::Utf8PathBuf;
use typy_diagnostics::{FileId, Span};
use typy_source::syntax::{Constant, Expr, Identifier, Stmt};
use typy_source::{PythonFloor, SourceFile, parse};

use super::visit::{visit_expression, visit_statement};

/// What a class name of a `map` or `filter` class adds to `map` or `filter`; the name with
/// the space is no Python name, and lowering writes the class by the part before it.
pub(crate) const BUILT_IN: &str = " (built-in";

/// The name the classes read `typing`'s `Callable` by.
const CALLABLE: &str = "Callable (built-in)";

/// The name the classes read `Iterator` by.
const ITERATOR: &str = "Iterator (built-in)";

/// The forms of `map` and `filter` a module calls.
#[derive(Default)]
struct Calls {
    /// The numbers of iterables `map` is called with.
    map: BTreeSet<usize>,
    /// Whether `filter` is called with a function, and with `None`.
    filter: (bool, bool),
    /// Where the first call is, which the written classes are moved to.
    at: Option<Span>,
}

/// Returns `map` or `filter` for the name of a class written for them, and `None` for the
/// name of any other class.
#[must_use]
pub fn built_in_class(name: &str) -> Option<&str> {
    name.split_once(BUILT_IN).map(|(name, _)| name)
}

/// Returns the name of the class of `map` with `count` iterables.
#[must_use]
pub(crate) fn map_class(count: usize) -> String {
    format!("map{BUILT_IN} {count})")
}

/// Returns the name of the class of `filter`, with a function or with `None`.
#[must_use]
pub(crate) fn filter_class(with_function: bool) -> String {
    if with_function {
        format!("filter{BUILT_IN})")
    } else {
        format!("filter{BUILT_IN} None)")
    }
}

/// Adds the classes of the calls of `map` and `filter` the module makes to its body.
pub(super) fn expand(body: &mut Vec<Stmt>) {
    let bound = bound_names(body);
    let mut calls = Calls::default();
    scan(body, &mut calls);
    if bound.contains("map") {
        calls.map.clear();
    }
    if bound.contains("filter") {
        calls.filter = (false, false);
    }
    // The classes call the built-in `next`, which a module that binds the name hides.
    let Some(at) = calls.at.filter(|_| !bound.contains("next")) else {
        return;
    };
    let mut source = String::new();
    for count in &calls.map {
        write_map(&mut source, *count);
    }
    if calls.filter.0 {
        write_filter(&mut source, true);
    }
    if calls.filter.1 {
        write_filter(&mut source, false);
    }
    if source.is_empty() {
        return;
    }
    let source =
        format!("from collections.abc import Callable as CALLABLE, Iterator as ITERATOR\n{source}");
    let file = SourceFile::from_text(FileId(u32::MAX), Utf8PathBuf::from("<builtins>"), &source);
    // The source is written above and always parses.
    let mut written = parse(&file, PythonFloor::default())
        .map(|tree| tree.body)
        .unwrap_or_default();
    for statement in &mut written {
        statement.relocate(at);
        rename(statement, at);
    }
    body.extend(written);
}

/// Returns the names the module's top level binds, which hide the built-ins of those names.
fn bound_names(body: &[Stmt]) -> BTreeSet<String> {
    let mut names = BTreeSet::new();
    for statement in body {
        match statement {
            Stmt::FunctionDef { name, .. } | Stmt::ClassDef { name, .. } => {
                names.insert(name.name.clone());
            }
            Stmt::Assign { targets, .. } => {
                for target in targets {
                    target_names(target, &mut names);
                }
            }
            Stmt::AnnAssign { target, .. } | Stmt::AugAssign { target, .. } => {
                target_names(target, &mut names);
            }
            Stmt::Import { names: aliases, .. } | Stmt::ImportFrom { names: aliases, .. } => {
                for alias in aliases {
                    let bound = alias.as_name.as_ref().unwrap_or(&alias.name);
                    let top = bound.name.split('.').next().unwrap_or_default();
                    names.insert(top.to_owned());
                }
            }
            _ => {}
        }
    }
    names
}

/// Adds the names an assignment's target binds.
fn target_names(target: &Expr, names: &mut BTreeSet<String>) {
    match target {
        Expr::Name { id, .. } => {
            names.insert(id.clone());
        }
        Expr::Tuple { elements, .. } | Expr::List { elements, .. } => {
            for element in elements {
                target_names(element, names);
            }
        }
        Expr::Starred { value, .. } => target_names(value, names),
        _ => {}
    }
}

/// Notes the calls of `map` and `filter` in `statements` and every body inside them.
fn scan(statements: &[Stmt], calls: &mut Calls) {
    for statement in statements {
        visit_statement(statement, &mut |expression| note(expression, calls));
        match statement {
            Stmt::FunctionDef { body, .. }
            | Stmt::ClassDef { body, .. }
            | Stmt::With { body, .. } => scan(body, calls),
            Stmt::For { body, orelse, .. }
            | Stmt::While { body, orelse, .. }
            | Stmt::If { body, orelse, .. } => {
                scan(body, calls);
                scan(orelse, calls);
            }
            Stmt::Try { .. } => {
                for block in statement.try_blocks() {
                    scan(block, calls);
                }
            }
            Stmt::Match { cases, .. } => {
                for case in cases {
                    scan(&case.body, calls);
                }
            }
            _ => {}
        }
    }
}

/// Notes `expression` when it calls `map` or `filter`, and the calls inside a `lambda`'s body,
/// which the visit does not enter.
fn note(expression: &Expr, calls: &mut Calls) {
    match expression {
        Expr::Lambda { body, .. } => visit_expression(body, &mut |inner| note(inner, calls)),
        Expr::Call {
            function,
            arguments,
            span,
            ..
        } => {
            let Expr::Name { id, .. } = function.as_ref() else {
                return;
            };
            let unpacked = arguments
                .iter()
                .any(|argument| matches!(argument, Expr::Starred { .. }));
            match (id.as_str(), arguments.as_slice()) {
                ("map", [_, rest @ ..]) if !rest.is_empty() && !unpacked => {
                    calls.map.insert(rest.len());
                }
                (
                    "filter",
                    [
                        Expr::Constant {
                            value: Constant::None,
                            ..
                        },
                        _,
                    ],
                ) => calls.filter.1 = true,
                ("filter", [_, _]) => calls.filter.0 = true,
                _ => return,
            }
            calls.at.get_or_insert(*span);
        }
        _ => {}
    }
}

/// Writes the class of `map` with `count` iterables, its annotations given by `rename`. With
/// `strict=True` (CPython 3.14), the first iterator to end is checked against the others, and a
/// difference raises CPython's `ValueError`.
fn write_map(source: &mut String, count: usize) {
    let numbers = 1..=count;
    let items: Vec<String> = numbers.clone().map(|item| format!("T{item}")).collect();
    let _ = writeln!(source, "class map{count}[{}, R]:", items.join(", "));
    let _ = write!(source, "    def __init__(self, function: int");
    for item in numbers.clone() {
        let _ = write!(source, ", iterator{item}: int");
    }
    let _ = writeln!(
        source,
        ", strict: bool) -> None:\n        self.hidden_function = function\n        \
         self.hidden_strict = strict"
    );
    for item in numbers.clone() {
        let _ = writeln!(
            source,
            "        self.hidden_iterator{item} = iterator{item}"
        );
    }
    let steps: Vec<String> = numbers
        .clone()
        .map(|item| format!("next(self.hidden_iterator{item})"))
        .collect();
    let _ = writeln!(
        source,
        "    def __iter__(self) -> int:\n        return self\n    def __next__(self) -> R:\n        \
         if not self.hidden_strict:\n            return self.hidden_function({})\n        try:\n            \
         item1 = next(self.hidden_iterator1)\n        except StopIteration:\n            \
         self.hidden_ended()\n            raise",
        steps.join(", ")
    );
    for item in 2..=count {
        let _ = writeln!(
            source,
            "        try:\n            item{item} = next(self.hidden_iterator{item})\n        \
             except StopIteration:\n            raise ValueError(\"map() argument {item} is \
             shorter than {}\") from None",
            before(item)
        );
    }
    let taken: Vec<String> = numbers.map(|item| format!("item{item}")).collect();
    let _ = writeln!(
        source,
        "        return self.hidden_function({})\n    def hidden_ended(self) -> None:\n        \
         pass",
        taken.join(", ")
    );
    for item in 2..=count {
        let _ = writeln!(
            source,
            "        try:\n            next(self.hidden_iterator{item})\n        except \
             StopIteration:\n            pass\n        else:\n            raise \
             ValueError(\"map() argument {item} is longer than {}\")",
            before(item)
        );
    }
}

/// Returns the arguments before argument `item` as CPython's `map` names them: `argument 1`,
/// or `arguments 1-2`.
fn before(item: usize) -> String {
    if item == 2 {
        "argument 1".to_owned()
    } else {
        format!("arguments 1-{}", item - 1)
    }
}

/// Writes the class of `filter`, with a function or with `None`.
fn write_filter(source: &mut String, with_function: bool) {
    let (head, test) = if with_function {
        (
            "class filter[T1, R]:\n    def __init__(self, function: int, iterator1: int) -> \
             None:\n        self.hidden_function = function",
            "self.hidden_function(item)",
        )
    } else {
        (
            "class filterNone[T1]:\n    def __init__(self, iterator1: int) -> None:",
            "item",
        )
    };
    let _ = writeln!(source, "{head}\n        self.hidden_iterator1 = iterator1");
    let _ = writeln!(
        source,
        "    def __iter__(self) -> int:\n        return self\n    def __next__(self) -> T1:\n        \
         while True:\n            item = next(self.hidden_iterator1)\n            if {test}:\n                \
         return item"
    );
}

/// Gives a written statement its hidden names: the class's own, and the annotations of
/// `__init__` and `__iter__`, built from the type parameters.
fn rename(statement: &mut Stmt, at: Span) {
    match statement {
        Stmt::ImportFrom { names, .. } => {
            for alias in names {
                let hidden = if alias.name.name == "Callable" {
                    CALLABLE
                } else {
                    ITERATOR
                };
                alias.as_name = Some(Identifier {
                    name: hidden.to_owned(),
                    span: at,
                });
            }
        }
        Stmt::ClassDef {
            name,
            body,
            type_parameters,
            ..
        } => {
            let hidden = match name.name.strip_prefix("map") {
                Some(count) => map_class(count.parse().unwrap_or(1)),
                None => filter_class(name.name == "filter"),
            };
            let parameters: Vec<Expr> = type_parameters
                .iter()
                .map(|parameter| named(&parameter.name.name, at))
                .collect();
            let own = Expr::Quoted {
                annotation: Box::new(subscript(&hidden, parameters.clone(), at)),
                span: at,
            };
            // The type parameters are the item types, `T1` on, then the function's result `R`.
            let (items, result) = match type_parameters.last() {
                Some(last) if last.name.name == "R" => {
                    (&parameters[..parameters.len() - 1], parameters.last())
                }
                _ => (parameters.as_slice(), None),
            };
            name.name = hidden;
            for method in body.iter_mut() {
                annotate(method, (items, result), &own, at);
            }
            hide_statements(body);
        }
        _ => {}
    }
}

/// Gives a method of a written class its annotations: `__init__` the function's
/// `Callable[[T1, ...], R]` and each iterator's `Iterator[T]`, and `__iter__` the class.
fn annotate(method: &mut Stmt, (items, result): (&[Expr], Option<&Expr>), own: &Expr, at: Span) {
    let Stmt::FunctionDef {
        name,
        parameters: taken,
        returns,
        ..
    } = method
    else {
        return;
    };
    if name.name == "__iter__" {
        *returns = Some(own.clone());
        return;
    }
    if name.name != "__init__" {
        return;
    }
    let mut iterators = items.iter();
    for parameter in taken.iter_mut().skip(1) {
        if parameter.name.name == "function" {
            let arguments = Expr::List {
                elements: items.to_vec(),
                span: at,
            };
            let result = result.cloned().unwrap_or_else(|| named("None", at));
            parameter.annotation = Some(subscript(CALLABLE, vec![arguments, result], at));
        } else if parameter.name.name.starts_with("iterator") {
            let item = iterators
                .next()
                .cloned()
                .unwrap_or_else(|| named("None", at));
            parameter.annotation = Some(subscript(ITERATOR, vec![item], at));
        }
    }
}

/// Returns `name`, read as a name.
fn named(name: &str, at: Span) -> Expr {
    Expr::Name {
        id: name.to_owned(),
        span: at,
    }
}

/// Returns `name[A, B]`.
fn subscript(name: &str, mut arguments: Vec<Expr>, at: Span) -> Expr {
    let index = if arguments.len() == 1 {
        arguments.remove(0)
    } else {
        Expr::Tuple {
            elements: arguments,
            span: at,
        }
    };
    Expr::Subscript {
        value: Box::new(named(name, at)),
        index: Box::new(index),
        span: at,
    }
}

/// What the written source starts the names of the classes' own attributes and methods with,
/// which `hide_statements` gives a space, so no program reads them, where CPython's `map` and
/// `filter` objects have none.
const HIDDEN: &str = "hidden_";

/// Gives each attribute and method name of `statements` that starts with `HIDDEN` its hidden
/// form, `function (built-in)`.
fn hide_statements(statements: &mut [Stmt]) {
    for statement in statements {
        match statement {
            Stmt::FunctionDef { name, body, .. } => {
                hide(name);
                hide_statements(body);
            }
            Stmt::Assign { targets, value, .. } => {
                targets.iter_mut().for_each(hide_expression);
                hide_expression(value);
            }
            Stmt::Return {
                value: Some(value), ..
            }
            | Stmt::Expr { value, .. } => hide_expression(value),
            Stmt::If {
                test, body, orelse, ..
            }
            | Stmt::While {
                test, body, orelse, ..
            } => {
                hide_expression(test);
                hide_statements(body);
                hide_statements(orelse);
            }
            Stmt::Try {
                body,
                handlers,
                orelse,
                ..
            } => {
                hide_statements(body);
                for handler in handlers {
                    hide_statements(&mut handler.body);
                }
                hide_statements(orelse);
            }
            _ => {}
        }
    }
}

/// Gives each hidden name of `expression` its hidden form.
fn hide_expression(expression: &mut Expr) {
    match expression {
        Expr::Attribute {
            value, attribute, ..
        } => {
            hide(attribute);
            hide_expression(value);
        }
        Expr::Call {
            function,
            arguments,
            ..
        } => {
            hide_expression(function);
            arguments.iter_mut().for_each(hide_expression);
        }
        Expr::UnaryOp { operand, .. } => hide_expression(operand),
        _ => {}
    }
}

/// Gives `name` its hidden form when it starts with `HIDDEN`.
fn hide(name: &mut Identifier) {
    if let Some(own) = name.name.strip_prefix(HIDDEN) {
        name.name = format!("{own}{BUILT_IN})");
    }
}
