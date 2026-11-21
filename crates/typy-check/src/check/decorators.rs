//! Rewrites each decorated function of a module, at its top level or nested in a function,
//! into the function under a hidden name and an assignment of what its decorators return to
//! its own name (FUN-06, ADR-0082): `@trace def inc(...)` becomes `def inc (undecorated)(...)`
//! then `inc = trace(inc (undecorated))`. The decorators are evaluated top to bottom and
//! applied bottom up, as CPython evaluates them, and a read of the name, also inside the
//! function's body, reads what they returned. A method keeps its decorators, which the class
//! reads (CLS-06), as does a class.

use typy_source::syntax::{Expr, Identifier, Stmt};

/// What a decorated function's hidden name adds to its own; a name with a space is no Python
/// name, so it never meets one the program writes, and the compiled program shows the own
/// name in its place.
pub(crate) const UNDECORATED: &str = " (undecorated)";

/// Rewrites the decorated functions of `body` and of the functions in it.
pub(super) fn expand(body: &mut Vec<Stmt>) {
    let statements = std::mem::take(body);
    for mut statement in statements {
        match &mut statement {
            Stmt::FunctionDef { body: inner, .. } => expand(inner),
            Stmt::ClassDef { body: inner, .. } => {
                for member in inner.iter_mut() {
                    if let Stmt::FunctionDef { body: method, .. } = member {
                        expand(method);
                    }
                }
            }
            _ => {}
        }
        match statement {
            Stmt::FunctionDef {
                name,
                parameters,
                returns,
                body: inner,
                decorators,
                type_parameters,
                span,
            } if !decorators.is_empty() => {
                let hidden = Identifier {
                    name: format!("{}{UNDECORATED}", name.name),
                    span: name.span,
                };
                let applied = decorators.into_iter().rev().fold(
                    Expr::Name {
                        id: hidden.name.clone(),
                        span: name.span,
                    },
                    |argument, decorator| {
                        let decorator_span = decorator.span();
                        Expr::Call {
                            function: Box::new(decorator),
                            arguments: vec![argument],
                            keywords: Vec::new(),
                            span: decorator_span,
                        }
                    },
                );
                body.push(Stmt::FunctionDef {
                    name: hidden,
                    parameters,
                    returns,
                    body: inner,
                    decorators: Vec::new(),
                    type_parameters,
                    span,
                });
                body.push(Stmt::Assign {
                    targets: vec![Expr::Name {
                        id: name.name,
                        span: name.span,
                    }],
                    value: applied,
                    span,
                });
            }
            other => body.push(other),
        }
    }
}

/// Returns `name` as Python shows it: a decorated function's own name in place of its hidden
/// one, also inside a qualified name such as `inc (undecorated).<locals>.step`.
#[must_use]
pub fn python_name(name: &str) -> String {
    name.replace(UNDECORATED, "")
}

#[cfg(test)]
mod tests {
    use super::{UNDECORATED, python_name};

    #[test]
    fn a_hidden_name_shows_as_the_functions_own() {
        assert_eq!(python_name(&format!("inc{UNDECORATED}")), "inc");
        assert_eq!(
            python_name(&format!("outer{UNDECORATED}.<locals>.step")),
            "outer.<locals>.step"
        );
        assert_eq!(python_name("plain"), "plain");
    }
}
