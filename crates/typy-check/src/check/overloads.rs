//! Gathers the `@overload` signatures of functions and methods (TYP-29, ADR-0112). Each
//! `def` decorated with `typing.overload` before the `def` of the same name without it, its
//! implementation, leaves the module or the class body and is kept by name, so the checker
//! reads its signature and no program runs it.

use std::collections::{BTreeMap, BTreeSet};

use typy_diagnostics::{Diagnostic, DiagnosticCode, Diagnostics, RuleId};
use typy_source::syntax::{Expr, Stmt};

use super::definition::{Definition, FunctionParts};
use crate::body::Program;
use crate::typed::FunctionId;

/// The signatures of the overloads of each function or method that has them, by its ID.
pub(crate) type Overloaded = Vec<(FunctionId, Vec<crate::typed::FunctionSignature>)>;

/// The overloads of one module: for each function, and each method by its class's name,
/// the `def` statements of its overloads in source order.
#[derive(Clone, Debug, Default)]
pub(crate) struct Overloads {
    functions: BTreeMap<String, Vec<Stmt>>,
    methods: BTreeMap<(String, String), Vec<Stmt>>,
}

impl Overloads {
    /// Returns the overloads of the module function `name`.
    pub(crate) fn of_function(&self, name: &str) -> &[Stmt] {
        self.functions.get(name).map_or(&[], Vec::as_slice)
    }

    /// Returns the overloads of the method `name` of the class `class`.
    pub(crate) fn of_method(&self, class: &str, name: &str) -> &[Stmt] {
        self.methods
            .get(&(class.to_owned(), name.to_owned()))
            .map_or(&[], Vec::as_slice)
    }
}

/// Takes the overloads out of a module's body and of its classes' bodies, and reports an
/// overload without an implementation, and one in a form a later release gives.
pub(crate) fn gather(body: &mut Vec<Stmt>, diagnostics: &mut Diagnostics) -> Overloads {
    let names = OverloadNames::of(body);
    let mut overloads = Overloads::default();
    if names.is_empty() {
        return overloads;
    }
    for (name, stubs) in gather_block(body, &names, None, diagnostics) {
        overloads.functions.insert(name, stubs);
    }
    for statement in body.iter_mut() {
        match statement {
            Stmt::ClassDef {
                name, body: inner, ..
            } => {
                let class = name.name.clone();
                for (method, stubs) in gather_block(inner, &names, Some(&class), diagnostics) {
                    overloads.methods.insert((class.clone(), method), stubs);
                }
                for member in inner.iter() {
                    if let Stmt::FunctionDef { body: method, .. } = member {
                        refuse_nested(method, &names, diagnostics);
                    }
                }
            }
            Stmt::FunctionDef { body: inner, .. } => refuse_nested(inner, &names, diagnostics),
            other => {
                for block in super::nested_blocks(other) {
                    refuse_nested(block, &names, diagnostics);
                }
            }
        }
    }
    overloads
}

/// The names a module reads `typing.overload` through: the names `from typing import
/// overload` binds, and the names of the module `typing` itself.
struct OverloadNames {
    direct: BTreeSet<String>,
    modules: BTreeSet<String>,
}

impl OverloadNames {
    /// Returns the names the imports of `body` bind to `typing.overload` and to `typing`.
    fn of(body: &[Stmt]) -> Self {
        let mut found = Self {
            direct: BTreeSet::new(),
            modules: BTreeSet::new(),
        };
        for statement in body {
            match statement {
                Stmt::ImportFrom {
                    module: Some(module),
                    names,
                    level: 0,
                    ..
                } if module.name == "typing" => {
                    for alias in names {
                        if alias.name.name == "overload" {
                            let bound = alias.as_name.as_ref().unwrap_or(&alias.name);
                            found.direct.insert(bound.name.clone());
                        }
                    }
                }
                Stmt::Import { names, .. } => {
                    for alias in names {
                        if alias.name.name == "typing" {
                            let bound = alias.as_name.as_ref().unwrap_or(&alias.name);
                            found.modules.insert(bound.name.clone());
                        }
                    }
                }
                _ => {}
            }
        }
        found
    }

    fn is_empty(&self) -> bool {
        self.direct.is_empty() && self.modules.is_empty()
    }

    /// Returns whether `decorator` names `typing.overload`.
    fn names(&self, decorator: &Expr) -> bool {
        match decorator {
            Expr::Name { id, .. } => self.direct.contains(id),
            Expr::Attribute {
                value, attribute, ..
            } => {
                attribute.name == "overload"
                    && matches!(value.as_ref(), Expr::Name { id, .. } if self.modules.contains(id))
            }
            _ => false,
        }
    }

    /// Returns whether a statement is a `def` decorated with `typing.overload`.
    fn decorates(&self, statement: &Stmt) -> bool {
        matches!(statement, Stmt::FunctionDef { decorators, .. }
            if decorators.iter().any(|decorator| self.names(decorator)))
    }
}

/// Takes the overloads out of one block, a module's or a class's body, and returns them by
/// the name of their implementation, which must follow them.
fn gather_block(
    body: &mut Vec<Stmt>,
    names: &OverloadNames,
    class: Option<&str>,
    diagnostics: &mut Diagnostics,
) -> Vec<(String, Vec<Stmt>)> {
    let mut gathered = Vec::new();
    let mut waiting: Vec<Stmt> = Vec::new();
    for statement in std::mem::take(body) {
        if names.decorates(&statement) {
            let stub = stub(statement, names, class, diagnostics);
            if waiting
                .first()
                .is_some_and(|first| name_of(first) != name_of(&stub))
            {
                diagnostics.push(no_implementation(&waiting[0]));
                waiting.clear();
            }
            waiting.push(stub);
            continue;
        }
        if let Some(first) = waiting.first() {
            if matches!(&statement, Stmt::FunctionDef { name, .. } if name.name == name_of(first)) {
                check_implementation(&statement, class, diagnostics);
                gathered.push((name_of(first).to_owned(), std::mem::take(&mut waiting)));
            } else {
                diagnostics.push(no_implementation(first));
                waiting.clear();
            }
        }
        body.push(statement);
    }
    if let Some(first) = waiting.first() {
        diagnostics.push(no_implementation(first));
    }
    gathered
}

/// Returns an overload without its decorator, after reporting a form a later release gives:
/// another decorator, type parameters, or a special method.
fn stub(
    mut statement: Stmt,
    names: &OverloadNames,
    class: Option<&str>,
    diagnostics: &mut Diagnostics,
) -> Stmt {
    if let Stmt::FunctionDef {
        name,
        decorators,
        type_parameters,
        ..
    } = &mut statement
    {
        let others = decorators
            .iter()
            .filter(|decorator| !names.names(decorator))
            .count();
        if others > 0 || !type_parameters.is_empty() {
            diagnostics.push(planned_form(
                name.span,
                "with another decorator or type parameters",
            ));
        } else if class.is_some() && is_special(&name.name) {
            diagnostics.push(planned_form(name.span, "on a special method"));
        }
        decorators.clear();
    }
    statement
}

/// Reports an implementation in a form a later release gives: decorated, generic, or a
/// special method.
fn check_implementation(statement: &Stmt, class: Option<&str>, diagnostics: &mut Diagnostics) {
    let Some(parts) = FunctionParts::of(statement) else {
        return;
    };
    if !parts.decorators.is_empty() || !parts.type_parameters.is_empty() {
        diagnostics.push(planned_form(
            parts.name.span,
            "with a decorated or generic implementation",
        ));
    } else if class.is_some() && is_special(&parts.name.name) {
        diagnostics.push(planned_form(parts.name.span, "on a special method"));
    }
}

/// Reports each overload in a block of a function, a class method's body or a block such as
/// an `if`, which a later release gives.
fn refuse_nested(body: &[Stmt], names: &OverloadNames, diagnostics: &mut Diagnostics) {
    for statement in body {
        if names.decorates(statement)
            && let Stmt::FunctionDef { name, .. } = statement
        {
            diagnostics.push(planned_form(name.span, "inside a function or a block"));
        }
        if let Stmt::FunctionDef { body: inner, .. } | Stmt::ClassDef { body: inner, .. } =
            statement
        {
            refuse_nested(inner, names, diagnostics);
        }
        for block in super::nested_blocks(statement) {
            refuse_nested(block, names, diagnostics);
        }
    }
}

fn name_of(statement: &Stmt) -> &str {
    match statement {
        Stmt::FunctionDef { name, .. } => &name.name,
        _ => "",
    }
}

fn is_special(name: &str) -> bool {
    name.len() > 4 && name.starts_with("__") && name.ends_with("__")
}

/// Returns E0432 for overloads that no implementation follows.
fn no_implementation(first: &Stmt) -> Diagnostic {
    let (name, span) = match first {
        Stmt::FunctionDef { name, .. } => (name.name.as_str(), name.span),
        other => ("", other.span()),
    };
    Diagnostic::new(
        DiagnosticCode::error(432),
        span,
        format!("the overloads of `{name}` have no implementation"),
    )
    .with_rule(RuleId::TYP_29)
    .with_help(format!(
        "after the `@overload` definitions, write `def {name}(...)` without `@overload`, \
         whose body runs for every call"
    ))
}

/// Returns E0201 for an overload in a form planned for 0.13 (TYP-35).
fn planned_form(span: typy_diagnostics::Span, form: &str) -> Diagnostic {
    Diagnostic::new(
        DiagnosticCode::error(201),
        span,
        format!("`@overload` {form} is planned for TyPy 0.13 (TYP-35)"),
    )
    .with_rule(RuleId::TYP_35)
}

/// Reports each overload its implementation does not fit, one whose result no test tells
/// apart from the implementation's others, and an override of a method with overloads,
/// which a later release gives (TYP-29, TYP-35).
pub(super) fn check_fits(
    program: &Program,
    definitions: &[Definition<'_>],
    first_function: usize,
    diagnostics: &mut Diagnostics,
) {
    for (index, definition) in definitions.iter().enumerate() {
        let id = FunctionId(u32::try_from(first_function + index).unwrap_or(u32::MAX));
        for (position, overload) in program.overloads.get(&id).into_iter().flatten().enumerate() {
            let reason = match super::overload_fit::fit(overload, &definition.signature) {
                Err(reason) => reason,
                Ok(_)
                    if !crate::body::can_test(
                        &definition.signature.returns.value_type(),
                        &overload.returns.value_type(),
                    ) =>
                {
                    format!(
                        "no test tells its result, {}, from the implementation's others when \
                         the program runs",
                        overload.returns
                    )
                }
                Ok(_) => continue,
            };
            diagnostics.push(super::overload_fit::unfit(
                &definition.name,
                position,
                &reason,
                definition.span,
            ));
        }
        if let Some(class) = definition.class
            && program
                .overloaded_base_method(class, &definition.name)
                .is_some()
        {
            diagnostics.push(planned_form(
                definition.span,
                "on a method a derived class overrides",
            ));
        }
    }
}
